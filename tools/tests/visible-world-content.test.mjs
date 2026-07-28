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
const level1ScenePath = path.join(repoRoot, "reverse-engineering/evidence/level1-scene-evidence-v2.json");

test("visible-world release exposes evidence-safe fixture metadata", async () => {
  const manifest = JSON.parse(await fs.readFile(releasePath, "utf8"));
  validateVisibleWorldClaims(manifest);
  assert.equal(collectVisibleWorldAssets(manifest).length, 102);
  assert.deepEqual(manifest.village.signboards.map((signboard) => [
    signboard.sceneObject,
    signboard.regionId,
    signboard.states.map((state) => state.densityLevel),
  ]), [
    ["sign_01", "map_new01", [1, 2, 3]],
    ["sign_02", "background_08", [1, 2, 3]],
    ["sign_03", "background_11", [1, 2, 3]],
  ]);
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
  assert.deepEqual(
    manifest.village.foreground
      .filter((piece) => piece.id.startsWith("bridge"))
      .map((piece) => [piece.id, piece.anchor]),
    [
      ["bridgeA", { x: 0.5084745762711864, y: 0.4900688833119918 }],
      ["bridgeB", { x: 0.5113784421321957, y: 0.41964285714285715 }],
      ["bridgeC", { x: 0.48493496855797125, y: 0.4461465488780629 }],
    ],
  );
  assert.equal(manifest.actors.find((actor) => actor.family === "mon_a_01_1").evidence.runtimeRole.confidence, "tentative");
});

test("visible-world bootstrap checksum-pins the release bytes", async () => {
  const release = await fs.readFile(releasePath);
  const bootstrap = JSON.parse(await fs.readFile(path.join(path.dirname(releasePath), "manifest.json"), "utf8"));
  assert.equal(bootstrap.schemaVersion, 3);
  assert.equal(bootstrap.releaseBytes, release.length);
  assert.equal(bootstrap.releaseSha256, createHash("sha256").update(release).digest("hex"));
});

test("village foreground positions come directly from serialized level1 transforms", async () => {
  const [manifest, scene] = await Promise.all([
    fs.readFile(releasePath, "utf8").then(JSON.parse),
    fs.readFile(level1ScenePath, "utf8").then(JSON.parse),
  ]);
  const transformByPathId = new Map(scene.components.transforms.map((transform) => [transform.pathId, transform]));
  for (const piece of manifest.village.foreground) {
    const gameObject = scene.gameObjects.find((candidate) => candidate.name === piece.sceneObject);
    assert.ok(gameObject, `Missing serialized GameObject ${piece.sceneObject}`);
    const transformPathId = gameObject.components.find((component) => component.type === "Transform")?.pathId;
    const transform = transformByPathId.get(transformPathId);
    assert.deepEqual({ x: piece.x, y: piece.y, z: piece.z }, transform.localPosition);
    assert.equal(transform.parent.pathId, 23531);
  }
});

test("visible-world claims reject false confirmation", async () => {
  const manifest = JSON.parse(await fs.readFile(releasePath, "utf8"));
  manifest.actors[0].evidence.skin = { resolution: "unresolved", confidence: "confirmed" };
  assert.throws(() => validateVisibleWorldClaims(manifest), /cannot be confirmed/);
});
