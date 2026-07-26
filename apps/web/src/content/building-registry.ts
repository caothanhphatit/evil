const REGISTRY_ROOT = "/content/releases/evil-hunter-1.411/";
const REGISTRY_BOOTSTRAP_PATH = `${REGISTRY_ROOT}building-registry-manifest.json`;
const REGISTRY_PATH = `${REGISTRY_ROOT}building-registry.json`;
const SHA256_PATTERN = /^[a-f0-9]{64}$/;
const RESOLVED_CONFIDENCE = new Set(["confirmed", "strongly-inferred", "tentative"]);
const EVIDENCE_METHODS = new Set([
  "serialized-row", "metadata-field", "native-code", "localization-entry", "scene-object", "ui-hierarchy", "asset-object", "runtime-trace",
]);

type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

export interface EvidenceField {
  state: "resolved" | "unresolved";
  confidence: "confirmed" | "strongly-inferred" | "tentative" | "unknown";
  value: JsonValue;
  evidence: EvidenceRef[];
  requiredEvidence: string | null;
}

export interface EvidenceCollection<T = Record<string, unknown>> {
  binding: Omit<EvidenceField, "value">;
  rows: T[];
}

export interface EvidenceBuildingRegistry {
  schemaVersion: 1;
  contractType: "building-registry";
  registryId: "evil-hunter-1.411.buildings-v1";
  runtimeState: "blocked" | "runtime-ready";
  legacy: RuntimeBuildingRegistry["legacy"];
  evidencePolicy: RuntimeBuildingRegistry["evidencePolicy"];
  evidenceSources: RuntimeBuildingRegistry["evidenceSources"];
  catalogs: {
    items: EvidenceCollection;
    products: EvidenceCollection;
    capabilities: EvidenceCollection;
  };
  buildings: EvidenceCollection;
  releaseGate: { runnable: boolean; blockingPaths: string[]; reason: string };
}

export interface BuildingRegistryBootstrap {
  schemaVersion: 1;
  contractType: "building-registry-bootstrap";
  registryId: "evil-hunter-1.411.buildings-v1";
  registryPath: typeof REGISTRY_PATH;
  registryBytes: number;
  registrySha256: string;
}

export interface EvidenceRef {
  sourceId: string;
  locator: string;
  method: string;
  note?: string;
}

export interface ResolvedBinding {
  state: "resolved";
  confidence: "confirmed" | "strongly-inferred" | "tentative";
  evidence: EvidenceRef[];
  requiredEvidence: null;
}

export interface ResolvedField extends ResolvedBinding {
  value: Exclude<JsonValue, null>;
}

export interface RegistryCollection<T = Record<string, JsonValue>> {
  binding: ResolvedBinding;
  rows: T[];
}

export interface RuntimeBuildingRegistry {
  schemaVersion: 1;
  contractType: "building-registry";
  registryId: "evil-hunter-1.411.buildings-v1";
  legacy: {
    game: "Evil Hunter Tycoon";
    version: "1.411";
    package: "com.superplanet.evilhunter";
  };
  runtimeState: "runtime-ready";
  evidencePolicy: {
    semanticFields: "evidence-required-per-field";
    unresolvedValues: "fail-closed-null-or-empty";
    visualBinding: "separate-from-gameplay-semantics";
  };
  evidenceSources: Array<{ id: string; path: string; bytes: number; sha256: string }>;
  catalogs: {
    items: RegistryCollection;
    products: RegistryCollection;
    capabilities: RegistryCollection;
  };
  buildings: RegistryCollection;
  releaseGate: { runnable: true; blockingPaths: []; reason: string };
}

export async function loadVerifiedBuildingRegistry(fetchFn: typeof fetch = fetch): Promise<RuntimeBuildingRegistry> {
  return validateRuntimeBuildingRegistry(await loadVerifiedRegistryPayload(fetchFn));
}

/** Loads decoded fields for read-only UI without treating a blocked registry as executable game data. */
export async function loadVerifiedBuildingEvidenceRegistry(fetchFn: typeof fetch = fetch): Promise<EvidenceBuildingRegistry> {
  return validateEvidenceBuildingRegistry(await loadVerifiedRegistryPayload(fetchFn));
}

