'use strict';

/*
 * Focused IL2CPP native-method capture. It records code addresses and a bounded
 * byte prefix only for explicitly requested methods; it never reads game saves
 * or managed object instances.
 */

const TARGET_ASSEMBLY = globalThis.IL2CPP_NATIVE_TARGET_ASSEMBLY || 'Assembly-CSharp.dll';
const TARGET_METHODS = Array.isArray(globalThis.IL2CPP_NATIVE_TARGET_METHODS)
  ? globalThis.IL2CPP_NATIVE_TARGET_METHODS
  : [];
const TARGET_CLASSES = new Set(
  Array.isArray(globalThis.IL2CPP_NATIVE_TARGET_CLASSES)
    ? globalThis.IL2CPP_NATIVE_TARGET_CLASSES
    : []
);
const TARGET_MODULE_OFFSETS = new Set(
  Array.isArray(globalThis.IL2CPP_NATIVE_TARGET_MODULE_OFFSETS)
    ? globalThis.IL2CPP_NATIVE_TARGET_MODULE_OFFSETS.map(Number)
    : []
);
const CODE_BYTE_LIMIT = Number.isInteger(globalThis.IL2CPP_NATIVE_CODE_BYTE_LIMIT)
  ? globalThis.IL2CPP_NATIVE_CODE_BYTE_LIMIT
  : 192;
const EXACT_BOUNDARIES = globalThis.IL2CPP_NATIVE_EXACT_BOUNDARIES === true;
const INCLUDE_METHOD_INDEX = globalThis.IL2CPP_NATIVE_INCLUDE_METHOD_INDEX === true;
const MAX_ASSEMBLIES = 4096;
const MAX_CLASSES = 200000;
const MAX_READY_ATTEMPTS = 80;
const POLL_MS = 125;

let started = false;
let readyAttempts = 0;

function emit(kind, payload) {
  send({ kind: kind, payload: payload });
}

function bind(module, name, returnType, argumentTypes, required) {
  let address = null;
  try {
    address = module.findExportByName(name);
  } catch (_) {
    address = null;
  }
  if (address === null && required !== false) {
    throw new Error('Missing required IL2CPP export: ' + name);
  }
  return address === null ? null : new NativeFunction(address, returnType, argumentTypes);
}

function readCString(address) {
  return address === null || address.isNull() ? null : address.readUtf8String();
}

function pointerSizedValue(address) {
  return Process.pointerSize === 8 ? address.readU64().toNumber() : address.readU32();
}

function typeName(api, type) {
  if (type.isNull()) {
    return null;
  }
  const allocated = api.typeGetName(type);
  if (allocated.isNull()) {
    return null;
  }
  const value = allocated.readUtf8String();
  if (api.free !== null) {
    api.free(allocated);
  }
  return value;
}

function bytesToHex(buffer) {
  const bytes = new Uint8Array(buffer);
  let value = '';
  for (let index = 0; index < bytes.length; index += 1) {
    value += bytes[index].toString(16).padStart(2, '0');
  }
  return value;
}

function createApi(module) {
  return {
    domainGet: bind(module, 'il2cpp_domain_get', 'pointer', []),
    threadAttach: bind(module, 'il2cpp_thread_attach', 'pointer', ['pointer']),
    domainGetAssemblies: bind(module, 'il2cpp_domain_get_assemblies', 'pointer', ['pointer', 'pointer']),
    assemblyGetImage: bind(module, 'il2cpp_assembly_get_image', 'pointer', ['pointer']),
    imageGetName: bind(module, 'il2cpp_image_get_name', 'pointer', ['pointer']),
    imageGetClassCount: bind(module, 'il2cpp_image_get_class_count', 'uint64', ['pointer']),
    imageGetClass: bind(module, 'il2cpp_image_get_class', 'pointer', ['pointer', 'uint64']),
    classGetName: bind(module, 'il2cpp_class_get_name', 'pointer', ['pointer']),
    classGetNamespace: bind(module, 'il2cpp_class_get_namespace', 'pointer', ['pointer']),
    classGetMethods: bind(module, 'il2cpp_class_get_methods', 'pointer', ['pointer', 'pointer']),
    methodGetName: bind(module, 'il2cpp_method_get_name', 'pointer', ['pointer']),
    methodGetReturnType: bind(module, 'il2cpp_method_get_return_type', 'pointer', ['pointer']),
    methodGetParamCount: bind(module, 'il2cpp_method_get_param_count', 'uint32', ['pointer']),
    methodGetParam: bind(module, 'il2cpp_method_get_param', 'pointer', ['pointer', 'uint32']),
    methodGetToken: bind(module, 'il2cpp_method_get_token', 'uint32', ['pointer'], false),
    typeGetName: bind(module, 'il2cpp_type_get_name', 'pointer', ['pointer']),
    free: bind(module, 'il2cpp_free', 'void', ['pointer'], false),
  };
}

