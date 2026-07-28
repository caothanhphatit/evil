'use strict';

/* Traces ACTk PlayerPrefs keys without exporting stored values. */

const MODULE_NAME = 'libil2cpp.so';
const ASSEMBLY_NAME = 'ACTk.Runtime.dll';
const CLASS_NAME = 'ObscuredPrefs';
const METHODS = new Set([
  'GetString', 'SetString', 'GetInt', 'SetInt', 'GetFloat', 'SetFloat',
  'GetBool', 'SetBool', 'HasKey', 'GetRawValue', 'SetRawValue',
]);

let attached = false;
let attempts = 0;

function bind(module, name, returnType, argumentTypes) {
  const address = module.findExportByName(name);
  if (address === null) throw new Error('Missing IL2CPP export: ' + name);
  return new NativeFunction(address, returnType, argumentTypes);
}

function readCString(pointer) {
  return pointer.isNull() ? null : pointer.readUtf8String();
}

function attachHooks(module) {
  const domainGet = bind(module, 'il2cpp_domain_get', 'pointer', []);
  const threadAttach = bind(module, 'il2cpp_thread_attach', 'pointer', ['pointer']);
  const domainGetAssemblies = bind(module, 'il2cpp_domain_get_assemblies', 'pointer', ['pointer', 'pointer']);
  const assemblyGetImage = bind(module, 'il2cpp_assembly_get_image', 'pointer', ['pointer']);
  const imageGetName = bind(module, 'il2cpp_image_get_name', 'pointer', ['pointer']);
  const imageGetClassCount = bind(module, 'il2cpp_image_get_class_count', 'uint64', ['pointer']);
  const imageGetClass = bind(module, 'il2cpp_image_get_class', 'pointer', ['pointer', 'uint64']);
  const classGetName = bind(module, 'il2cpp_class_get_name', 'pointer', ['pointer']);
  const classGetMethods = bind(module, 'il2cpp_class_get_methods', 'pointer', ['pointer', 'pointer']);
  const methodGetName = bind(module, 'il2cpp_method_get_name', 'pointer', ['pointer']);
  const methodGetParamCount = bind(module, 'il2cpp_method_get_param_count', 'uint32', ['pointer']);
  const stringLength = bind(module, 'il2cpp_string_length', 'int32', ['pointer']);
  const stringChars = bind(module, 'il2cpp_string_chars', 'pointer', ['pointer']);

  function managedString(value) {
    if (value.isNull()) return null;
    const length = stringLength(value);
    if (length < 0 || length > 1_000_000) return null;
    return stringChars(value).readUtf16String(length);
  }

  function findImage(domain) {
    const countPointer = Memory.alloc(Process.pointerSize);
    countPointer.writePointer(NULL);
    const assemblies = domainGetAssemblies(domain, countPointer);
    const count = Process.pointerSize === 8
      ? countPointer.readU64().toNumber()
      : countPointer.readU32();
    for (let index = 0; index < count; index += 1) {
      const assembly = assemblies.add(index * Process.pointerSize).readPointer();
      const image = assemblyGetImage(assembly);
      if (readCString(imageGetName(image)) === ASSEMBLY_NAME) return image;
    }
    throw new Error(ASSEMBLY_NAME + ' not loaded');
  }

  function findClass(image) {
    const count = Number(imageGetClassCount(image));
    for (let index = 0; index < count; index += 1) {
      const klass = imageGetClass(image, index);
      if (!klass.isNull() && readCString(classGetName(klass)) === CLASS_NAME) return klass;
    }
    throw new Error(CLASS_NAME + ' not found');
  }

  const domain = domainGet();
  if (domain.isNull()) throw new Error('IL2CPP domain unavailable');
  threadAttach(domain);
  const klass = findClass(findImage(domain));
  const iterator = Memory.alloc(Process.pointerSize);
  iterator.writePointer(NULL);
  const hooked = [];
  while (true) {
    const method = classGetMethods(klass, iterator);
    if (method.isNull()) break;
    const name = readCString(methodGetName(method));
    if (!METHODS.has(name)) continue;
    const implementation = method.readPointer();
    if (implementation.isNull()) continue;
    const parameterCount = methodGetParamCount(method);
    Interceptor.attach(implementation, {
      onEnter(args) {
        const key = managedString(args[0]);
        const secondString = parameterCount > 1 && (name === 'GetString' || name === 'SetString' || name === 'SetRawValue')
          ? managedString(args[1])
          : null;
        send({
          kind: 'actk-playerprefs-key',
          payload: {
            method: name,
            key,
            parameterCount,
            secondStringLength: secondString === null ? null : secondString.length,
          },
        });
      },
    });
    hooked.push({ name, parameterCount, implementation: implementation.toString() });
  }
  send({ kind: 'actk-playerprefs-hooks-ready', payload: { assembly: ASSEMBLY_NAME, className: CLASS_NAME, hooked } });
}

const timer = setInterval(function () {
  if (attached) return;
  attempts += 1;
  const module = Process.findModuleByName(MODULE_NAME);
  if (module === null) return;
  try {
    attachHooks(module);
    attached = true;
    clearInterval(timer);
  } catch (error) {
    if (attempts >= 120) {
      attached = true;
      clearInterval(timer);
      send({ kind: 'actk-playerprefs-trace-error', payload: { message: String(error), attempts } });
    }
  }
}, 250);