async function loadVerifiedRegistryPayload(fetchFn: typeof fetch): Promise<unknown> {
  const bootstrapResponse = await fetchFn(REGISTRY_BOOTSTRAP_PATH, { cache: "no-cache", credentials: "same-origin" });
  if (!bootstrapResponse.ok) throw new Error(`Building-registry bootstrap returned ${bootstrapResponse.status}`);
  const bootstrap = validateBootstrap(await bootstrapResponse.json());

  const registryResponse = await fetchFn(bootstrap.registryPath, { cache: "no-cache", credentials: "same-origin" });
  if (!registryResponse.ok) throw new Error(`Building registry returned ${registryResponse.status}`);
  const payload = new Uint8Array(await registryResponse.arrayBuffer());
  await verifyRegistryBytes(payload, bootstrap.registryBytes, bootstrap.registrySha256);

  let decoded: unknown;
  try {
    decoded = JSON.parse(new TextDecoder().decode(payload)) as unknown;
  } catch {
    throw new Error("Building registry JSON is malformed");
  }
  return decoded;
}

export function validateEvidenceBuildingRegistry(value: unknown): EvidenceBuildingRegistry {
  validateRegistryIdentity(value);
  if (!isRecord(value) || (value.runtimeState !== "blocked" && value.runtimeState !== "runtime-ready")
    || !isRecord(value.releaseGate) || typeof value.releaseGate.runnable !== "boolean"
    || !Array.isArray(value.releaseGate.blockingPaths) || typeof value.releaseGate.reason !== "string"
    || !isRecord(value.catalogs) || !isRecord(value.catalogs.capabilities)
    || !Array.isArray(value.catalogs.capabilities.rows) || !isRecord(value.buildings)
    || !Array.isArray(value.buildings.rows) || value.buildings.rows.length === 0) {
    throw new Error("Building evidence registry structure is invalid");
  }
  for (const [index, building] of value.buildings.rows.entries()) {
    if (!isRecord(building) || typeof building.key !== "string" || !isEvidenceField(building.buildId)
      || !isEvidenceField(building.displayName) || !isRecord(building.levels) || !Array.isArray(building.levels.rows)
      || !isRecord(building.buildRows) || !Array.isArray(building.buildRows.rows)
      || !isRecord(building.visualBinding) || !isEvidenceField(building.visualBinding.popupClass)) {
      throw new Error(`Building evidence row is invalid at buildings.rows[${index}]`);
    }
  }
  return value as unknown as EvidenceBuildingRegistry;
}

export function validateRuntimeBuildingRegistry(value: unknown): RuntimeBuildingRegistry {
  validateRegistryIdentity(value);
  if (value.runtimeState !== "runtime-ready") {
    const reason = isRecord(value.releaseGate) && typeof value.releaseGate.reason === "string" ? `: ${value.releaseGate.reason}` : "";
    throw new Error(`Building registry is blocked${reason}`);
  }
  validateReleaseGate(value.releaseGate);

  const sourceIds = validateEvidenceSources(value.evidenceSources);
  if (!isRecord(value.catalogs)) throw new Error("Building registry catalogs are invalid");
  validateCollection(value.catalogs.items, "catalogs.items", sourceIds, validateItem);
  validateCollection(value.catalogs.products, "catalogs.products", sourceIds, validateProduct);
  validateCollection(value.catalogs.capabilities, "catalogs.capabilities", sourceIds, validateCapability);
  validateCollection(value.buildings, "buildings", sourceIds, validateBuilding);
  if (!isRecord(value.buildings) || !Array.isArray(value.buildings.rows) || value.buildings.rows.length === 0) {
    throw new Error("Runtime-ready building registry has no buildings");
  }
  return value as unknown as RuntimeBuildingRegistry;
}