function findAssemblyImage(api, domain) {
  const countAddress = Memory.alloc(Process.pointerSize);
  countAddress.writePointer(NULL);
  const assemblies = api.domainGetAssemblies(domain, countAddress);
  const count = pointerSizedValue(countAddress);
  if (assemblies.isNull() || count > MAX_ASSEMBLIES) {
    throw new Error('Invalid assembly table: count=' + count);
  }
  const assemblyWithoutExtension = TARGET_ASSEMBLY.replace(/\.dll$/, '');
  for (let index = 0; index < count; index += 1) {
    const assembly = assemblies.add(index * Process.pointerSize).readPointer();
    const image = api.assemblyGetImage(assembly);
    const name = readCString(api.imageGetName(image));
    if (name === TARGET_ASSEMBLY || name === assemblyWithoutExtension) {
      return image;
    }
  }
  throw new Error(TARGET_ASSEMBLY + ' was not loaded');
}

function requestedMethod(className, methodName, parameterCount) {
  return TARGET_METHODS.find(function (target) {
    return target.className === className
      && target.methodName === methodName
      && (target.parameterCount === undefined || target.parameterCount === parameterCount);
  });
}

function captureMethod(api, module, namespaceName, className, method) {
  const methodName = readCString(api.methodGetName(method));
  const parameterCount = api.methodGetParamCount(method);
  const requested = requestedMethod(className, methodName, parameterCount);
  let pointerOffset = null;
  try {
    const pointer = method.readPointer();
    const owner = pointer.isNull() ? null : Process.findModuleByAddress(pointer);
    if (owner !== null && owner.name === module.name) {
      pointerOffset = pointer.sub(module.base).toUInt32();
    }
  } catch (_) {
    pointerOffset = null;
  }
  if (!TARGET_CLASSES.has(className)
      && requested === undefined
      && (pointerOffset === null || !TARGET_MODULE_OFFSETS.has(pointerOffset))) {
    return null;
  }

  const parameters = [];
  for (let index = 0; index < parameterCount; index += 1) {
    parameters.push(typeName(api, api.methodGetParam(method, index)));
  }

  // In this Unity IL2CPP build MethodInfo starts with methodPointer followed by
  // virtualMethodPointer. Both are recorded so the assumption stays auditable.
  const candidates = [];
  for (let slot = 0; slot < 2; slot += 1) {
    const fieldAddress = method.add(slot * Process.pointerSize);
    try {
      const address = fieldAddress.readPointer();
      const owner = address.isNull() ? null : Process.findModuleByAddress(address);
      const belongsToIl2Cpp = owner !== null && owner.name === module.name;
      let codeHex = null;
      if (belongsToIl2Cpp && CODE_BYTE_LIMIT > 0) {
        codeHex = bytesToHex(address.readByteArray(CODE_BYTE_LIMIT));
      }
      candidates.push({
        slot: slot,
        assumedField: slot === 0 ? 'methodPointer' : 'virtualMethodPointer',
        address: address.toString(),
        module: owner === null ? null : owner.name,
        moduleOffset: owner === null ? null : address.sub(owner.base).toString(),
        codeHex: codeHex,
      });
    } catch (error) {
      candidates.push({
        slot: slot,
        assumedField: slot === 0 ? 'methodPointer' : 'virtualMethodPointer',
        error: String(error),
      });
    }
  }

  return {
    namespace: namespaceName,
    className: className,
    methodName: methodName,
    parameterCount: parameterCount,
    parameterTypes: parameters,
    returnType: typeName(api, api.methodGetReturnType(method)),
    token: api.methodGetToken === null ? null : api.methodGetToken(method),
    candidates: candidates,
  };
}

