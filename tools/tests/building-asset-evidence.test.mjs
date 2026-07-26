import assert from "node:assert/strict";
import { promises as fs } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { validateBuildingAssetEvidence } from "../validate-building-asset-evidence.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const evidencePath = path.join(root, "reverse-engineering/evidence/building-asset-evidence-v1.json");
const registryPath = path.join(root, "packages/content/releases/evil-hunter-1.411/building-registry.json");

async function evidence() {
  return JSON.parse(await fs.readFile(evidencePath, "utf8"));
}

test("building visual evidence keeps exact source-key bindings deterministic", async () => {
  const recovered = await validateBuildingAssetEvidence(await evidence(), root);
  const bindings = new Map(recovered.buildingVisualBindings.map((item) => [item.sourceBuildIndex, item]));
  for (const id of [1, 2, 3, 4, 7, 8, 9, 10, 11, 12, 13, 14, 20, 21]) {
    const binding = bindings.get(id);
    assert.equal(binding.sourceBuildKey, `build_${id}`);
    assert.equal(binding.animationClip.name, `build_${id}`);
    assert.equal(binding.animatorController.name, `build_${id}`);
    assert.deepEqual(binding.animatorController.animationClipPathIds, [binding.animationClip.pathId]);
  }
});

test("building visual evidence fails closed for unsupported semantic bindings", async () => {
  const recovered = await evidence();
  const binding = recovered.buildingVisualBindings.find((item) => item.sourceBuildIndex === 10);
  binding.popupClass = "GearCreatePop";
  await assert.rejects(() => validateBuildingAssetEvidence(recovered, root), /popupClass must remain unresolved/);
});

test("building registry migrates only the exact base visual asset ID", async () => {
  const registry = JSON.parse(await fs.readFile(registryPath, "utf8"));
  const buildings = new Map(registry.buildings.rows.map((item) => [item.buildId.value, item]));
  for (const id of [1, 2, 3, 4, 7, 8, 9, 10, 11, 12, 13, 14, 20, 21]) {
    const visual = buildings.get(`build_${id}`).visualBinding;
    assert.equal(visual.spriteAssetId.state, "resolved");
    assert.equal(visual.spriteAssetId.value, `build_${id}`);
    assert.equal(visual.controllerClass.state, "unresolved");
    assert.equal(visual.popupClass.state, "unresolved");
    assert.equal(visual.townPosition.state, "unresolved");
    assert.equal(visual.sorting.state, "unresolved");
    assert.equal(visual.collider.state, "unresolved");
  }
});
