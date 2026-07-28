import assert from "node:assert/strict";
import { promises as fs } from "node:fs";
import test from "node:test";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { buildOriginalFlowRelease, validateOriginalFlowRelease } from "../original-flow-content-lib.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

test("original flow preserves source order and blocks unresolved runtime bindings", async () => {
  const release = await buildOriginalFlowRelease(repoRoot);
  assert.deepEqual(release.flows.map((flow) => flow.id), ["boot", "village", "hunter-roster", "field"]);
  assert.equal(release.releaseGate.runnable, false);
  assert.ok(release.releaseGate.blockingBindingIds.includes("field.first-monster"));
  assert.equal(release.flows.find((flow) => flow.id === "field").bindings.find((binding) => binding.id === "field.combat-rules").value, null);
});

test("original flow evidence resolves against scene, inventory, and asset index", async () => {
  const [release, selection] = await Promise.all([
    validateOriginalFlowRelease(repoRoot),
    fs.readFile(path.join(repoRoot, "game-assets/manifests/original-flow-v1.selection.json"), "utf8")
      .then(JSON.parse),
  ]);
  assert.equal(release.assets.length, selection.assets.length);
  assert.deepEqual(
    release.assets
      .filter((asset) => asset.id.startsWith("field.hp-"))
      .map((asset) => [asset.id, asset.unity.name, asset.bindingState, asset.confidence]),
    [
      ["field.hp-fill", "hp_in", "scene-component-confirmed", "confirmed"],
      ["field.hp-background", "hp_bg", "scene-component-confirmed", "confirmed"],
      ["field.hp-level-frame", "hp_lv_bg_9", "scene-component-confirmed", "confirmed"],
    ],
  );
  assert.ok(release.flows.flatMap((flow) => flow.sceneObjects).every((object) => object.confidence === "confirmed"));
  assert.ok(release.assets.every((asset) => /^[a-f0-9]{64}$/.test(asset.sha256)));
});
