const RELEASE_ROOT = "/content/releases/visible-world-v1/";
const BOOTSTRAP_PATH = `${RELEASE_ROOT}manifest.json`;
const RELEASE_PATH = `${RELEASE_ROOT}release.json`;
const EXPECTED_FAMILIES = ["hunter", "Chief", "Npc", "npc_animal", "pet", "mon_goldblin", "mon_a_01_1"] as const;
const SHA256_PATTERN = /^[a-f0-9]{64}$/;

export interface VisibleWorldAsset {
  publicPath: string;
  bytes: number;
  sha256: string;
}

export interface ActorBundle {
  family: string;
  runtimeUse: "migration-fixture";
  evidence: { note: string };
  skeleton: VisibleWorldAsset;
  atlas: VisibleWorldAsset;
  texture: VisibleWorldAsset;
}

export interface ScenePiece extends Partial<VisibleWorldAsset> {
  id?: string;
  publicPath: string;
  x: number;
  y: number;
  z?: number;
  scale?: number;
  anchor?: { x?: number; y?: number };
}

export interface TownBuilding extends VisibleWorldAsset {
  id: string;
  sourceSprite: string;
  runtimeUse: "migration-fixture-town-candidate";
  positionConfidence: "unresolved-runtime-placement";
  contractEvidence: "reverse-engineering/evidence/building-ui-contract-v1.json";
  semanticBinding: "unresolved";
  x: number;
  y: number;
  z: number;
  scale: number;
  anchor: { x: number; y: number };
}

export interface VisibleWorldRelease {
  schemaVersion: 3;
  releaseId: "visible-world-v1";
  releaseState: "development-evidence";
  bindingState: "mixed-resolved-and-unresolved";
  evidencePolicy: { runtimeAuthority: "presentation-only"; fixtureLabelRequired: true };
  runtimeDiagnostics: { fixture: true; unresolved: string[] };
  map: VisibleWorldAsset & { runtimeUse: "migration-fixture"; evidence: { note: string } };
  fieldMap: VisibleWorldAsset;
  village: {
    bindingState: "partial-scene-derived";
    completeness: "partial";
    tiles: ScenePiece[];
    foreground: ScenePiece[];
    decorations: ScenePiece[];
    buildings?: TownBuilding[];
  };
  actors: ActorBundle[];
}

interface VisibleWorldBootstrap {
  schemaVersion: 3;
  releaseId: "visible-world-v1";
  releasePath: typeof RELEASE_PATH;
  releaseBytes: number;
  releaseSha256: string;
}

export async function loadVerifiedVisibleWorldRelease(fetchFn: typeof fetch = fetch, onProgress?: (loaded: number, total: number) => void): Promise<VisibleWorldRelease> {
  const bootstrapResponse = await fetchFn(BOOTSTRAP_PATH, { cache: "no-cache", credentials: "same-origin" });
  if (!bootstrapResponse.ok) throw new Error(`Visible-world bootstrap returned ${bootstrapResponse.status}`);
  const bootstrap = parseBootstrap(await bootstrapResponse.json());

  const releaseResponse = await fetchFn(bootstrap.releasePath, { cache: "no-cache", credentials: "same-origin" });
  if (!releaseResponse.ok) throw new Error(`Visible-world release returned ${releaseResponse.status}`);
  const releaseBytes = new Uint8Array(await releaseResponse.arrayBuffer());
  await verifyBytes(RELEASE_PATH, releaseBytes, bootstrap.releaseBytes, bootstrap.releaseSha256);
  const release = validateVisibleWorldRelease(JSON.parse(new TextDecoder().decode(releaseBytes)) as unknown);

  const assets = collectAssets(release);
  const payloads = new Map<string, Uint8Array>();
  let loaded = 0;
  onProgress?.(0, assets.length);
  await Promise.all(assets.map(async (asset) => {
    const response = await fetchFn(asset.publicPath, { cache: "force-cache", credentials: "same-origin" });
    if (!response.ok) throw new Error(`Visible-world asset returned ${response.status}: ${asset.publicPath}`);
    const payload = new Uint8Array(await response.arrayBuffer());
    await verifyBytes(asset.publicPath, payload, asset.bytes, asset.sha256);
    payloads.set(asset.publicPath, payload);
    onProgress?.(++loaded, assets.length);
  }));
  validateAtomicActorBundles(release, payloads);
  return release;
}