function validateRegistryIdentity(value: unknown): asserts value is Record<string, unknown> {
  if (!isRecord(value) || value.schemaVersion !== 1 || value.contractType !== "building-registry"
    || value.registryId !== "evil-hunter-1.411.buildings-v1") {
    throw new Error("Building registry identity or schema is invalid");
  }
  if (!isRecord(value.legacy) || value.legacy.game !== "Evil Hunter Tycoon" || value.legacy.version !== "1.411"
    || value.legacy.package !== "com.superplanet.evilhunter") {
    throw new Error("Building registry legacy source is invalid");
  }
  if (!isRecord(value.evidencePolicy) || value.evidencePolicy.semanticFields !== "evidence-required-per-field"
    || value.evidencePolicy.unresolvedValues !== "fail-closed-null-or-empty"
    || value.evidencePolicy.visualBinding !== "separate-from-gameplay-semantics") {
    throw new Error("Building registry evidence policy is invalid");
  }
}

function isEvidenceField(value: unknown): value is EvidenceField {
  return isRecord(value) && (value.state === "resolved" || value.state === "unresolved")
    && "value" in value && Array.isArray(value.evidence)
    && (value.requiredEvidence === null || typeof value.requiredEvidence === "string");
}

export async function verifyRegistryBytes(payload: Uint8Array, expectedBytes: number, expectedSha256: string): Promise<void> {
  if (!Number.isSafeInteger(expectedBytes) || expectedBytes <= 0 || !SHA256_PATTERN.test(expectedSha256)) {
    throw new Error("Building-registry bootstrap integrity metadata is invalid");
  }
  if (payload.byteLength !== expectedBytes) throw new Error("Building registry byte length mismatch");
  const actual = await sha256Hex(payload);
  if (actual !== expectedSha256) throw new Error("Building registry checksum mismatch");
}

function validateBootstrap(value: unknown): BuildingRegistryBootstrap {
  if (!isRecord(value) || value.schemaVersion !== 1 || value.contractType !== "building-registry-bootstrap"
    || value.registryId !== "evil-hunter-1.411.buildings-v1" || value.registryPath !== REGISTRY_PATH
    || !Number.isSafeInteger(value.registryBytes) || (value.registryBytes as number) <= 0
    || typeof value.registrySha256 !== "string" || !SHA256_PATTERN.test(value.registrySha256)) {
    throw new Error("Building-registry bootstrap is invalid");
  }
  return value as unknown as BuildingRegistryBootstrap;
}

function validateReleaseGate(value: unknown): void {
  if (!isRecord(value) || value.runnable !== true || !Array.isArray(value.blockingPaths)
    || value.blockingPaths.length !== 0 || typeof value.reason !== "string" || value.reason.length === 0) {
    throw new Error("Building registry release gate is not runnable");
  }
}

function validateEvidenceSources(value: unknown): Set<string> {
  if (!Array.isArray(value) || value.length === 0) throw new Error("Building registry evidence sources are invalid");
  const ids = new Set<string>();
  for (const [index, source] of value.entries()) {
    if (!isRecord(source) || typeof source.id !== "string" || source.id.length === 0 || ids.has(source.id)
      || typeof source.path !== "string" || source.path.length === 0 || source.path.startsWith("/") || source.path.split(/[\\/]/).includes("..")
      || !Number.isSafeInteger(source.bytes) || (source.bytes as number) <= 0
      || typeof source.sha256 !== "string" || !SHA256_PATTERN.test(source.sha256)) {
      throw new Error(`Building registry evidence source is invalid at evidenceSources[${index}]`);
    }
    ids.add(source.id);
  }
  return ids;
}

type RowValidator = (value: unknown, path: string, sourceIds: Set<string>) => void;

function validateCollection(value: unknown, path: string, sourceIds: Set<string>, validateRow?: RowValidator): void {
  if (!isRecord(value) || !Array.isArray(value.rows)) throw new Error(`Building registry collection is invalid at ${path}`);
  validateResolution(value.binding, `${path}.binding`, sourceIds, false);
  value.rows.forEach((row, index) => (validateRow ?? walkSemanticBindings)(row, `${path}.rows[${index}]`, sourceIds));
}

