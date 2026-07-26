import assert from "node:assert/strict";
import { promises as fs } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { validateBuildingRegistry } from "../validate-building-registry.mjs";

const fixturePath = path.join(path.dirname(fileURLToPath(import.meta.url)), "fixtures/building-registry.blocked.json");

async function fixture() {
  return JSON.parse(await fs.readFile(fixturePath, "utf8"));
}

test("building registry accepts an explicit non-runnable unresolved contract", async () => {
  const registry = validateBuildingRegistry(await fixture());
  assert.equal(registry.runtimeState, "blocked");
  assert.equal(registry.releaseGate.runnable, false);
  assert.equal(registry.buildings.rows.length, 0);
});

test("building registry rejects fabricated rows behind an unresolved collection", async () => {
  const registry = await fixture();
  registry.buildings.rows.push({ key: "fabricated-building" });
  assert.throws(() => validateBuildingRegistry(registry), /rows must be empty/);
});

test("building registry rejects runnable promotion while blockers remain", async () => {
  const registry = await fixture();
  registry.runtimeState = "runtime-ready";
  registry.releaseGate.runnable = true;
  assert.throws(() => validateBuildingRegistry(registry), /runnable must be false|runtimeState disagrees/);
});

test("building registry requires the release gate to enumerate every unresolved path", async () => {
  const registry = await fixture();
  registry.releaseGate.blockingPaths.pop();
  assert.throws(() => validateBuildingRegistry(registry), /must exactly match unresolved paths/);
});
