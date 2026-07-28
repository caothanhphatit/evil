'use strict';

/* Matches candidate plaintext names to ACTk PlayerPrefs entries without reading values. */

const MODULE_NAME = 'libil2cpp.so';
const ASSEMBLY_NAME = 'ACTk.Runtime.dll';
const CLASS_NAME = 'ObscuredPrefs';

function bind(module, name, returnType, argumentTypes) {
  const address = module.findExportByName(name);
  if (address === null) throw new Error('Missing IL2CPP export: ' + name);
  return new NativeFunction(address, returnType, argumentTypes);
}

function readCString(pointer) {
  return pointer.isNull() ? null : pointer.readUtf8String();
}

function createMatcher() {
  const module = Process.getModuleByName(MODULE_NAME);
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
  const stringNew = bind(module, 'il2cpp_string_new', 'pointer', ['pointer']);
  const stringLength = bind(module, 'il2cpp_string_length', 'int32', ['pointer']);
  const stringChars = bind(module, 'il2cpp_string_chars', 'pointer', ['pointer']);

  const domain = domainGet();
  if (domain.isNull()) throw new Error('IL2CPP domain unavailable');
  threadAttach(domain);

  const countPointer = Memory.alloc(Process.pointerSize);
  countPointer.writePointer(NULL);
  const assemblies = domainGetAssemblies(domain, countPointer);
  const assemblyCount = Process.pointerSize === 8 ? countPointer.readU64().toNumber() : countPointer.readU32();
  let image = NULL;
  for (let index = 0; index < assemblyCount; index += 1) {
    const candidate = assemblyGetImage(assemblies.add(index * Process.pointerSize).readPointer());
    if (readCString(imageGetName(candidate)) === ASSEMBLY_NAME) {
      image = candidate;
      break;
    }
  }
  if (image.isNull()) throw new Error(ASSEMBLY_NAME + ' not loaded');

  let klass = NULL;
  const classCount = Number(imageGetClassCount(image));
  for (let index = 0; index < classCount; index += 1) {
    const candidate = imageGetClass(image, index);
    if (!candidate.isNull() && readCString(classGetName(candidate)) === CLASS_NAME) {
      klass = candidate;
      break;
    }
  }
  if (klass.isNull()) throw new Error(CLASS_NAME + ' not found');

  const iterator = Memory.alloc(Process.pointerSize);
  iterator.writePointer(NULL);
  let method = NULL;
  while (true) {
    const candidate = classGetMethods(klass, iterator);
    if (candidate.isNull()) break;
    if (readCString(methodGetName(candidate)) === 'EncryptKey' && methodGetParamCount(candidate) === 1) {
      method = candidate;
      break;
    }
  }
  if (method.isNull()) throw new Error('EncryptKey(String) not found');
  const encryptKey = new NativeFunction(method.readPointer(), 'pointer', ['pointer', 'pointer']);

  return function match(candidates, storedEntries) {
    const matches = [];
    for (const candidate of candidates) {
      const utf8 = Memory.allocUtf8String(candidate);
      const result = encryptKey(stringNew(utf8), method);
      if (result.isNull()) continue;
      const length = stringLength(result);
      const encrypted = stringChars(result).readUtf16String(length);
      if (Object.prototype.hasOwnProperty.call(storedEntries, encrypted)) {
        matches.push({ candidate, valueLength: storedEntries[encrypted] });
      }
    }
    return matches;
  };
}

let matcher = null;
rpc.exports = {
  match(candidates, storedEntries) {
    if (matcher === null) matcher = createMatcher();
    return matcher(candidates, storedEntries);
  },
};
