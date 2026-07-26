import { promises as fs } from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const CONFIDENCE = new Set(["confirmed", "strongly-inferred", "tentative", "unknown"]);
const RESOLVED_CONFIDENCE = new Set(["confirmed", "strongly-inferred", "tentative"]);
const EVIDENCE_METHODS = new Set([
  "serialized-row",
  "metadata-field",
  "native-code",
  "localization-entry",
  "scene-object",
  "ui-hierarchy",
  "asset-object",
  "runtime-trace"
]);

function fail(message) {
  throw new Error(`Building registry validation failed: ${message}`);
}

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function assertExactKeys(value, required, optional, label) {
  if (!isObject(value)) fail(`${label} must be an object`);
  const allowed = new Set([...required, ...optional]);
  for (const key of required) if (!(key in value)) fail(`${label}.${key} is required`);
  for (const key of Object.keys(value)) if (!allowed.has(key)) fail(`${label}.${key} is not allowed`);
}

function assertString(value, label) {
  if (typeof value !== "string" || value.length === 0) fail(`${label} must be a non-empty string`);
}

function assertUnique(values, label) {
  const seen = new Set();
  for (const value of values) {
    if (seen.has(value)) fail(`duplicate ${label}: ${value}`);
    seen.add(value);
  }
}

function validateEvidenceRefs(refs, label, sourceIds) {
  if (!Array.isArray(refs)) fail(`${label} must be an array`);
  refs.forEach((ref, index) => {
    const refLabel = `${label}[${index}]`;
    assertExactKeys(ref, ["sourceId", "locator", "method"], ["note"], refLabel);
    assertString(ref.sourceId, `${refLabel}.sourceId`);
    if (!sourceIds.has(ref.sourceId)) fail(`${refLabel}.sourceId does not resolve: ${ref.sourceId}`);
    assertString(ref.locator, `${refLabel}.locator`);
    if (!EVIDENCE_METHODS.has(ref.method)) fail(`${refLabel}.method is invalid: ${ref.method}`);
    if ("note" in ref) assertString(ref.note, `${refLabel}.note`);
  });
}

function validateResolution(value, label, sourceIds, unresolvedPaths) {
  const isField = Object.hasOwn(value, "value");
  assertExactKeys(
    value,
    isField ? ["state", "confidence", "value", "evidence", "requiredEvidence"] : ["state", "confidence", "evidence", "requiredEvidence"],
    [],
    label
  );
  if (value.state !== "resolved" && value.state !== "unresolved") fail(`${label}.state is invalid`);
  if (!CONFIDENCE.has(value.confidence)) fail(`${label}.confidence is invalid`);
  validateEvidenceRefs(value.evidence, `${label}.evidence`, sourceIds);

  if (value.state === "resolved") {
    if (!RESOLVED_CONFIDENCE.has(value.confidence)) fail(`${label} resolved confidence cannot be unknown`);
    if (value.evidence.length === 0) fail(`${label} resolved value requires evidence`);
    if (value.requiredEvidence !== null) fail(`${label}.requiredEvidence must be null when resolved`);
    if (isField && value.value === null) fail(`${label}.value cannot be null when resolved`);
    return;
  }

  unresolvedPaths.push(label);
  if (value.confidence !== "unknown") fail(`${label} must use unknown confidence while unresolved`);
  if (typeof value.requiredEvidence !== "string" || value.requiredEvidence.length === 0) {
    fail(`${label}.requiredEvidence must explain how to resolve the field`);
  }
  if (isField && value.value !== null) fail(`${label}.value must be null while unresolved`);
}

