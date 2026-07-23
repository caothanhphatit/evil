import { promises as fs } from "node:fs";
import path from "node:path";
import { buildRelease, serializeJson, sha256 } from "./content-pipeline-lib.mjs";

const repoRoot = path.resolve(import.meta.dirname, "..");
const { manifest, payloads } = await buildRelease(repoRoot);
const repositoryManifestDir = path.join(repoRoot, "game-assets/manifests/releases");
const publicRoot = path.join(repoRoot, "apps/web/public/content");
const releaseRoot = path.join(publicRoot, "releases", manifest.releaseId);

await fs.rm(releaseRoot, { recursive: true, force: true });
for (const [outputPath, payload] of payloads) {
  const output = path.join(releaseRoot, outputPath);
  await fs.mkdir(path.dirname(output), { recursive: true });
  await fs.writeFile(output, payload);
}

const serializedManifest = serializeJson(manifest);
await fs.mkdir(repositoryManifestDir, { recursive: true });
await fs.writeFile(path.join(repositoryManifestDir, `${manifest.releaseId}.json`), serializedManifest);
await fs.writeFile(path.join(releaseRoot, "manifest.json"), serializedManifest);
await fs.mkdir(publicRoot, { recursive: true });
await fs.writeFile(path.join(publicRoot, "manifest.json"), serializeJson({
  schemaVersion: 1,
  currentRelease: manifest.releaseId,
  manifestPath: `/content/releases/${manifest.releaseId}/manifest.json`,
  manifestSha256: sha256(Buffer.from(serializedManifest))
}));

console.log(`Packaged ${manifest.totalFiles} verified assets (${manifest.totalBytes} bytes) as ${manifest.releaseId}.`);
