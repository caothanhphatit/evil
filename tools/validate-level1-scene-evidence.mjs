import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const evidencePath = path.join(repoRoot, "reverse-engineering/evidence/level1-scene-evidence-v2.json");
const evidence = JSON.parse(await fs.readFile(evidencePath, "utf8"));
if (evidence.schemaVersion !== 2 || evidence.manifestType !== "unity-scene-evidence" || evidence.runtimeCompatibility !== "not-claimed") {
  throw new Error("Unsupported or over-claimed level1 scene evidence manifest");
}
await fs.access(path.join(repoRoot, "packages/content/level1-scene-evidence-v2.schema.json"));
const source = await fs.readFile(path.join(repoRoot, evidence.source.path));
if (source.length !== evidence.source.bytes || createHash("sha256").update(source).digest("hex") !== evidence.source.sha256) {
  throw new Error("Level1 scene evidence source checksum differs");
}
const expectedCoverage = {
  gameObjects: evidence.gameObjects.length,
  transforms: evidence.components.transforms.length,
  spriteRenderers: evidence.components.spriteRenderers.length,
  canvases: evidence.components.canvases.length,
  cameras: evidence.components.cameras.length,
  animators: evidence.components.animators.length,
  colliders: evidence.components.colliders.length,
  uiBehaviours: evidence.components.uiBehaviours.length,
  textMeshes: evidence.components.textMeshes.length,
  canvasGroups: evidence.components.canvasGroups.length,
  diagnostics: evidence.diagnostics.length
};
for (const [name, count] of Object.entries(expectedCoverage)) {
  if (evidence.coverage[name] !== count) throw new Error(`Scene coverage differs for ${name}: ${evidence.coverage[name]} != ${count}`);
}
if (evidence.coverage.transforms !== evidence.objectCounts.Transform + evidence.objectCounts.RectTransform) {
  throw new Error("Transform/RectTransform coverage is incomplete");
}
for (const [coverageName, objectType] of [["spriteRenderers", "SpriteRenderer"], ["canvases", "Canvas"], ["cameras", "Camera"], ["animators", "Animator"]]) {
  if (evidence.coverage[coverageName] !== evidence.objectCounts[objectType]) throw new Error(`${objectType} coverage is incomplete`);
}
if (evidence.coverage.colliders !== evidence.objectCounts.BoxCollider2D + evidence.objectCounts.CircleCollider2D) {
  throw new Error("Collider coverage is incomplete");
}
const gameObjectIds = new Set(evidence.gameObjects.map((object) => object.pathId));
for (const records of Object.values(evidence.components)) {
  const componentIds = new Set();
  for (const component of records) {
    if (componentIds.has(component.pathId)) throw new Error(`Duplicate component path ID: ${component.pathId}`);
    componentIds.add(component.pathId);
    if (!gameObjectIds.has(component.gameObjectPathId)) throw new Error(`Component references unknown GameObject: ${component.pathId}`);
  }
}
if (evidence.coverage.uiPayloadsResolved !== 0 || !evidence.gaps.some((gap) => gap.includes("header-only"))) {
  throw new Error("UI payload limitations must remain explicit");
}
console.log(`Validated level1 scene evidence v2: ${evidence.coverage.gameObjects} GameObjects, ${evidence.coverage.transforms} transforms, ${evidence.coverage.spriteRenderers} SpriteRenderers, ${evidence.coverage.uiBehaviours} UI headers, ${evidence.coverage.diagnostics} diagnostics.`);
