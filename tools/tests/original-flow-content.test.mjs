import assert from "node:assert/strict";
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
  const release = await validateOriginalFlowRelease(repoRoot);
  assert.equal(release.assets.length, 20);
  assert.ok(release.flows.flatMap((flow) => flow.sceneObjects).every((object) => object.confidence === "confirmed"));
  assert.ok(release.assets.every((asset) => /^[a-f0-9]{64}$/.test(asset.sha256)));
});
