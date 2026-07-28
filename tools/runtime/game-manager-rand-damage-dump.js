'use strict';

/* Focused value capture for the 30-entry table consumed by RandDamage(). */

const MAX_READY_ATTEMPTS = 80;
const POLL_MS = 100;
let attempts = 0;
let completed = false;

function bind(module, name, returnType, argumentTypes) {
  const address = module.findExportByName(name);
  if (address === null) {
    throw new Error('Missing IL2CPP export: ' + name);
  }
  return new NativeFunction(address, returnType, argumentTypes);
}

function readCString(address) {
  return address.isNull() ? null : address.readUtf8String();
}

function run(module) {
  const api = {
    domainGet: bind(module, 'il2cpp_domain_get', 'pointer', []),
    threadAttach: bind(module, 'il2cpp_thread_attach', 'pointer', ['pointer']),
    domainGetAssemblies: bind(module, 'il2cpp_domain_get_assemblies', 'pointer', ['pointer', 'pointer']),
    assemblyGetImage: bind(module, 'il2cpp_assembly_get_image', 'pointer', ['pointer']),
    imageGetName: bind(module, 'il2cpp_image_get_name', 'pointer', ['pointer']),
    imageGetClassCount: bind(module, 'il2cpp_image_get_class_count', 'uint64', ['pointer']),
    imageGetClass: bind(module, 'il2cpp_image_get_class', 'pointer', ['pointer', 'uint64']),
    classGetName: bind(module, 'il2cpp_class_get_name', 'pointer', ['pointer']),
    classGetFields: bind(module, 'il2cpp_class_get_fields', 'pointer', ['pointer', 'pointer']),
    fieldGetName: bind(module, 'il2cpp_field_get_name', 'pointer', ['pointer']),
    fieldStaticGetValue: bind(module, 'il2cpp_field_static_get_value', 'void', ['pointer', 'pointer']),
  };

  const domain = api.domainGet();
  if (domain.isNull()) {
    throw new Error('IL2CPP domain is not initialized');
  }
  api.threadAttach(domain);
  const countAddress = Memory.alloc(Process.pointerSize);
  countAddress.writePointer(NULL);
  const assemblies = api.domainGetAssemblies(domain, countAddress);
  const count = Process.pointerSize === 8 ? countAddress.readU64().toNumber() : countAddress.readU32();
  let image = NULL;
  for (let index = 0; index < count; index += 1) {
    const candidate = api.assemblyGetImage(assemblies.add(index * Process.pointerSize).readPointer());
    const name = readCString(api.imageGetName(candidate));
    if (name === 'Assembly-CSharp.dll' || name === 'Assembly-CSharp') {
      image = candidate;
      break;
    }
  }
  if (image.isNull()) {
    throw new Error('Assembly-CSharp.dll was not loaded');
  }

  let gameManager = NULL;
  const classCount = Number(api.imageGetClassCount(image));
  for (let index = 0; index < classCount; index += 1) {
    const candidate = api.imageGetClass(image, index);
    if (!candidate.isNull() && readCString(api.classGetName(candidate)) === 'GameManager') {
      gameManager = candidate;
      break;
    }
  }
  if (gameManager.isNull()) {
    throw new Error('GameManager was not found');
  }

  let singletonField = NULL;
  const iterator = Memory.alloc(Process.pointerSize);
  iterator.writePointer(NULL);
  while (true) {
    const field = api.classGetFields(gameManager, iterator);
    if (field.isNull()) {
      break;
    }
    if (readCString(api.fieldGetName(field)) === 'KPABKEBKMFE') {
      singletonField = field;
      break;
    }
  }
  if (singletonField.isNull()) {
    throw new Error('GameManager singleton field was not found');
  }

  const singletonAddress = Memory.alloc(Process.pointerSize);
  singletonAddress.writePointer(NULL);
  api.fieldStaticGetValue(singletonField, singletonAddress);
  const instance = singletonAddress.readPointer();
  if (instance.isNull()) {
    throw new Error('GameManager singleton is not initialized');
  }

  const currentIndex = instance.add(3552).readS32();
  const valuesArray = instance.add(3560).readPointer();
  if (valuesArray.isNull()) {
    throw new Error('RandDamage value array is null');
  }
  const length = valuesArray.add(0x18).readU32();
  if (length !== 30) {
    throw new Error('Expected 30 RandDamage values, got ' + length);
  }
  const values = [];
  for (let index = 0; index < length; index += 1) {
    values.push(valuesArray.add(0x20 + index * 4).readFloat());
  }
  send({
    kind: 'game-manager-rand-damage-values',
    payload: {
      currentIndex: currentIndex,
      values: values,
      evidence: {
        indexField: { name: 'JPLPBMHEALD', offset: 3552 },
        valuesField: { name: 'JJNKFPHDBPL', offset: 3560 },
        nativeMethodToken: '0x06006B1F',
        nativeMethodVirtualAddress: '0x2706384',
      },
    },
  });
}

const timer = setInterval(function () {
  if (completed) {
    return;
  }
  const module = Process.findModuleByName('libil2cpp.so');
  if (module === null) {
    return;
  }
  try {
    run(module);
    completed = true;
    clearInterval(timer);
  } catch (error) {
    attempts += 1;
    if (attempts >= MAX_READY_ATTEMPTS) {
      completed = true;
      clearInterval(timer);
      send({ kind: 'game-manager-rand-damage-error', payload: { message: String(error) } });
    }
  }
}, POLL_MS);
