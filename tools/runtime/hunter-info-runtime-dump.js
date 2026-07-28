'use strict';

/*
 * Reflection-only IL2CPP schema capture for the hunter information migration.
 * This script does not inspect managed object instances or application saves.
 */

const TARGET_ASSEMBLY = globalThis.HUNTER_SCHEMA_TARGET_ASSEMBLY || 'Assembly-CSharp.dll';
const DEFAULT_TARGET_TYPES = [
  'HunterData',
  'HunterLookData',
  'UserData',
  'SaveData',
  'HunterDetailPop',
];
const configuredTargetTypes = globalThis.HUNTER_SCHEMA_TARGET_TYPES;
const TARGET_TYPES = new Set(
  Array.isArray(configuredTargetTypes) && configuredTargetTypes.length > 0
    ? configuredTargetTypes
    : DEFAULT_TARGET_TYPES
);
const MAX_ASSEMBLIES = 4096;
const MAX_CLASSES = 200000;
const MAX_READY_ATTEMPTS = 80;
const POLL_MS = 250;

let started = false;
let readyAttempts = 0;

function emit(kind, payload) {
  send({ kind: kind, payload: payload });
}

function pointerSizedValue(address) {
  return Process.pointerSize === 8 ? address.readU64().toNumber() : address.readU32();
}

function findExport(module, name, required) {
  let address = null;
  try {
    address = module.findExportByName(name);
  } catch (_) {
    address = null;
  }
  if (address === null && required) {
    throw new Error('Missing required IL2CPP export: ' + name);
  }
  return address;
}

function bind(module, name, returnType, argumentTypes, required) {
  const address = findExport(module, name, required !== false);
  return address === null ? null : new NativeFunction(address, returnType, argumentTypes);
}

function readCString(address) {
  if (address === null || address.isNull()) {
    return null;
  }
  return address.readUtf8String();
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
    classGetFields: bind(module, 'il2cpp_class_get_fields', 'pointer', ['pointer', 'pointer']),
    classGetMethods: bind(module, 'il2cpp_class_get_methods', 'pointer', ['pointer', 'pointer']),
    classGetTypeToken: bind(module, 'il2cpp_class_get_type_token', 'uint32', ['pointer'], false),
    fieldGetName: bind(module, 'il2cpp_field_get_name', 'pointer', ['pointer']),
    fieldGetType: bind(module, 'il2cpp_field_get_type', 'pointer', ['pointer']),
    fieldGetOffset: bind(module, 'il2cpp_field_get_offset', 'int32', ['pointer']),
    fieldGetFlags: bind(module, 'il2cpp_field_get_flags', 'uint32', ['pointer'], false),
    methodGetName: bind(module, 'il2cpp_method_get_name', 'pointer', ['pointer']),
    methodGetReturnType: bind(module, 'il2cpp_method_get_return_type', 'pointer', ['pointer']),
    methodGetParamCount: bind(module, 'il2cpp_method_get_param_count', 'uint32', ['pointer']),
    methodGetParam: bind(module, 'il2cpp_method_get_param', 'pointer', ['pointer', 'uint32']),
    methodGetParamName: bind(module, 'il2cpp_method_get_param_name', 'pointer', ['pointer', 'uint32']),
    methodGetFlags: bind(module, 'il2cpp_method_get_flags', 'uint32', ['pointer', 'pointer'], false),
    methodGetToken: bind(module, 'il2cpp_method_get_token', 'uint32', ['pointer'], false),
    typeGetName: bind(module, 'il2cpp_type_get_name', 'pointer', ['pointer']),
    free: bind(module, 'il2cpp_free', 'void', ['pointer'], false),
  };
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

function dumpFields(api, klass) {
  const fields = [];
  const iterator = Memory.alloc(Process.pointerSize);
  iterator.writePointer(NULL);
  while (true) {
    const field = api.classGetFields(klass, iterator);
    if (field.isNull()) {
      break;
    }
    fields.push({
      name: readCString(api.fieldGetName(field)),
      type: typeName(api, api.fieldGetType(field)),
      offset: api.fieldGetOffset(field),
      flags: api.fieldGetFlags === null ? null : api.fieldGetFlags(field),
    });
  }
  return fields;
}

function dumpMethods(api, klass) {
  const methods = [];
  const iterator = Memory.alloc(Process.pointerSize);
  iterator.writePointer(NULL);
  while (true) {
    const method = api.classGetMethods(klass, iterator);
    if (method.isNull()) {
      break;
    }
    const parameterCount = api.methodGetParamCount(method);
    const parameters = [];
    for (let index = 0; index < parameterCount; index += 1) {
      parameters.push({
        index: index,
        name: readCString(api.methodGetParamName(method, index)),
        type: typeName(api, api.methodGetParam(method, index)),
      });
    }
    const implementationFlags = Memory.alloc(4);
    implementationFlags.writeU32(0);
    methods.push({
      name: readCString(api.methodGetName(method)),
      token: api.methodGetToken === null ? null : api.methodGetToken(method),
      returnType: typeName(api, api.methodGetReturnType(method)),
      parameters: parameters,
      flags: api.methodGetFlags === null ? null : api.methodGetFlags(method, implementationFlags),
      implementationFlags: api.methodGetFlags === null ? null : implementationFlags.readU32(),
    });
  }
  return methods;
}

function dumpClass(api, klass) {
  return {
    namespace: readCString(api.classGetNamespace(klass)) || '',
    name: readCString(api.classGetName(klass)),
    token: api.classGetTypeToken === null ? null : api.classGetTypeToken(klass),
    fields: dumpFields(api, klass),
    methods: dumpMethods(api, klass),
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
  for (let index = 0; index < count; index += 1) {
    const assembly = assemblies.add(index * Process.pointerSize).readPointer();
    const image = api.assemblyGetImage(assembly);
    const name = readCString(api.imageGetName(image));
    const assemblyWithoutExtension = TARGET_ASSEMBLY.replace(/\.dll$/, '');
    if (name === TARGET_ASSEMBLY || name === assemblyWithoutExtension) {
      return image;
    }
  }
  throw new Error(TARGET_ASSEMBLY + ' was not loaded');
}

function run(module) {
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

  const classes = [];
  for (let index = 0; index < classCount; index += 1) {
    const klass = api.imageGetClass(image, index);
    if (klass.isNull()) {
      continue;
    }
    const name = readCString(api.classGetName(klass));
    if (TARGET_TYPES.has(name)) {
      classes.push(dumpClass(api, klass));
    }
  }

  const found = new Set(classes.map(function (item) { return item.name; }));
  emit('hunter-info-schema', {
    formatVersion: 1,
    source: 'IL2CPP exported reflection API',
    module: module.name,
    assembly: TARGET_ASSEMBLY,
    pointerSize: Process.pointerSize,
    architecture: Process.arch,
    targets: Array.from(TARGET_TYPES),
    missing: Array.from(TARGET_TYPES).filter(function (name) { return !found.has(name); }),
    classes: classes,
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
      emit('hunter-info-schema-error', {
        message: String(error && error.stack ? error.stack : error),
        attempts: readyAttempts,
      });
    }
  }
}, POLL_MS);
