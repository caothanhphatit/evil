import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";

export const SELECTION_PATH = "game-assets/manifests/slice-001.selection.json";

export function sha256(data) {
  return createHash("sha256").update(data).digest("hex");
}

function assertSafeRelativePath(value, label) {
  if (!value || path.isAbsolute(value) || value.split(/[\\/]/).includes("..")) {
    throw new Error(`${label} must be a safe relative path: ${value}`);
  }
}

async function readJson(file) {
  return JSON.parse(await fs.readFile(file, "utf8"));
}

function pngMetadata(buffer) {
  const signature = "89504e470d0a1a0a";
  if (buffer.subarray(0, 8).toString("hex") !== signature || buffer.toString("ascii", 12, 16) !== "IHDR") {
    throw new Error("Invalid PNG payload");
  }
  return { format: "png", width: buffer.readUInt32BE(16), height: buffer.readUInt32BE(20) };
}

function wavMetadata(buffer) {
  if (buffer.toString("ascii", 0, 4) !== "RIFF" || buffer.toString("ascii", 8, 12) !== "WAVE") {
    throw new Error("Invalid WAV payload");
  }
  let offset = 12;
  let format;
  let dataBytes;
  while (offset + 8 <= buffer.length) {
    const id = buffer.toString("ascii", offset, offset + 4);
    const size = buffer.readUInt32LE(offset + 4);
    const start = offset + 8;
    if (id === "fmt " && size >= 16) {
      format = {
        channels: buffer.readUInt16LE(start + 2),
        sampleRate: buffer.readUInt32LE(start + 4),
        byteRate: buffer.readUInt32LE(start + 8),
        bitsPerSample: buffer.readUInt16LE(start + 14)
      };
    } else if (id === "data") {
      dataBytes = size;
    }
    offset = start + size + (size % 2);
  }
  if (!format || dataBytes === undefined || !format.byteRate) throw new Error("Incomplete WAV metadata");
  return {
    format: "wav",
    channels: format.channels,
    sampleRate: format.sampleRate,
    bitsPerSample: format.bitsPerSample,
    durationSeconds: Number((dataBytes / format.byteRate).toFixed(6))
  };
}

function atlasMetadata(text) {
  const pages = [];
  const lines = text.split(/\r?\n/);
  for (let index = 0; index < lines.length - 1; index += 1) {
    const candidate = lines[index].trim();
    if (candidate && /^size:\d+,\d+$/.test(lines[index + 1].trim())) pages.push(candidate);
  }
  if (pages.length === 0) throw new Error("Spine atlas has no texture page");
  return { format: "spine-atlas", pages };
}

function mediaMetadata(asset, payload) {
  if (asset.outputPath.endsWith(".png")) return pngMetadata(payload);
  if (asset.outputPath.endsWith(".wav")) return wavMetadata(payload);
  if (asset.outputPath.endsWith(".atlas")) return atlasMetadata(payload.toString("utf8"));
  if (asset.outputPath.endsWith(".json")) {
    const document = JSON.parse(payload.toString("utf8"));
    return {
      format: "spine-json",
      spineVersion: document.skeleton?.spine ?? null,
      animations: Object.keys(document.animations ?? {}).sort(),
      skins: (document.skins ?? []).map((skin) => skin.name).filter(Boolean).sort()
    };
  }
  throw new Error(`Unsupported output format: ${asset.outputPath}`);
}

function validateUnique(values, label) {
  const seen = new Set();
  for (const value of values) {
    if (seen.has(value)) throw new Error(`Duplicate ${label}: ${value}`);
    seen.add(value);
  }
}