function run(module) {
  if (TARGET_METHODS.length === 0 && TARGET_CLASSES.size === 0
      && TARGET_MODULE_OFFSETS.size === 0 && !INCLUDE_METHOD_INDEX) {
    throw new Error('No IL2CPP native target methods were configured');
  }
  const api = createApi(module);
  const domain = api.domainGet();
  if (domain.isNull()) {
    throw new Error('IL2CPP domain is not initialized');
  }
  api.threadAttach(domain);
  const image = findAssemblyImage(api, domain);
  const classCount = Number(api.imageGetClassCount(image));
  if (classCount > MAX_CLASSES) {
    throw new Error('Refusing implausible class count: ' + classCount);
  }

  const requestedClasses = new Set(TARGET_METHODS.map(function (target) { return target.className; }));
  TARGET_CLASSES.forEach(function (className) { requestedClasses.add(className); });
  const methods = [];
  const methodOffsets = [];
  const methodIndex = [];
  const moduleEnd = module.base.add(module.size);
  for (let classIndex = 0; classIndex < classCount; classIndex += 1) {
    const klass = api.imageGetClass(image, classIndex);
    if (klass.isNull()) {
      continue;
    }
    const className = readCString(api.classGetName(klass));
    const requestedClass = requestedClasses.has(className) || TARGET_MODULE_OFFSETS.size > 0;
    if (!requestedClass && !EXACT_BOUNDARIES) {
      continue;
    }
    const namespaceName = readCString(api.classGetNamespace(klass)) || '';
    const iterator = Memory.alloc(Process.pointerSize);
    iterator.writePointer(NULL);
    while (true) {
      const method = api.classGetMethods(klass, iterator);
      if (method.isNull()) {
        break;
      }
      if (EXACT_BOUNDARIES) {
        try {
          const address = method.readPointer();
          const belongsToModule = !address.isNull()
            && address.compare(module.base) >= 0
            && address.compare(moduleEnd) < 0;
          if (belongsToModule) {
            const moduleOffset = address.sub(module.base).toUInt32();
            methodOffsets.push(moduleOffset);
            if (INCLUDE_METHOD_INDEX) {
              methodIndex.push({
                namespace: namespaceName,
                className: className,
                methodName: readCString(api.methodGetName(method)),
                parameterCount: api.methodGetParamCount(method),
                token: api.methodGetToken === null ? null : api.methodGetToken(method),
                moduleOffset: '0x' + moduleOffset.toString(16),
              });
            }
          }
        } catch (_) {
          // A malformed MethodInfo is ignored; requested methods still retain
          // their bounded prefix and auditable pointer candidates.
        }
      }
      if (requestedClass) {
        const captured = captureMethod(api, module, namespaceName, className, method);
        if (captured !== null) {
          methods.push(captured);
        }
      }
    }
  }

  if (EXACT_BOUNDARIES) {
    const boundaries = Array.from(new Set(methodOffsets)).sort(function (left, right) { return left - right; });
    const nextBoundary = new Map();
    for (let index = 0; index + 1 < boundaries.length; index += 1) {
      nextBoundary.set(boundaries[index], boundaries[index + 1]);
    }
    methods.forEach(function (method) {
      method.candidates.forEach(function (candidate) {
        if (candidate.module !== module.name || candidate.moduleOffset === null) {
          return;
        }
        const offset = parseInt(candidate.moduleOffset, 16);
        const next = nextBoundary.get(offset);
        if (next === undefined) {
          return;
        }
        const nativeSize = next - offset;
        const captureSize = Math.min(nativeSize, CODE_BYTE_LIMIT);
        candidate.nativeSizeBytes = nativeSize;
        candidate.boundaryModuleOffset = '0x' + next.toString(16);
        candidate.codeHex = captureSize > 0
          ? bytesToHex(ptr(candidate.address).readByteArray(captureSize))
          : '';
        candidate.codeTruncated = captureSize < nativeSize;
      });
    });
    methodIndex.forEach(function (method) {
      const offset = parseInt(method.moduleOffset, 16);
      const next = nextBoundary.get(offset);
      method.nativeSizeBytes = next === undefined ? null : next - offset;
    });
  }

  const found = new Set(methods.map(function (method) {
    return method.className + '::' + method.methodName + '/' + method.parameterCount;
  }));
  const missing = TARGET_METHODS.filter(function (target) {
    if (target.parameterCount === undefined) {
      return !methods.some(function (method) {
        return method.className === target.className && method.methodName === target.methodName;
      });
    }
    return !found.has(target.className + '::' + target.methodName + '/' + target.parameterCount);
  });

  emit('il2cpp-native-methods', {
    formatVersion: 1,
    source: 'IL2CPP exported reflection API and MethodInfo pointer prefix',
    module: {
      name: module.name,
      base: module.base.toString(),
      size: module.size,
    },
    assembly: TARGET_ASSEMBLY,
    architecture: Process.arch,
    pointerSize: Process.pointerSize,
    codeByteLimit: CODE_BYTE_LIMIT,
    exactBoundaries: EXACT_BOUNDARIES,
    requested: TARGET_METHODS,
    requestedClasses: Array.from(TARGET_CLASSES),
    requestedModuleOffsets: Array.from(TARGET_MODULE_OFFSETS).map(function (offset) {
      return '0x' + offset.toString(16);
    }),
    methodIndex: INCLUDE_METHOD_INDEX ? methodIndex : undefined,
    missing: missing,
    methods: methods,
  });
}

const timer = setInterval(function () {
  if (started) {
    return;
  }
  let module = null;
  try {
    module = Process.findModuleByName('libil2cpp.so') || Process.findModuleByName('GameAssembly.dylib');
  } catch (_) {
    module = null;
  }
  if (module === null) {
    return;
  }
  try {
    run(module);
    started = true;
    clearInterval(timer);
  } catch (error) {
    readyAttempts += 1;
    if (readyAttempts >= MAX_READY_ATTEMPTS) {
      started = true;
      clearInterval(timer);
      emit('il2cpp-native-methods-error', {
        message: String(error && error.stack ? error.stack : error),
        attempts: readyAttempts,
      });
    }
  }
}, POLL_MS);
