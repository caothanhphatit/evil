import assert from "node:assert/strict";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { buildFullAssetCatalog } from "../full-asset-catalog-lib.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

test("full export catalog preserves coverage and unresolved extraction evidence", async () => {
  const catalog = await buildFullAssetCatalog(repoRoot);
  assert.equal(catalog.assets.length, 9359);
  assert.deepEqual(catalog.coverage.counts, { audio: 116, fonts: 2, metadata: 3, sprites: 8980, text: 106, textures: 152 });
  assert.equal(catalog.extractionFailures.length, 0);
  assert.equal(catalog.extractionOutcomes.length, 4);
  assert.equal(catalog.behaviorBinding.defaultState, "unbound-evidence");
});

test("all 53 Spine families are atomic and runtime-addressable", async () => {
  const catalog = await buildFullAssetCatalog(repoRoot);
  assert.equal(catalog.spineFamilies.length, 53);
  assert.equal(new Set(catalog.spineFamilies.map((family) => family.name)).size, 53);
  for (const family of catalog.spineFamilies) {
    assert.match(family.skeleton, /^text\/.*\.json__\d+\.bin$/);
    assert.match(family.atlas, /^text\/.*\.atlas__\d+\.bin$/);
    assert.ok(family.pages.length > 0);
    for (const page of family.pages) assert.match(page, /^textures\/.*__\d+\.png$/);
  }
});
