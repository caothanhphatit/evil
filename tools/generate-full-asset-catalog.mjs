import { promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  buildFullAssetCatalog,
  FULL_ASSET_BOOTSTRAP_PATH,
  FULL_ASSET_CATALOG_PATH,
  FULL_ASSET_PUBLIC_CATALOG_PATH,
  serializeJson,
  sha256
} from "./full-asset-catalog-lib.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const catalog = await buildFullAssetCatalog(repoRoot);
const catalogPayload = serializeJson(catalog);
const catalogHash = sha256(catalogPayload);
const bootstrapPayload = serializeJson({
  schemaVersion: 1,
  currentRelease: catalog.releaseId,
  catalogPath: catalog.runtime.catalogPath,
  catalogBytes: Buffer.byteLength(catalogPayload),
  catalogSha256: catalogHash
});

for (const relative of [FULL_ASSET_CATALOG_PATH, FULL_ASSET_PUBLIC_CATALOG_PATH]) {
  const destination = path.join(repoRoot, relative);
  await fs.mkdir(path.dirname(destination), { recursive: true });
  await fs.writeFile(destination, catalogPayload);
}

const bootstrapDestination = path.join(repoRoot, FULL_ASSET_BOOTSTRAP_PATH);
await fs.mkdir(path.dirname(bootstrapDestination), { recursive: true });
await fs.writeFile(bootstrapDestination, bootstrapPayload);

console.log(
  `Generated ${catalog.releaseId}: ${catalog.coverage.exportedFiles} files, ` +
  `${catalog.coverage.spineFamilies} complete Spine families, ` +
  `${catalog.coverage.unresolvedExtractionFailures} unresolved extraction failures.`
);