function validateItem(value: unknown, path: string, sourceIds: Set<string>): void {
  requireRow(value, path, ["itemId", "internalName", "displayName", "itemType", "stackLimit", "buyPrice", "sellPrice"]);
  for (const field of ["itemId", "internalName", "displayName", "itemType", "stackLimit"]) validateField(value[field], `${path}.${field}`, sourceIds);
  validateCollection(value.buyPrice, `${path}.buyPrice`, sourceIds, validateAmount);
  validateCollection(value.sellPrice, `${path}.sellPrice`, sourceIds, validateAmount);
}

function validateProduct(value: unknown, path: string, sourceIds: Set<string>): void {
  requireRow(value, path, ["productId", "buildingId", "inputs", "outputs", "durationMs", "salePrice", "conditions"]);
  for (const field of ["productId", "buildingId", "durationMs"]) validateField(value[field], `${path}.${field}`, sourceIds);
  for (const collection of ["inputs", "outputs", "salePrice"] as const) validateCollection(value[collection], `${path}.${collection}`, sourceIds, validateAmount);
  validateCollection(value.conditions, `${path}.conditions`, sourceIds, validateCondition);
}

function validateCapability(value: unknown, path: string, sourceIds: Set<string>): void {
  requireRow(value, path, ["capabilityId", "buildingId", "kind", "parameters", "conditions"]);
  for (const field of ["capabilityId", "buildingId", "kind", "parameters"]) validateField(value[field], `${path}.${field}`, sourceIds);
  validateCollection(value.conditions, `${path}.conditions`, sourceIds, validateCondition);
}

function validateBuilding(value: unknown, path: string, sourceIds: Set<string>): void {
  requireRow(value, path, ["buildId", "internalName", "displayName", "category", "buildRows", "levels", "tradeRules", "productIds", "capabilityIds", "visualBinding"]);
  for (const field of ["buildId", "internalName", "displayName", "category"]) validateField(value[field], `${path}.${field}`, sourceIds);
  validateCollection(value.buildRows, `${path}.buildRows`, sourceIds, validateBuildRow);
  validateCollection(value.levels, `${path}.levels`, sourceIds, validateLevel);
  validateCollection(value.tradeRules, `${path}.tradeRules`, sourceIds, validateTradeRule);
  validateCollection(value.productIds, `${path}.productIds`, sourceIds, validateReference);
  validateCollection(value.capabilityIds, `${path}.capabilityIds`, sourceIds, validateReference);
  validateVisualBinding(value.visualBinding, `${path}.visualBinding`, sourceIds);
}

function validateBuildRow(value: unknown, path: string, sourceIds: Set<string>): void {
  requireRow(value, path, ["sourceRowId", "buildId", "level", "conditions", "costs", "durationMs"]);
  for (const field of ["sourceRowId", "buildId", "level", "durationMs"]) validateField(value[field], `${path}.${field}`, sourceIds);
  validateCollection(value.conditions, `${path}.conditions`, sourceIds, validateCondition);
  validateCollection(value.costs, `${path}.costs`, sourceIds, validateAmount);
}

function validateLevel(value: unknown, path: string, sourceIds: Set<string>): void {
  requireRow(value, path, ["level", "conditions", "upgradeCosts", "upgradeDurationMs", "inventoryCapacity", "productionSlots", "capabilityIds", "productIds"]);
  for (const field of ["level", "upgradeDurationMs", "inventoryCapacity", "productionSlots"]) validateField(value[field], `${path}.${field}`, sourceIds);
  validateCollection(value.conditions, `${path}.conditions`, sourceIds, validateCondition);
  validateCollection(value.upgradeCosts, `${path}.upgradeCosts`, sourceIds, validateAmount);
  validateCollection(value.capabilityIds, `${path}.capabilityIds`, sourceIds, validateReference);
  validateCollection(value.productIds, `${path}.productIds`, sourceIds, validateReference);
}