function walkEvidenceBindings(value, label, sourceIds, unresolvedPaths) {
  if (Array.isArray(value)) {
    value.forEach((entry, index) => walkEvidenceBindings(entry, `${label}[${index}]`, sourceIds, unresolvedPaths));
    return;
  }
  if (!isObject(value)) return;

  const resolutionKeys = ["state", "confidence", "evidence", "requiredEvidence"];
  if (resolutionKeys.every((key) => Object.hasOwn(value, key))) {
    validateResolution(value, label, sourceIds, unresolvedPaths);
    return;
  }

  if (isObject(value.binding) && Array.isArray(value.rows)) {
    if (value.binding.state === "unresolved" && value.rows.length !== 0) {
      fail(`${label}.rows must be empty while ${label}.binding is unresolved`);
    }
  }

  for (const [key, entry] of Object.entries(value)) {
    walkEvidenceBindings(entry, label ? `${label}.${key}` : key, sourceIds, unresolvedPaths);
  }
}

function resolvedId(row, fieldName) {
  const field = row?.[fieldName];
  return field?.state === "resolved" && typeof field.value === "string" ? field.value : null;
}

function validateKeyedRows(rows, label, idField) {
  if (!Array.isArray(rows)) fail(`${label} must be an array`);
  rows.forEach((row, index) => assertString(row?.key, `${label}[${index}].key`));
  assertUnique(rows.map((row) => row.key), `${label} key`);
  const ids = rows.map((row) => resolvedId(row, idField)).filter(Boolean);
  assertUnique(ids, `${label} resolved ${idField}`);
  return new Set(ids);
}

function collectLocalUnresolved(value, label = "") {
  const paths = [];
  if (Array.isArray(value)) {
    value.forEach((entry, index) => paths.push(...collectLocalUnresolved(entry, `${label}[${index}]`)));
    return paths;
  }
  if (!isObject(value)) return paths;
  if (["state", "confidence", "evidence", "requiredEvidence"].every((key) => Object.hasOwn(value, key))) {
    if (value.state === "unresolved") paths.push(label);
    return paths;
  }
  for (const [key, entry] of Object.entries(value)) {
    if (key === "readiness") continue;
    paths.push(...collectLocalUnresolved(entry, label ? `${label}.${key}` : key));
  }
  return paths;
}

function validateCapabilityReadiness(capability, label) {
  assertExactKeys(
    capability,
    ["key", "capabilityId", "buildingId", "kind", "parameters", "popupTemplateId", "popupBinding", "runtimeBinding", "conditions", "readiness"],
    [],
    label
  );
  const readiness = capability.readiness;
  assertExactKeys(readiness, ["staticDataReady", "runnable", "blockingPaths", "reason"], [], `${label}.readiness`);
  if (typeof readiness.staticDataReady !== "boolean") fail(`${label}.readiness.staticDataReady must be boolean`);
  if (typeof readiness.runnable !== "boolean") fail(`${label}.readiness.runnable must be boolean`);
  if (!Array.isArray(readiness.blockingPaths)) fail(`${label}.readiness.blockingPaths must be an array`);
  assertUnique(readiness.blockingPaths, `${label}.readiness blocker path`);
  assertString(readiness.reason, `${label}.readiness.reason`);

  const staticFields = ["capabilityId", "buildingId", "kind", "parameters", "popupTemplateId"];
  const staticReady = staticFields.every((field) => capability[field]?.state === "resolved");
  if (readiness.staticDataReady !== staticReady) {
    fail(`${label}.readiness.staticDataReady must be ${staticReady}`);
  }
  const expectedBlockers = [...new Set(collectLocalUnresolved(capability))].sort();
  const declaredBlockers = [...readiness.blockingPaths].sort();
  if (JSON.stringify(expectedBlockers) !== JSON.stringify(declaredBlockers)) {
    fail(`${label}.readiness.blockingPaths must exactly match local unresolved paths`);
  }
  if (readiness.runnable !== (expectedBlockers.length === 0)) {
    fail(`${label}.readiness.runnable disagrees with local unresolved bindings`);
  }
}

