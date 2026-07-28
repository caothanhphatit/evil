'use strict';

function scanRange(module, target, range, hits) {
  const bytes = range.base.readByteArray(range.size);
  if (bytes === null) return;
  const view = new DataView(bytes);
  for (let offset = 0; offset + 4 <= view.byteLength; offset += 4) {
    const word = view.getUint32(offset, true);
    if (((word & 0xfc000000) >>> 0) !== 0x94000000) continue;
    let immediate = word & 0x03ffffff;
    if ((immediate & 0x02000000) !== 0) immediate -= 0x04000000;
    const address = range.base.add(offset);
    if (address.add(immediate * 4).equals(target)) {
      hits.push(address.sub(module.base).toString());
    }
  }
}

function scan(targetOffset, moduleName, requestedRanges) {
  const module = Process.findModuleByName(moduleName);
  if (module === null) {
    setTimeout(() => scan(targetOffset, moduleName), 250);
    return;
  }

  const target = module.base.add(targetOffset);
  const hits = [];
  const ranges = Array.isArray(requestedRanges)
    ? requestedRanges.map((range) => ({
        base: module.base.add(ptr(range.offset)),
        size: range.size,
      }))
    : module.enumerateRanges('r-x');
  for (const range of ranges) {
    scanRange(module, target, range, hits);
  }

  send({
    type: 'arm64-direct-callers',
    module: module.name,
    targetOffset: targetOffset.toString(),
    callSiteOffsets: hits,
  });
}

recv('scan', (message) => {
  const payload = message.payload || {};
  scan(ptr(payload.targetOffset), payload.moduleName || 'libil2cpp.so', payload.ranges);
});
