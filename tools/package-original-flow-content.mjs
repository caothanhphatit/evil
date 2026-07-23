import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const manifestPath = path.join(repoRoot, "game-assets/manifests/releases/original-flow-v1.json");
const sourceRoot = path.join(repoRoot, "game-assets/extracted/exported");
const outputRoot = path.join(repoRoot, "apps/web/public/content/releases/original-flow-v1");
const manifest = JSON.parse(await fs.readFile(manifestPath, "utf8"));

await fs.rm(outputRoot, { recursive: true, force: true });
for (const asset of manifest.assets) {
  const source = path.join(sourceRoot, asset.sourcePath);
  const payload = await fs.readFile(source);
  const digest = createHash("sha256").update(payload).digest("hex");
  if (payload.length !== asset.bytes || digest !== asset.sha256) {
    throw new Error(`Original-flow source checksum mismatch: ${asset.sourcePath}`);
  }
  const output = path.join(outputRoot, asset.sourcePath);
  await fs.mkdir(path.dirname(output), { recursive: true });
  await fs.writeFile(output, payload);
}

await fs.writeFile(path.join(outputRoot, "evidence-manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Packaged ${manifest.assets.length} original-flow evidence assets into apps/web/public/content.`);