function validateReferences(registry) {
  const itemIds = validateKeyedRows(registry.catalogs.items.rows, "catalogs.items.rows", "itemId");
  const productIds = validateKeyedRows(registry.catalogs.products.rows, "catalogs.products.rows", "productId");
  const capabilityIds = validateKeyedRows(registry.catalogs.capabilities.rows, "catalogs.capabilities.rows", "capabilityId");
  const buildingIds = validateKeyedRows(registry.buildings.rows, "buildings.rows", "buildId");
  validateKeyedRows(registry.catalogs.skins.rows, "catalogs.skins.rows", "missingCompositeId");

  const checkResolvedRef = (field, ids, label) => {
    if (field?.state === "resolved" && typeof field.value === "string" && !ids.has(field.value)) {
      fail(`${label} does not resolve: ${field.value}`);
    }
  };

  for (const [index, product] of registry.catalogs.products.rows.entries()) {
    checkResolvedRef(product.buildingId, buildingIds, `catalogs.products.rows[${index}].buildingId`);
    for (const collectionName of ["inputs", "outputs", "salePrice"]) {
      product[collectionName]?.rows.forEach((amount, amountIndex) =>
        checkResolvedRef(amount.itemId, itemIds, `catalogs.products.rows[${index}].${collectionName}.rows[${amountIndex}].itemId`)
      );
    }
  }
  for (const [index, capability] of registry.catalogs.capabilities.rows.entries()) {
    validateCapabilityReadiness(capability, `catalogs.capabilities.rows[${index}]`);
    checkResolvedRef(capability.buildingId, buildingIds, `catalogs.capabilities.rows[${index}].buildingId`);
  }
  for (const [index, skin] of registry.catalogs.skins.rows.entries()) {
    checkResolvedRef(skin.buildingId, buildingIds, `catalogs.skins.rows[${index}].buildingId`);
    skin.costs.rows.forEach((amount, amountIndex) =>
      checkResolvedRef(amount.itemId, itemIds, `catalogs.skins.rows[${index}].costs.rows[${amountIndex}].itemId`)
    );
  }
  for (const [buildingIndex, building] of registry.buildings.rows.entries()) {
    building.productIds.rows.forEach((ref, index) =>
      checkResolvedRef(ref.id, productIds, `buildings.rows[${buildingIndex}].productIds.rows[${index}].id`)
    );
    building.capabilityIds.rows.forEach((ref, index) =>
      checkResolvedRef(ref.id, capabilityIds, `buildings.rows[${buildingIndex}].capabilityIds.rows[${index}].id`)
    );
  }
}