function validateTradeRule(value: unknown, path: string, sourceIds: Set<string>): void {
  requireRow(value, path, ["itemId", "direction", "unitPrice", "quantityLimit", "conditions"]);
  for (const field of ["itemId", "direction", "quantityLimit"]) validateField(value[field], `${path}.${field}`, sourceIds);
  validateCollection(value.unitPrice, `${path}.unitPrice`, sourceIds, validateAmount);
  validateCollection(value.conditions, `${path}.conditions`, sourceIds, validateCondition);
}

function validateCondition(value: unknown, path: string, sourceIds: Set<string>): void {
  requireRow(value, path, ["kind", "subjectId", "operator", "operand"]);
  for (const field of ["kind", "subjectId", "operator", "operand"]) validateField(value[field], `${path}.${field}`, sourceIds);
}

function validateAmount(value: unknown, path: string, sourceIds: Set<string>): void {
  requireRow(value, path, ["itemId", "quantity"]);
  validateField(value.itemId, `${path}.itemId`, sourceIds);
  validateField(value.quantity, `${path}.quantity`, sourceIds);
}

function validateReference(value: unknown, path: string, sourceIds: Set<string>): void {
  requireRow(value, path, ["id"]);
  validateField(value.id, `${path}.id`, sourceIds);
}

function validateVisualBinding(value: unknown, path: string, sourceIds: Set<string>): void {
  if (!isRecord(value)) throw new Error(`Building registry visual binding is invalid at ${path}`);
  validateResolution(value.binding, `${path}.binding`, sourceIds, false);
  for (const field of ["spriteAssetId", "controllerClass", "popupClass", "townPosition", "sorting", "collider"]) {
    validateField(value[field], `${path}.${field}`, sourceIds);
  }
}

function validateField(value: unknown, path: string, sourceIds: Set<string>): void {
  validateResolution(value, path, sourceIds, true);
}

function requireRow(value: unknown, path: string, fields: string[]): asserts value is Record<string, unknown> {
  if (!isRecord(value) || typeof value.key !== "string" || value.key.length === 0 || fields.some((field) => !(field in value))) {
    throw new Error(`Building registry row is invalid at ${path}`);
  }
}

function walkSemanticBindings(value: unknown, path: string, sourceIds: Set<string>): void {
  if (Array.isArray(value)) {
    value.forEach((entry, index) => walkSemanticBindings(entry, `${path}[${index}]`, sourceIds));
    return;
  }
  if (!isRecord(value)) return;
  if ("state" in value || "confidence" in value || "requiredEvidence" in value) {
    validateResolution(value, path, sourceIds, "value" in value);
    return;
  }
  if ("rows" in value) {
    if (!isRecord(value.binding) || !Array.isArray(value.rows)) throw new Error(`Building registry collection is invalid at ${path}`);
  }
  Object.entries(value).forEach(([key, entry]) => walkSemanticBindings(entry, `${path}.${key}`, sourceIds));
}

function validateResolution(value: unknown, path: string, sourceIds: Set<string>, isField: boolean): void {
  if (!isRecord(value) || value.state !== "resolved" || typeof value.confidence !== "string"
    || !RESOLVED_CONFIDENCE.has(value.confidence) || value.requiredEvidence !== null || !Array.isArray(value.evidence)
    || value.evidence.length === 0 || (isField && (value.value === null || value.value === undefined))) {
    throw new Error(`Building registry contains blocked or unresolved data at ${path}`);
  }
  for (const [index, evidence] of value.evidence.entries()) {
    if (!isRecord(evidence) || typeof evidence.sourceId !== "string" || !sourceIds.has(evidence.sourceId)
      || typeof evidence.locator !== "string" || evidence.locator.length === 0 || typeof evidence.method !== "string" || !EVIDENCE_METHODS.has(evidence.method)) {
      throw new Error(`Building registry evidence reference is invalid at ${path}.evidence[${index}]`);
    }
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

async function sha256Hex(payload: Uint8Array): Promise<string> {
  const stableBuffer = new ArrayBuffer(payload.byteLength);
  new Uint8Array(stableBuffer).set(payload);
  const digest = await crypto.subtle.digest("SHA-256", stableBuffer);
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}
