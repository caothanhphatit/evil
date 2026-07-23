import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  buildFullAssetCatalog,
  EXPORTED_ROOT,
  FULL_ASSET_BOOTSTRAP_PATH,
  FULL_ASSET_CATALOG_PATH,
  FULL_ASSET_PUBLIC_CATALOG_PATH,
  readJson,
  serializeJson,
  sha256
} from "./full-asset-catalog-lib.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const expected = await buildFullAssetCatalog(repoRoot);
const expectedPayload = serializeJson(expected);
const repositoryPayload = await fs.readFile(path.join(repoRoot, FULL_ASSET_CATALOG_PATH));
const publicPayload = await fs.readFile(path.join(repoRoot, FULL_ASSET_PUBLIC_CATALOG_PATH));
if (!repositoryPayload.equals(Buffer.from(expectedPayload))) throw new Error("Repository full-asset catalog is stale");
if (!publicPayload.equals(repositoryPayload)) throw new Error("Public full-asset catalog differs from repository release");

const bootstrap = await readJson(path.join(repoRoot, FULL_ASSET_BOOTSTRAP_PATH));
if (bootstrap.currentRelease !== expected.releaseId || bootstrap.catalogPath !== expected.runtime.catalogPath) {
  throw new Error("Full-asset bootstrap does not select the generated release");
}
if (bootstrap.catalogBytes !== repositoryPayload.length || bootstrap.catalogSha256 !== sha256(repositoryPayload)) {
  throw new Error("Full-asset bootstrap checksum or byte length is stale");
}

const requiredCounts = { audio: 116, fonts: 2, metadata: 3, sprites: 8980, text: 106, textures: 152 };
if (JSON.stringify(expected.coverage.counts) !== JSON.stringify(requiredCounts)) {
  throw new Error(`Unexpected exported class counts: ${JSON.stringify(expected.coverage.counts)}`);
}
if (expected.assets.length !== 9359 || expected.coverage.exportedBytes !== 190429626) {
  throw new Error("Full exported file or byte baseline changed without an inventory update");
}
if (expected.spineFamilies.length !== 53 || expected.spineFamilies.some((family) => family.pages.length === 0)) {
  throw new Error("All 53 Spine families must contain skeleton, atlas, and texture pages");
}
if (expected.extractionFailures.length !== 0) throw new Error("No extraction failures should remain unresolved");
if (expected.extractionOutcomes.length !== 4) throw new Error("All four original extraction errors must retain an explicit outcome");
if (expected.extractionOutcomes.filter((outcome) => outcome.state === "recovered").length !== 2) {
  throw new Error("Both embedded fonts must be recorded as recovered");
}
if (expected.extractionOutcomes.filter((outcome) => outcome.state === "excluded-with-reason").length !== 2) {
  throw new Error("Both empty Font Texture placeholders must be excluded with a reason");
}

const exportedRoot = path.join(repoRoot, EXPORTED_ROOT);
for (const asset of expected.assets) {
  const payload = await fs.readFile(path.join(exportedRoot, asset.path));
  if (payload.length !== asset.bytes) throw new Error(`Byte length mismatch: ${asset.path}`);
  const digest = createHash("sha256").update(payload).digest("hex");
  if (digest !== asset.sha256) throw new Error(`Checksum mismatch: ${asset.path}`);
}

console.log(
  `Validated ${expected.releaseId}: 9,359 checksums, audio=116, fonts=2, text=106, textures=152, ` +
  `sprites=8,980, Spine=53/53, unresolved extraction failures=0.`
);
