import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

interface ExportAsset {
  path: string;
  bytes: number;
  sha256: string;
}

interface ExportManifest {
  assets: ExportAsset[];
}

const repositoryRoot = resolve(import.meta.dirname, "../../../..");
const stylesPath = resolve(repositoryRoot, "apps/web/src/styles.css");
const exportRoot = resolve(repositoryRoot, "game-assets/extracted/exported");
const manifestPath = resolve(
  repositoryRoot,
  "game-assets/manifests/releases/evil-hunter-1.411-export-v1.json",
);

describe("building popup assets", () => {
  it("keeps every CSS runtime asset backed by the checksum-pinned export", async () => {
    const [styles, manifestText] = await Promise.all([
      readFile(stylesPath, "utf8"),
      readFile(manifestPath, "utf8"),
    ]);
    const manifest = JSON.parse(manifestText) as ExportManifest;
    const indexedAssets = new Map(manifest.assets.map((asset) => [asset.path, asset]));
    const runtimePaths = [...styles.matchAll(/url\(['"]?(?:\/game-assets\/|\/content\/releases\/original-flow-v1\/)([^)'"?]+)['"]?\)/g)]
      .map((match) => decodeURIComponent(match[1]))
      .filter((path, index, paths) => paths.indexOf(path) === index);

    expect(runtimePaths.length).toBeGreaterThan(0);
    for (const runtimePath of runtimePaths) {
      const expected = indexedAssets.get(runtimePath);
      expect(expected, `${runtimePath} is absent from the export manifest`).toBeDefined();

      const payload = await readFile(resolve(exportRoot, runtimePath));
      expect(payload.byteLength, `${runtimePath} byte length`).toBe(expected?.bytes);
      expect(
        createHash("sha256").update(payload).digest("hex"),
        `${runtimePath} checksum`,
      ).toBe(expected?.sha256);
    }
  });
});
