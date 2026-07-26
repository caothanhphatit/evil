import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

const root = new URL("../", import.meta.url).pathname;
const releasePath = join(root, "apps/web/public/content/releases/visible-world-v1/release.json");
const bootstrapPath = join(root, "apps/web/public/content/releases/visible-world-v1/manifest.json");
const buildingPath = join(root, "reverse-engineering/evidence/building-runtime-assets-v1.json");
const contractPath = join(root, "reverse-engineering/evidence/building-ui-contract-v1.json");

const release = JSON.parse(await readFile(releasePath, "utf8"));
const contract = JSON.parse(await readFile(contractPath, "utf8"));
if (contract.contractType !== "building-ui-evidence" || contract.buildIdPolicy?.status !== "unresolved") {
  throw new Error("Building UI contract is missing its unresolved build-ID safety policy");
}
const confirmedAssetIds = new Set(contract.animationAssets
  .filter((asset) => asset.assetClass === "base-building")
  .map((asset) => asset.name));
const buildings = JSON.parse(await readFile(buildingPath, "utf8")).buildings
  .filter((building) => confirmedAssetIds.has(building.id))
  .filter((building) => /^build_(?:[1-9]|1\d|2[0-8])$/.test(building.id))
  .sort((left, right) => Number(left.id.slice(6)) - Number(right.id.slice(6)));
if (buildings.length !== 28) throw new Error(`Expected 28 source-confirmed core building assets, found ${buildings.length}`);
const slots = buildings.map((_, index) => [
  110 + (index % 7) * 130,
  190 + Math.floor(index / 7) * 210,
]);
release.runtimeDiagnostics.unresolved = [...new Set([...release.runtimeDiagnostics.unresolved, "town-building-placement"])];
release.village.buildings = buildings.map((building, index) => ({
  ...building,
  contractEvidence: "reverse-engineering/evidence/building-ui-contract-v1.json",
  semanticBinding: "unresolved",
  x: slots[index][0],
  y: slots[index][1],
  z: slots[index][1],
  scale: 0.78,
  anchor: { x: 0.5, y: 1 },
}));
const releaseBytes = Buffer.from(`${JSON.stringify(release, null, 2)}\n`);
await writeFile(releasePath, releaseBytes);
const bootstrap = JSON.parse(await readFile(bootstrapPath, "utf8"));
bootstrap.releaseBytes = releaseBytes.byteLength;
bootstrap.releaseSha256 = createHash("sha256").update(releaseBytes).digest("hex");
await writeFile(bootstrapPath, `${JSON.stringify(bootstrap, null, 2)}\n`);
console.log(`Packaged ${release.village.buildings.length} unnamed source-confirmed town candidates`);
