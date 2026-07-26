import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";
import test from "node:test";
import {
  collectVisibleWorldAssets,
  validateVisibleWorldClaims
} from "../visible-world-content-lib.mjs";

const repoRoot = path.resolve(import.meta.dirname, "../..");
const releasePath = path.join(repoRoot, "apps/web/public/content/releases/visible-world-v1/release.json");

test("visible-world release exposes evidence-safe fixture metadata", async () => {
  const manifest = JSON.parse(await fs.readFile(releasePath, "utf8"));
  validateVisibleWorldClaims(manifest);
  assert.equal(collectVisibleWorldAssets(manifest).length, 91);
  assert.deepEqual(manifest.village.buildings.map((building) => building.id),
    Array.from({ length: 28 }, (_, index) => `build_${index + 1}`));
  assert.deepEqual(manifest.village.buildings.slice(4, 9).map((building) => [building.id, building.sourceSprite]), [
    ["build_5", "bd_a_005_0"],
    ["build_6", "bd_a_006_0"],
    ["build_7", "bd_a_007_0"],
    ["build_8", "bd_a_008_0"],
    ["build_9", "bd_a_009_0"],
  ]);
  assert.ok(manifest.village.buildings.every((building) => !("name" in building) && !("feature" in building)));
  assert.ok(manifest.village.buildings.every((building) => building.semanticBinding === "unresolved"
    && building.contractEvidence === "reverse-engineering/evidence/building-ui-contract-v1.json"));
  assert.equal(manifest.actors.find((actor) => actor.family === "mon_a_01_1").evidence.runtimeRole.confidence, "tentative");
});

test("visible-world bootstrap checksum-pins the release bytes", async () => {
  const release = await fs.readFile(releasePath);
  const bootstrap = JSON.parse(await fs.readFile(path.join(path.dirname(releasePath), "manifest.json"), "utf8"));
  assert.equal(bootstrap.schemaVersion, 3);
  assert.equal(bootstrap.releaseBytes, release.length);
  assert.equal(bootstrap.releaseSha256, createHash("sha256").update(release).digest("hex"));
});

test("visible-world claims reject false confirmation", async () => {
  const manifest = JSON.parse(await fs.readFile(releasePath, "utf8"));
  manifest.actors[0].evidence.skin = { resolution: "unresolved", confidence: "confirmed" };
  assert.throws(() => validateVisibleWorldClaims(manifest), /cannot be confirmed/);
});
