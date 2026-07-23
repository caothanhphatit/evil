import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";

export const ORIGINAL_FLOW_SELECTION = "game-assets/manifests/original-flow-v1.selection.json";
export const ORIGINAL_FLOW_RELEASE = "game-assets/manifests/releases/original-flow-v1.json";

const CONFIDENCE = new Set(["confirmed", "strongly-inferred", "tentative", "unknown"]);
const REQUIRED_FLOWS = ["boot", "village", "hunter-roster", "field"];

function sha256(payload) {
  return createHash("sha256").update(payload).digest("hex");
}

function assertSafeRelativePath(value, label) {
  if (!value || path.isAbsolute(value) || value.split(/[\\/]/).includes("..")) {
    throw new Error(`${label} must be a safe relative path: ${value}`);
  }
}

function assertUnique(values, label) {
  const seen = new Set();
  for (const value of values) {
    if (seen.has(value)) throw new Error(`Duplicate ${label}: ${value}`);
    seen.add(value);
  }
}

async function readJson(file) {
  return JSON.parse(await fs.readFile(file, "utf8"));
}

async function evidenceSource(repoRoot, relativePath) {
  assertSafeRelativePath(relativePath, "evidence source");
  const payload = await fs.readFile(path.join(repoRoot, relativePath));
  return { path: relativePath, bytes: payload.length, sha256: sha256(payload) };
}

export async function buildOriginalFlowRelease(repoRoot) {
  const selectionPath = path.join(repoRoot, ORIGINAL_FLOW_SELECTION);
  const selectionPayload = await fs.readFile(selectionPath);
  const selection = JSON.parse(selectionPayload.toString("utf8"));
  if (selection.schemaVersion !== 1) throw new Error(`Unsupported original-flow selection schema: ${selection.schemaVersion}`);
  assertSafeRelativePath(selection.releaseId, "releaseId");

  const flowIds = selection.flows.map((flow) => flow.id);
  if (JSON.stringify(flowIds) !== JSON.stringify(REQUIRED_FLOWS)) {
    throw new Error(`Original flow must be ordered exactly: ${REQUIRED_FLOWS.join(" -> ")}`);
  }
  assertUnique(flowIds, "flow ID");
  assertUnique(selection.assets.map((asset) => asset.id), "asset ID");

  const sourcePaths = Object.values(selection.sources);
  const [assetIndex, inventory, sceneHierarchy] = await Promise.all(sourcePaths.map((source) => readJson(path.join(repoRoot, source))));
  const assetByPath = new Map(assetIndex.assets.map((asset) => [asset.path, asset]));
  const sceneById = new Map(sceneHierarchy.gameObjects.map((object) => [object.pathId, object]));
  const selectedAssetIds = new Set(selection.assets.map((asset) => asset.id));

  const assets = selection.assets.map((asset) => {
    if (!CONFIDENCE.has(asset.confidence)) throw new Error(`Invalid confidence for ${asset.id}`);
    assertSafeRelativePath(asset.sourcePath, `sourcePath for ${asset.id}`);
    const indexed = assetByPath.get(asset.sourcePath);
    if (!indexed) throw new Error(`Asset index does not contain ${asset.sourcePath}`);
    const unityObject = inventory.find((entry) =>
      entry.source === asset.unity.source && entry.path_id === asset.unity.pathId &&
      entry.type === asset.unity.type && entry.name === asset.unity.name
    );
    if (!unityObject) throw new Error(`Unity evidence does not resolve for ${asset.id}`);
    return { ...asset, bytes: indexed.bytes, sha256: indexed.sha256 };
  });

  const bindingIds = [];
  const flows = selection.flows.map((flow, index) => {
    if (flow.order !== index + 1) throw new Error(`Flow ${flow.id} has invalid order ${flow.order}`);
    if (!CONFIDENCE.has(flow.confidence)) throw new Error(`Invalid confidence for flow ${flow.id}`);
    for (const assetId of flow.assetIds) {
      if (!selectedAssetIds.has(assetId)) throw new Error(`Flow ${flow.id} references missing asset ${assetId}`);
    }
    const sceneObjects = flow.sceneObjects.map((reference) => {
      const sceneObject = sceneById.get(reference.pathId);
      if (!sceneObject || sceneObject.name !== reference.name) {
        throw new Error(`Scene object does not resolve: ${reference.pathId}/${reference.name}`);
      }
      return { ...reference, confidence: "confirmed" };
    });
    const bindings = flow.bindings.map((binding) => {
      bindingIds.push(binding.id);
      return { ...binding, state: "unresolved", confidence: "unknown", value: null };
    });
    return { ...flow, status: bindings.length ? "binding-blocked" : "evidence-ready", sceneObjects, bindings };
  });
  assertUnique(bindingIds, "binding ID");

  return {
    schemaVersion: 1,
    releaseId: selection.releaseId,
    legacy: selection.legacy,
    evidencePolicy: {
      allowedConfidence: ["confirmed", "strongly-inferred", "tentative", "unknown"],
      numericRulePolicy: "legacy-verified-or-observed-only",
      runtimeBindingPolicy: "unresolved-bindings-block-release"
    },
    evidenceSources: await Promise.all([
      evidenceSource(repoRoot, ORIGINAL_FLOW_SELECTION),
      ...sourcePaths.map((source) => evidenceSource(repoRoot, source))
    ]),
    flows,
    assets,
    releaseGate: {
      runnable: false,
      blockingBindingIds: bindingIds,
      reason: "Static evidence identifies the original flow surfaces, but runtime bindings and numeric rules remain unverified."
    }
  };
}

export function serializeOriginalFlowRelease(release) {
  return `${JSON.stringify(release, null, 2)}\n`;
}

export async function validateOriginalFlowRelease(repoRoot) {
  const generated = serializeOriginalFlowRelease(await buildOriginalFlowRelease(repoRoot));
  const committed = await fs.readFile(path.join(repoRoot, ORIGINAL_FLOW_RELEASE), "utf8");
  if (committed !== generated) throw new Error("Original-flow release manifest is stale; run assets:generate:original-flow");
  return JSON.parse(generated);
}
