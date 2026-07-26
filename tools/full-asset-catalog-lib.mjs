import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";

export const FULL_ASSET_RELEASE_ID = "evil-hunter-1.411-export-v1";
export const FULL_ASSET_CATALOG_PATH = `game-assets/manifests/releases/${FULL_ASSET_RELEASE_ID}.json`;
export const FULL_ASSET_BOOTSTRAP_PATH = "apps/web/public/full-assets/manifest.json";
export const FULL_ASSET_PUBLIC_CATALOG_PATH = `apps/web/public/full-assets/releases/${FULL_ASSET_RELEASE_ID}.json`;
export const EXPORTED_ROOT = "game-assets/extracted/exported";

export function sha256(payload) {
  return createHash("sha256").update(payload).digest("hex");
}

export async function readJson(file) {
  return JSON.parse(await fs.readFile(file, "utf8"));
}

export async function listRelativeFiles(root) {
  async function walk(directory) {
    const entries = await fs.readdir(directory, { withFileTypes: true });
    const files = [];
    for (const entry of entries) {
      if (entry.name.startsWith(".")) continue;
      const absolute = path.join(directory, entry.name);
      if (entry.isDirectory()) files.push(...await walk(absolute));
      else if (entry.isFile()) files.push(path.relative(root, absolute).split(path.sep).join("/"));
    }
    return files;
  }
  return (await walk(root)).sort();
}

export function compareAssetCoverage(indexedPaths, exportedPaths) {
  const indexed = new Set(indexedPaths);
  const exported = new Set(exportedPaths);
  return {
    missing: [...indexed].filter((assetPath) => !exported.has(assetPath)).sort(),
    unindexed: [...exported].filter((assetPath) => !indexed.has(assetPath)).sort()
  };
}

function atlasPages(text) {
  const pages = [];
  const lines = text.split(/\r?\n/);
  for (let index = 0; index < lines.length - 1; index += 1) {
    const candidate = lines[index].trim();
    if (candidate && /^size:\s*\d+,\s*\d+$/.test(lines[index + 1].trim())) pages.push(candidate);
  }
  return pages;
}

function exactNamedExport(files, logicalName, sourceExtension) {
  const matches = files.filter((asset) => {
    const parsed = path.posix.basename(asset.path).match(/^(.*)__\d+(\.[^.]+)$/);
    return parsed && parsed[1] === logicalName && parsed[2].toLowerCase() === sourceExtension.toLowerCase();
  });
  if (matches.length !== 1) {
    throw new Error(`Expected one export for ${logicalName}${sourceExtension}, found ${matches.length}`);
  }
  return matches[0];
}

export async function buildFullAssetCatalog(repoRoot) {
  const assetIndex = await readJson(path.join(repoRoot, "game-assets/asset-index.json"));
  const inventory = await readJson(path.join(repoRoot, "game-assets/manifests/full-source-inventory.json"));
  const exportedRoot = path.join(repoRoot, EXPORTED_ROOT);
  const textAssets = assetIndex.assets.filter((asset) => asset.path.startsWith("text/"));
  const textureAssets = assetIndex.assets.filter((asset) => asset.path.startsWith("textures/"));

  const spineFamilies = [];
  for (const name of inventory.spineSkeletonNames) {
    const skeleton = exactNamedExport(textAssets, `${name}.json`, ".bin");
    const atlas = exactNamedExport(textAssets, `${name}.atlas`, ".bin");
    const atlasText = await fs.readFile(path.join(exportedRoot, atlas.path), "utf8");
    const pageNames = atlasPages(atlasText);
    if (pageNames.length === 0) throw new Error(`Spine atlas has no page: ${atlas.path}`);
    const pages = pageNames.map((pageName) => {
      const extension = path.posix.extname(pageName);
      const logicalName = pageName.slice(0, -extension.length);
      return exactNamedExport(textureAssets, logicalName, extension);
    });
    spineFamilies.push({
      name,
      skeleton: skeleton.path,
      atlas: atlas.path,
      pages: pages.map((page) => page.path),
      state: "source-complete-unbound"
    });
  }

  return {
    schemaVersion: 1,
    releaseId: FULL_ASSET_RELEASE_ID,
    game: inventory.game,
    gameVersion: inventory.version,
    generatedFrom: {
      assetIndex: "game-assets/asset-index.json",
      sourceInventory: "game-assets/manifests/full-source-inventory.json"
    },
    runtime: {
      catalogPath: `/full-assets/releases/${FULL_ASSET_RELEASE_ID}.json`,
      assetBasePath: "/game-assets/",
      pathRule: "Append the catalog asset path to assetBasePath and URL-encode each path segment."
    },
    coverage: {
      exportedFiles: assetIndex.totalFiles,
      exportedBytes: assetIndex.totalBytes,
      counts: assetIndex.counts,
      spineFamilies: spineFamilies.length,
      unresolvedExtractionFailures: inventory.extractionFailures.length,
      recordedExtractionOutcomes: inventory.extractionOutcomes.length
    },
    behaviorBinding: {
      defaultState: "unbound-evidence",
      note: "Runtime addressability and checksum validation do not prove scene, UI, audio cue, animation-event, or gameplay behavior binding."
    },
    extractionFailures: inventory.extractionFailures,
    extractionOutcomes: inventory.extractionOutcomes,
    spineFamilies,
    assets: assetIndex.assets
  };
}

export function serializeJson(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}
