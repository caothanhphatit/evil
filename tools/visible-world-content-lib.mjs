import path from "node:path";

export const VISIBLE_WORLD_RELEASE_ID = "visible-world-v1";
export const VISIBLE_WORLD_ROOT = `apps/web/public/content/releases/${VISIBLE_WORLD_RELEASE_ID}`;
export const VISIBLE_WORLD_MANIFEST = `${VISIBLE_WORLD_ROOT}/release.json`;

const FORBIDDEN_BINDING_STATES = new Set(["confirmed-scene-bindings", "migration-derived-visible-world"]);

function assertClaim(claim, label) {
  const resolutions = new Set(["resolved", "candidate", "unresolved"]);
  const confidences = new Set(["confirmed", "strongly-inferred", "tentative", "unknown"]);
  if (!claim || !resolutions.has(claim.resolution) || !confidences.has(claim.confidence)) {
    throw new Error(`${label} has an invalid evidence claim`);
  }
  if (claim.confidence === "confirmed" && claim.resolution !== "resolved") {
    throw new Error(`${label} cannot be confirmed unless it is resolved`);
  }
  if (claim.resolution === "unresolved" && claim.confidence !== "unknown") {
    throw new Error(`${label} must use unknown confidence while unresolved`);
  }
}

export function validateVisibleWorldClaims(manifest) {
  if (manifest.schemaVersion !== 3 || manifest.releaseId !== VISIBLE_WORLD_RELEASE_ID) {
    throw new Error("Visible-world release identity or schema is invalid");
  }
  if (manifest.releaseState !== "development-evidence" || manifest.bindingState !== "mixed-resolved-and-unresolved") {
    throw new Error("Visible-world release must remain explicitly evidence-only");
  }
  if (manifest.evidencePolicy?.runtimeAuthority !== "presentation-only" || manifest.evidencePolicy?.fixtureLabelRequired !== true) {
    throw new Error("Visible-world evidence policy must require presentation-only fixture labeling");
  }
  if (manifest.runtimeDiagnostics?.fixture !== true || !manifest.runtimeDiagnostics.unresolved?.length) {
    throw new Error("Visible-world runtime diagnostics must expose unresolved fixture metadata");
  }
  if (manifest.village?.bindingState !== "partial-scene-derived" || manifest.village?.completeness !== "partial") {
    throw new Error("Village content must remain explicitly partial");
  }
  if (manifest.map?.runtimeUse !== "migration-fixture") throw new Error("Field map must be labeled as a migration fixture");
  assertClaim(manifest.map?.evidence, "field map");
  if (manifest.map.evidence.resolution !== "candidate" || manifest.map.evidence.confidence !== "tentative") {
    throw new Error("Field map must remain a tentative candidate");
  }
  if (JSON.stringify(manifest.fieldMap) !== JSON.stringify(manifest.map)) throw new Error("fieldMap alias differs from map");

  const expectedFamilies = ["hunter", "Chief", "Npc", "npc_animal", "pet", "mon_goldblin", "mon_a_01_1"];
  if (JSON.stringify(manifest.actors?.map((actor) => actor.family)) !== JSON.stringify(expectedFamilies)) {
    throw new Error("Visible-world actor family set or order changed");
  }
  for (const actor of manifest.actors) {
    if (actor.runtimeUse !== "migration-fixture") throw new Error(`${actor.family} is not labeled as a migration fixture`);
    for (const claimName of ["sourceBundle", "runtimeRole", "skin", "spawn", "legacyGameplayIdentity"]) {
      assertClaim(actor.evidence?.[claimName], `${actor.family}.${claimName}`);
    }
    if (actor.evidence.sourceBundle.resolution !== "resolved" || actor.evidence.sourceBundle.confidence !== "confirmed") {
      throw new Error(`${actor.family} source bundle must be resolved and confirmed`);
    }
    for (const claimName of ["skin", "spawn", "legacyGameplayIdentity"]) {
      if (actor.evidence[claimName].resolution !== "unresolved") throw new Error(`${actor.family}.${claimName} must remain unresolved`);
    }
  }
  for (const family of ["mon_goldblin", "mon_a_01_1"]) {
    const actor = manifest.actors.find((candidate) => candidate.family === family);
    if (actor.evidence.runtimeRole.resolution !== "candidate" || actor.evidence.runtimeRole.confidence !== "tentative") {
      throw new Error(`${family} must remain a tentative actor candidate`);
    }
  }

  const forbidden = [];
  function inspect(value, location = "manifest") {
    if (typeof value === "string" && FORBIDDEN_BINDING_STATES.has(value)) forbidden.push(`${location}=${value}`);
    else if (Array.isArray(value)) value.forEach((entry, index) => inspect(entry, `${location}[${index}]`));
    else if (value && typeof value === "object") {
      for (const [key, entry] of Object.entries(value)) inspect(entry, `${location}.${key}`);
    }
  }
  inspect(manifest);
  if (forbidden.length) throw new Error(`Visible-world contains forbidden binding claims: ${forbidden.join(", ")}`);
}

export function collectVisibleWorldAssets(manifest) {
  const byPublicPath = new Map();
  function inspect(value) {
    if (Array.isArray(value)) {
      for (const entry of value) inspect(entry);
      return;
    }
    if (!value || typeof value !== "object") return;
    if (value.publicPath && value.sourcePath && value.sourceNamespace && value.bytes !== undefined && value.sha256) {
      const record = {
        sourceNamespace: value.sourceNamespace,
        sourcePath: value.sourcePath,
        publicPath: value.publicPath,
        bytes: value.bytes,
        sha256: value.sha256
      };
      const existing = byPublicPath.get(record.publicPath);
      if (existing && JSON.stringify(existing) !== JSON.stringify(record)) {
        throw new Error(`Conflicting metadata for ${record.publicPath}`);
      }
      byPublicPath.set(record.publicPath, record);
    }
    for (const entry of Object.values(value)) inspect(entry);
  }
  inspect(manifest);
  return [...byPublicPath.values()].sort((left, right) => left.publicPath.localeCompare(right.publicPath));
}

export function publicPathToReleasePath(publicPath) {
  const prefix = `/content/releases/${VISIBLE_WORLD_RELEASE_ID}/`;
  if (!publicPath.startsWith(prefix)) throw new Error(`Asset public path is outside the release: ${publicPath}`);
  const relative = publicPath.slice(prefix.length);
  if (!relative || path.posix.isAbsolute(relative) || relative.split("/").includes("..")) {
    throw new Error(`Unsafe release asset path: ${publicPath}`);
  }
  return relative;
}