export function validateVisibleWorldRelease(value: unknown): VisibleWorldRelease {
  if (!isRecord(value) || value.schemaVersion !== 3 || value.releaseId !== "visible-world-v1") throw new Error("Visible-world release identity or schema is invalid");
  if (value.releaseState !== "development-evidence" || value.bindingState !== "mixed-resolved-and-unresolved") throw new Error("Visible-world release is not evidence-safe");
  if (!isRecord(value.evidencePolicy) || value.evidencePolicy.runtimeAuthority !== "presentation-only" || value.evidencePolicy.fixtureLabelRequired !== true) throw new Error("Visible-world fixture policy is invalid");
  if (!isRecord(value.runtimeDiagnostics) || value.runtimeDiagnostics.fixture !== true || !isStringArray(value.runtimeDiagnostics.unresolved) || value.runtimeDiagnostics.unresolved.length === 0) throw new Error("Visible-world runtime diagnostics are missing");
  if (!isRecord(value.village) || value.village.bindingState !== "partial-scene-derived" || value.village.completeness !== "partial") throw new Error("Visible-world village evidence state is invalid");
  if (!Array.isArray(value.village.tiles) || !Array.isArray(value.village.foreground) || !Array.isArray(value.village.decorations)) throw new Error("Visible-world village scene arrays are invalid");
  if (value.village.buildings !== undefined && !Array.isArray(value.village.buildings)) throw new Error("Visible-world town building candidates are invalid");
  if (!Array.isArray(value.actors) || value.actors.map(actorFamily).join("|") !== EXPECTED_FAMILIES.join("|")) throw new Error("Visible-world actor family set is invalid");
  if (!isRecord(value.map) || value.map.runtimeUse !== "migration-fixture" || !isRecord(value.map.evidence) || typeof value.map.evidence.note !== "string") throw new Error("Visible-world field map fixture metadata is invalid");
  if (!isRecord(value.fieldMap) || value.fieldMap.publicPath !== value.map.publicPath || value.fieldMap.bytes !== value.map.bytes || value.fieldMap.sha256 !== value.map.sha256) throw new Error("Visible-world field map alias is mixed");
  for (const actor of value.actors) {
    if (!isRecord(actor) || actor.runtimeUse !== "migration-fixture" || !isRecord(actor.evidence) || typeof actor.evidence.note !== "string") throw new Error("Visible-world actor fixture metadata is invalid");
  }
  collectAssets(value).forEach(validateAsset);
  validateAtomicActorBundles(value as unknown as VisibleWorldRelease);
  return value as unknown as VisibleWorldRelease;
}

export async function verifyBytes(label: string, payload: Uint8Array, expectedBytes: number, expectedSha256: string): Promise<void> {
  if (payload.byteLength !== expectedBytes) throw new Error(`${label} byte length mismatch`);
  const actual = await sha256Hex(payload);
  if (actual !== expectedSha256) throw new Error(`${label} checksum mismatch`);
}

export function validateAtomicActorBundles(release: VisibleWorldRelease, payloads?: Map<string, Uint8Array>): void {
  for (const actor of release.actors) {
    const root = `${RELEASE_ROOT}actors/${actor.family}/`;
    if (actor.skeleton.publicPath !== `${root}${actor.family}.json`
      || actor.atlas.publicPath !== `${root}${actor.family}.atlas`
      || actor.texture.publicPath !== `${root}${actor.family}.png`) {
      throw new Error(`Visible-world actor bundle paths are not atomic: ${actor.family}`);
    }
    if (!payloads) continue;
    const atlas = payloads.get(actor.atlas.publicPath);
    const skeleton = payloads.get(actor.skeleton.publicPath);
    if (!atlas || !skeleton) throw new Error(`Visible-world actor bundle payload is incomplete: ${actor.family}`);
    if (!new TextDecoder().decode(atlas).split(/\r?\n/).includes(`${actor.family}.png`)) throw new Error(`Visible-world atlas page is mixed: ${actor.family}`);
    const skeletonJson = JSON.parse(new TextDecoder().decode(skeleton)) as unknown;
    if (!isRecord(skeletonJson) || !isRecord(skeletonJson.skeleton) || typeof skeletonJson.skeleton.spine !== "string") throw new Error(`Visible-world skeleton is invalid: ${actor.family}`);
  }
}

function parseBootstrap(value: unknown): VisibleWorldBootstrap {
  if (!isRecord(value) || value.schemaVersion !== 3 || value.releaseId !== "visible-world-v1" || value.releasePath !== RELEASE_PATH
    || !Number.isSafeInteger(value.releaseBytes) || (value.releaseBytes as number) <= 0
    || typeof value.releaseSha256 !== "string" || !SHA256_PATTERN.test(value.releaseSha256)) {
    throw new Error("Visible-world bootstrap is invalid");
  }
  return value as unknown as VisibleWorldBootstrap;
}

function collectAssets(value: unknown): VisibleWorldAsset[] {
  const assets = new Map<string, VisibleWorldAsset>();
  const inspect = (entry: unknown): void => {
    if (Array.isArray(entry)) { entry.forEach(inspect); return; }
    if (!isRecord(entry)) return;
    if (typeof entry.publicPath === "string" && typeof entry.bytes === "number" && typeof entry.sha256 === "string") {
      const asset = { publicPath: entry.publicPath, bytes: entry.bytes, sha256: entry.sha256 };
      const previous = assets.get(asset.publicPath);
      if (previous && (previous.bytes !== asset.bytes || previous.sha256 !== asset.sha256)) throw new Error(`Conflicting visible-world asset metadata: ${asset.publicPath}`);
      assets.set(asset.publicPath, asset);
    }
    Object.values(entry).forEach(inspect);
  };
  inspect(value);
  return [...assets.values()];
}

function validateAsset(asset: VisibleWorldAsset): void {
  if (!asset.publicPath.startsWith(RELEASE_ROOT) || asset.publicPath.includes("..") || !Number.isSafeInteger(asset.bytes) || asset.bytes <= 0 || !SHA256_PATTERN.test(asset.sha256)) throw new Error(`Visible-world asset metadata is invalid: ${asset.publicPath}`);
}

function actorFamily(value: unknown): string { return isRecord(value) && typeof value.family === "string" ? value.family : ""; }
function isRecord(value: unknown): value is Record<string, unknown> { return typeof value === "object" && value !== null; }
function isStringArray(value: unknown): value is string[] { return Array.isArray(value) && value.every((entry) => typeof entry === "string"); }

async function sha256Hex(payload: Uint8Array): Promise<string> {
  const stableBuffer = new ArrayBuffer(payload.byteLength);
  new Uint8Array(stableBuffer).set(payload);
  const digest = await crypto.subtle.digest("SHA-256", stableBuffer);
  return [...new Uint8Array(digest)].map((value) => value.toString(16).padStart(2, "0")).join("");
}
