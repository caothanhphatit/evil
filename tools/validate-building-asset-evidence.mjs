import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const FOCUS_IDS = [1, 2, 3, 4, 7, 8, 9, 10, 11, 12, 13, 14, 20, 21];

function fail(message) {
  throw new Error(`Building asset evidence validation failed: ${message}`);
}

async function sha256(file) {
  return createHash("sha256").update(await fs.readFile(file)).digest("hex");
}

function unique(values, label) {
  if (new Set(values).size !== values.length) fail(`${label} contains duplicates`);
}

export async function validateBuildingAssetEvidence(evidence, repositoryRoot = root) {
  if (evidence.schemaVersion !== 1) fail("unsupported schemaVersion");
  if (evidence.runtimeCompatibility !== "evidence-only") fail("runtimeCompatibility must remain evidence-only");
  if (!Array.isArray(evidence.buildingAnimationAssets) || evidence.buildingAnimationAssets.length !== 44) {
    fail("expected the 44 recovered build animation assets");
  }
  if (!Array.isArray(evidence.buildingVisualBindings)) fail("buildingVisualBindings must be an array");

  const generic = evidence.genericBuildingPrefab;
  if (generic?.scope !== "generic-template-not-bound-to-individual-build-ids") fail("generic prefab scope must forbid per-ID inference");
  if (generic.controllerClass !== "BuildCtrl") fail("generic building controller class mismatch");
  if (generic.collider?.type !== "CapsuleCollider2D" || generic.collider.pathId !== 6060) fail("generic building collider mismatch");
  if (generic.visualChild?.defaultAnimatorController !== null) fail("generic prefab must retain its null default animator controller");

  const source = evidence.source;
  const tablesPath = path.resolve(repositoryRoot, source.serializedBuildingTables);
  const sharedAssetsPath = path.resolve(repositoryRoot, source.sharedAssets);
  if (await sha256(tablesPath) !== source.serializedBuildingTablesSha256) fail("serialized building table hash mismatch");
  if (await sha256(sharedAssetsPath) !== source.sharedAssetsSha256) fail("shared assets hash mismatch");

  unique(evidence.buildingAnimationAssets.map((item) => item.animationClip), "animation names");
  unique(evidence.buildingVisualBindings.map((item) => item.sourceBuildIndex), "visual source indices");
  unique(evidence.buildingVisualBindings.map((item) => item.sourceBuildKey), "visual source keys");

  const animations = new Map(evidence.buildingAnimationAssets.map((item) => [item.animationClip, item]));
  const bindings = new Map(evidence.buildingVisualBindings.map((item) => [item.sourceBuildIndex, item]));
  for (const binding of evidence.buildingVisualBindings) {
    const expectedKey = `build_${binding.sourceBuildIndex}`;
    if (binding.sourceBuildKey !== expectedKey) fail(`${expectedKey} source key mismatch`);
    if (binding.bindingConfidence !== "confirmed-exact-serialized-key-join") fail(`${expectedKey} has invalid confidence`);
    if (binding.animationClip?.name !== expectedKey) fail(`${expectedKey} animation clip mismatch`);
    if (binding.animatorController?.name !== expectedKey) fail(`${expectedKey} animator controller mismatch`);
    if (JSON.stringify(binding.animatorController.animationClipPathIds) !== JSON.stringify([binding.animationClip.pathId])) {
      fail(`${expectedKey} controller does not reference its exact base clip`);
    }
    const animation = animations.get(expectedKey);
    if (!animation || animation.animationClipPathId !== binding.animationClip.pathId) fail(`${expectedKey} clip locator mismatch`);
    if (!Array.isArray(binding.spriteFrames) || binding.spriteFrames.length === 0) fail(`${expectedKey} has no sprite frames`);
    for (const field of ["controllerClass", "popupClass", "townPosition", "sorting", "collider"]) {
      if (binding[field] !== null) fail(`${expectedKey}.${field} must remain unresolved without per-building evidence`);
    }
  }

  for (const sourceBuildIndex of FOCUS_IDS) {
    if (!bindings.has(sourceBuildIndex)) fail(`missing focus binding build_${sourceBuildIndex}`);
  }
  return evidence;
}

export async function validateBuildingAssetEvidenceFile(file, repositoryRoot = root) {
  return validateBuildingAssetEvidence(JSON.parse(await fs.readFile(file, "utf8")), repositoryRoot);
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) {
  const file = process.argv[2];
  if (!file) fail("usage: node tools/validate-building-asset-evidence.mjs <evidence.json>");
  const evidence = await validateBuildingAssetEvidenceFile(path.resolve(file));
  console.log(`Validated building asset evidence: bindings=${evidence.buildingVisualBindings.length}`);
}