export function validateBuildingRegistry(registry) {
  assertExactKeys(
    registry,
    ["schemaVersion", "contractType", "registryId", "legacy", "runtimeState", "evidencePolicy", "evidenceSources", "catalogs", "buildings", "releaseGate"],
    [],
    "registry"
  );
  if (registry.schemaVersion !== 1 || registry.contractType !== "building-registry") fail("unsupported contract version or type");
  assertString(registry.registryId, "registry.registryId");
  if (!/^[a-z0-9][a-z0-9.-]+$/.test(registry.registryId)) fail("registry.registryId has an invalid format");
  if (registry.runtimeState !== "blocked" && registry.runtimeState !== "runtime-ready") fail("registry.runtimeState is invalid");

  assertExactKeys(registry.legacy, ["game", "version", "package"], [], "registry.legacy");
  if (registry.legacy.game !== "Evil Hunter Tycoon" || registry.legacy.version !== "1.411" || registry.legacy.package !== "com.superplanet.evilhunter") {
    fail("registry.legacy must identify the pinned 1.411 source");
  }
  assertExactKeys(registry.evidencePolicy, ["semanticFields", "unresolvedValues", "visualBinding"], [], "registry.evidencePolicy");
  if (registry.evidencePolicy.semanticFields !== "evidence-required-per-field"
    || registry.evidencePolicy.unresolvedValues !== "fail-closed-null-or-empty"
    || registry.evidencePolicy.visualBinding !== "separate-from-gameplay-semantics") {
    fail("registry.evidencePolicy is not fail-closed");
  }

  if (!Array.isArray(registry.evidenceSources)) fail("registry.evidenceSources must be an array");
  const sourceIds = new Set();
  registry.evidenceSources.forEach((source, index) => {
    const label = `registry.evidenceSources[${index}]`;
    assertExactKeys(source, ["id", "path", "bytes", "sha256"], [], label);
    assertString(source.id, `${label}.id`);
    assertString(source.path, `${label}.path`);
    if (path.isAbsolute(source.path) || source.path.split(/[\\/]/).includes("..")) fail(`${label}.path must be repository-relative`);
    if (!Number.isInteger(source.bytes) || source.bytes < 1) fail(`${label}.bytes must be a positive integer`);
    if (!/^[a-f0-9]{64}$/.test(source.sha256)) fail(`${label}.sha256 is invalid`);
    if (sourceIds.has(source.id)) fail(`duplicate evidence source ID: ${source.id}`);
    sourceIds.add(source.id);
  });

  assertExactKeys(registry.catalogs, ["items", "products", "capabilities", "skins"], [], "registry.catalogs");
  for (const name of ["items", "products", "capabilities", "skins"]) {
    assertExactKeys(registry.catalogs[name], ["binding", "rows"], [], `registry.catalogs.${name}`);
    if (!Array.isArray(registry.catalogs[name].rows)) fail(`registry.catalogs.${name}.rows must be an array`);
  }
  assertExactKeys(registry.buildings, ["binding", "rows"], [], "registry.buildings");
  if (!Array.isArray(registry.buildings.rows)) fail("registry.buildings.rows must be an array");
  assertExactKeys(registry.releaseGate, ["runnable", "blockingPaths", "reason"], [], "registry.releaseGate");
  if (typeof registry.releaseGate.runnable !== "boolean") fail("registry.releaseGate.runnable must be boolean");
  if (!Array.isArray(registry.releaseGate.blockingPaths)) fail("registry.releaseGate.blockingPaths must be an array");
  assertUnique(registry.releaseGate.blockingPaths, "release blocker path");
  assertString(registry.releaseGate.reason, "registry.releaseGate.reason");

  const unresolvedPaths = [];
  walkEvidenceBindings(registry.catalogs, "catalogs", sourceIds, unresolvedPaths);
  walkEvidenceBindings(registry.buildings, "buildings", sourceIds, unresolvedPaths);
  validateReferences(registry);

  const expectedBlockers = [...new Set(unresolvedPaths)].sort();
  const declaredBlockers = [...registry.releaseGate.blockingPaths].sort();
  if (JSON.stringify(expectedBlockers) !== JSON.stringify(declaredBlockers)) {
    fail(`releaseGate.blockingPaths must exactly match unresolved paths; expected ${expectedBlockers.join(", ") || "none"}`);
  }
  const shouldRun = expectedBlockers.length === 0;
  if (registry.releaseGate.runnable !== shouldRun) fail(`releaseGate.runnable must be ${shouldRun}`);
  if (registry.runtimeState !== (shouldRun ? "runtime-ready" : "blocked")) fail("runtimeState disagrees with unresolved bindings");
  if (shouldRun && registry.buildings.rows.length === 0) fail("a runnable registry must contain at least one exact building row");

  return registry;
}

export async function validateBuildingRegistryFile(file) {
  return validateBuildingRegistry(JSON.parse(await fs.readFile(file, "utf8")));
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) {
  const file = process.argv[2];
  if (!file) fail("usage: node tools/validate-building-registry.mjs <registry.json>");
  const registry = await validateBuildingRegistryFile(path.resolve(file));
  console.log(`Validated ${registry.registryId}: buildings=${registry.buildings.rows.length}, runnable=${registry.releaseGate.runnable}`);
}
