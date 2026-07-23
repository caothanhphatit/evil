import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";

const root = path.resolve("game-assets/extracted/exported");
const output = path.resolve("game-assets/asset-index.json");
const publicOutput = path.resolve("apps/web/public/asset-index.json");

async function walk(directory) {
  const entries = await fs.readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await walk(absolute)));
    else if (entry.isFile() && !entry.name.startsWith(".")) files.push(absolute);
  }
  return files;
}

const files = (await walk(root)).sort();
const assets = [];
for (const absolute of files) {
  const relative = path.relative(root, absolute).split(path.sep).join("/");
  const stat = await fs.stat(absolute);
  const digest = createHash("sha256").update(await fs.readFile(absolute)).digest("hex");
  assets.push({ path: relative, bytes: stat.size, sha256: digest });
}

const byType = Object.groupBy(assets, (asset) => asset.path.split("/")[0]);
const document = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  root: "game-assets/extracted/exported",
  totalFiles: assets.length,
  totalBytes: assets.reduce((sum, asset) => sum + asset.bytes, 0),
  counts: Object.fromEntries(Object.entries(byType).map(([key, value]) => [key, value.length])),
  assets
};

await fs.mkdir(path.dirname(output), { recursive: true });
await fs.writeFile(output, `${JSON.stringify(document, null, 2)}\n`);
await fs.mkdir(path.dirname(publicOutput), { recursive: true });
await fs.copyFile(output, publicOutput);
console.log(`Indexed ${document.totalFiles} files (${document.totalBytes} bytes).`);