export async function buildRelease(repoRoot) {
  const selectionFile = path.join(repoRoot, SELECTION_PATH);
  const selectionPayload = await fs.readFile(selectionFile);
  const selection = JSON.parse(selectionPayload.toString("utf8"));
  if (selection.schemaVersion !== 1) throw new Error(`Unsupported selection schema: ${selection.schemaVersion}`);
  assertSafeRelativePath(selection.releaseId, "releaseId");

  const sourceRoot = path.join(repoRoot, "game-assets/extracted/exported");
  const assetIndex = await readJson(path.join(repoRoot, selection.source.assetIndex));
  const inventory = await readJson(path.join(repoRoot, selection.source.inventory));
  const indexByPath = new Map(assetIndex.assets.map((asset) => [asset.path, asset]));
  const units = selection.contentUnits;
  const selectedAssets = units.flatMap((unit) => unit.assets.map((asset) => ({ ...asset, unitId: unit.id })));
  validateUnique(units.map((unit) => unit.id), "content unit ID");
  validateUnique(selectedAssets.map((asset) => asset.id), "asset ID");
  validateUnique(selectedAssets.map((asset) => asset.outputPath), "output path");

  for (const unit of units.filter((candidate) => candidate.evidence)) {
    const evidence = unit.evidence;
    const resolves = inventory.some((entry) =>
      entry.source === evidence.source && entry.path_id === evidence.pathId &&
      entry.type === evidence.type && entry.name === evidence.name
    );
    if (!resolves) throw new Error(`Unity evidence does not resolve for ${unit.id}`);
  }

  const assets = [];
  const payloads = new Map();
  for (const asset of selectedAssets) {
    assertSafeRelativePath(asset.sourcePath, "sourcePath");
    assertSafeRelativePath(asset.outputPath, "outputPath");
    const indexed = indexByPath.get(asset.sourcePath);
    if (!indexed) throw new Error(`Source is absent from asset index: ${asset.sourcePath}`);
    if (indexed.bytes !== asset.bytes || indexed.sha256 !== asset.sha256) {
      throw new Error(`Pinned index metadata differs for ${asset.sourcePath}`);
    }
    const unityObject = inventory.find((entry) =>
      entry.source === asset.unity.source && entry.path_id === asset.unity.pathId &&
      entry.type === asset.unity.type && entry.name === asset.unity.name
    );
    if (!unityObject) throw new Error(`Unity inventory reference does not resolve for ${asset.id}`);

    const payload = await fs.readFile(path.join(sourceRoot, asset.sourcePath));
    if (payload.length !== asset.bytes || sha256(payload) !== asset.sha256) {
      throw new Error(`Source checksum differs for ${asset.sourcePath}`);
    }
    payloads.set(asset.outputPath, payload);
    assets.push({
      id: asset.id,
      unitId: asset.unitId,
      sourcePath: asset.sourcePath,
      outputPath: asset.outputPath,
      publicPath: `/content/releases/${selection.releaseId}/${asset.outputPath}`,
      bytes: asset.bytes,
      sha256: asset.sha256,
      media: mediaMetadata(asset, payload),
      unity: asset.unity
    });
  }

  for (const unit of units.filter((candidate) => candidate.kind === "spine-skeleton")) {
    const unitAssets = assets.filter((asset) => asset.unitId === unit.id);
    const skeleton = unitAssets.find((asset) => asset.media.format === "spine-json");
    const atlas = unitAssets.find((asset) => asset.media.format === "spine-atlas");
    const outputNames = new Set(unitAssets.map((asset) => path.basename(asset.outputPath)));
    if (!skeleton || !atlas) throw new Error(`Incomplete Spine unit: ${unit.id}`);
    for (const animation of unit.requiredAnimations ?? []) {
      if (!skeleton.media.animations.includes(animation)) throw new Error(`${unit.id} is missing animation ${animation}`);
    }
    for (const skin of unit.requiredSkins ?? []) {
      if (!skeleton.media.skins.includes(skin)) throw new Error(`${unit.id} is missing skin ${skin}`);
    }
    for (const page of atlas.media.pages) {
      if (!outputNames.has(page)) throw new Error(`${unit.id} atlas page does not resolve: ${page}`);
    }
  }

  const contentUnits = units.map(({ assets: unitAssets, ...unit }) => ({
    ...unit,
    assetIds: unitAssets.map((asset) => asset.id)
  }));
  const manifest = {
    schemaVersion: 1,
    releaseId: selection.releaseId,
    source: selection.source,
    selectionSha256: sha256(selectionPayload),
    totalFiles: assets.length,
    totalBytes: assets.reduce((sum, asset) => sum + asset.bytes, 0),
    contentUnits,
    assets
  };
  return { selection, manifest, payloads };
}

export function serializeJson(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

export async function assertPublishedAsset(asset, releaseRoot) {
  const file = path.join(releaseRoot, asset.outputPath);
  const payload = await fs.readFile(file);
  if (payload.length !== asset.bytes) throw new Error(`Published size differs for ${asset.outputPath}`);
  if (sha256(payload) !== asset.sha256) throw new Error(`Published checksum differs for ${asset.outputPath}`);
}

export async function validatePublishedRelease(repoRoot) {
  const { manifest } = await buildRelease(repoRoot);
  const repositoryManifestFile = path.join(repoRoot, "game-assets/manifests/releases", `${manifest.releaseId}.json`);
  const publicRoot = path.join(repoRoot, "apps/web/public/content");
  const publicReleaseRoot = path.join(publicRoot, "releases", manifest.releaseId);
  const expected = serializeJson(manifest);
  if (await fs.readFile(repositoryManifestFile, "utf8") !== expected) throw new Error("Repository release manifest is stale");
  if (await fs.readFile(path.join(publicReleaseRoot, "manifest.json"), "utf8") !== expected) throw new Error("Public release manifest is stale");
  for (const asset of manifest.assets) await assertPublishedAsset(asset, publicReleaseRoot);

  const manifestDigest = sha256(Buffer.from(expected));
  const bootstrap = await readJson(path.join(publicRoot, "manifest.json"));
  if (bootstrap.currentRelease !== manifest.releaseId ||
      bootstrap.manifestPath !== `/content/releases/${manifest.releaseId}/manifest.json` ||
      bootstrap.manifestSha256 !== manifestDigest) {
    throw new Error("Bootstrap manifest does not reference the exact release manifest");
  }
  return manifest;
}
