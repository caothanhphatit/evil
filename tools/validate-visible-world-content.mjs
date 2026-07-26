import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { listRelativeFiles } from "./full-asset-catalog-lib.mjs";
import {
  VISIBLE_WORLD_MANIFEST,
  VISIBLE_WORLD_RELEASE_ID,
  VISIBLE_WORLD_ROOT,
  collectVisibleWorldAssets,
  publicPathToReleasePath,
  validateVisibleWorldClaims
} from "./visible-world-content-lib.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const releaseRoot = path.join(repoRoot, VISIBLE_WORLD_ROOT);
const releasePayload = await fs.readFile(path.join(repoRoot, VISIBLE_WORLD_MANIFEST));
const manifestPayload = await fs.readFile(path.join(releaseRoot, "manifest.json"));
const bootstrap = JSON.parse(manifestPayload);
const releaseDigest = createHash("sha256").update(releasePayload).digest("hex");
if (bootstrap.schemaVersion !== 3 || bootstrap.releaseId !== VISIBLE_WORLD_RELEASE_ID
  || bootstrap.releasePath !== `/content/releases/${VISIBLE_WORLD_RELEASE_ID}/release.json`
  || bootstrap.releaseBytes !== releasePayload.length || bootstrap.releaseSha256 !== releaseDigest) {
  throw new Error("Visible-world bootstrap does not pin the published release");
}
const manifest = JSON.parse(releasePayload);
validateVisibleWorldClaims(manifest);

const assetIndex = JSON.parse(await fs.readFile(path.join(repoRoot, "game-assets/asset-index.json"), "utf8"));
const indexedByPath = new Map(assetIndex.assets.map((asset) => [asset.path, asset]));
const fullCatalog = JSON.parse(await fs.readFile(path.join(repoRoot, "game-assets/manifests/releases/evil-hunter-1.411-export-v1.json"), "utf8"));
const spineByName = new Map(fullCatalog.spineFamilies.map((family) => [family.name, family]));
const normalizedManifest = JSON.parse(await fs.readFile(path.join(repoRoot, "game-assets/normalized/village/manifest.json"), "utf8"));
if (normalizedManifest.stage !== "normalized-evidence" || normalizedManifest.transformation?.approximation !== true) {
  throw new Error("Normalized village evidence must declare its approximate transformation provenance");
}
const normalizedByPath = new Map([
  ...normalizedManifest.foreground,
  ...Object.values(normalizedManifest.npcs).flat()
].map((asset) => [asset.file, asset]));

const assets = collectVisibleWorldAssets(manifest);
if (assets.length !== 93) throw new Error(`Expected 93 unique visible-world assets, found ${assets.length}`);
if (manifest.village.buildings?.length !== 28) throw new Error("Expected 28 source-confirmed core town candidates");
for (const sourcePath of ["sprites/back_anim_a_01__1529.png", "sprites/back_anim_b_01__1546.png"]) {
  if (!manifest.village.tiles.some((piece) => piece.sourcePath === sourcePath && piece.z === 494)) {
    throw new Error(`Recovered town surround is missing: ${sourcePath}`);
  }
}
for (const asset of assets) {
  const relativeOutput = publicPathToReleasePath(asset.publicPath);
  const sourceSegments = asset.sourcePath.split(/[\\/]/);
  if (path.isAbsolute(asset.sourcePath) || sourceSegments.includes("..")) throw new Error(`Unsafe source path: ${asset.sourcePath}`);
  if (asset.sourceNamespace === "immutable-export") {
    if (asset.sourcePath.startsWith("game-assets/")) throw new Error(`Immutable source must be export-relative: ${asset.sourcePath}`);
    const indexed = indexedByPath.get(asset.sourcePath);
    if (!indexed || indexed.bytes !== asset.bytes || indexed.sha256 !== asset.sha256) {
      throw new Error(`Immutable source is not pinned by the asset index: ${asset.sourcePath}`);
    }
  } else if (asset.sourceNamespace === "normalized-evidence") {
    if (!asset.sourcePath.startsWith("game-assets/normalized/village/")) {
      throw new Error(`Normalized source is outside the village evidence namespace: ${asset.sourcePath}`);
    }
    const normalized = normalizedByPath.get(asset.sourcePath);
    if (!normalized || normalized.bytes !== asset.bytes || normalized.sha256 !== asset.sha256) {
      throw new Error(`Normalized source is not pinned by its provenance manifest: ${asset.sourcePath}`);
    }
  } else if (asset.sourceNamespace === "runtime-extracted") {
    if (!asset.sourcePath.startsWith("apps/web/public/content/releases/visible-world-v1/")) {
      throw new Error(`Runtime-extracted source is outside the visible-world publication: ${asset.sourcePath}`);
    }
  } else {
    throw new Error(`Unsupported visible-world source namespace: ${asset.sourceNamespace}`);
  }

  const source = asset.sourceNamespace === "immutable-export"
    ? path.join(repoRoot, "game-assets/extracted/exported", asset.sourcePath)
    : path.join(repoRoot, asset.sourcePath);
  for (const [label, file] of [["source", source], ["published", path.join(releaseRoot, relativeOutput)]]) {
    const payload = await fs.readFile(file);
    const digest = createHash("sha256").update(payload).digest("hex");
    if (payload.length !== asset.bytes || digest !== asset.sha256) throw new Error(`${label} checksum mismatch for ${asset.publicPath}`);
  }
}

const expectedPublished = new Set(["manifest.json", "release.json", ...assets.map((asset) => publicPathToReleasePath(asset.publicPath))]);
const actualPublished = await listRelativeFiles(releaseRoot);
const unexpectedPublished = actualPublished.filter((file) => !expectedPublished.has(file));
const missingPublished = [...expectedPublished].filter((file) => !actualPublished.includes(file));
if (unexpectedPublished.length || missingPublished.length) {
  throw new Error(`Visible-world publication coverage mismatch: missing=${JSON.stringify(missingPublished)}, unexpected=${JSON.stringify(unexpectedPublished)}`);
}

for (const actor of manifest.actors) {
  const family = spineByName.get(actor.family);
  if (!family || family.state !== "source-complete-unbound") throw new Error(`Spine catalog family is missing or incorrectly bound: ${actor.family}`);
  if (actor.skeleton.sourcePath !== family.skeleton || actor.atlas.sourcePath !== family.atlas) {
    throw new Error(`Spine skeleton/atlas source mismatch: ${actor.family}`);
  }
  if (family.pages.length !== 1 || actor.texture.sourcePath !== family.pages[0]) {
    throw new Error(`Visible-world currently requires one exact atlas page for ${actor.family}`);
  }
  const skeleton = JSON.parse(await fs.readFile(path.join(releaseRoot, publicPathToReleasePath(actor.skeleton.publicPath)), "utf8"));
  if (!skeleton.skeleton?.spine || !skeleton.animations || !skeleton.skins) throw new Error(`Invalid Spine skeleton payload: ${actor.family}`);
  const atlas = await fs.readFile(path.join(releaseRoot, publicPathToReleasePath(actor.atlas.publicPath)), "utf8");
  const page = path.posix.basename(actor.texture.publicPath);
  if (!atlas.split(/\r?\n/).includes(page)) throw new Error(`Spine atlas page does not resolve for ${actor.family}: ${page}`);
}

console.log(`Validated ${VISIBLE_WORLD_RELEASE_ID}: ${assets.length} assets, 7/7 atomic Spine fixtures, evidence-safe binding claims.`);
