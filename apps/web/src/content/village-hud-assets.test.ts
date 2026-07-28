import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const repositoryRoot = resolve(import.meta.dirname, "../../../..");
const releasePath = resolve(repositoryRoot, "game-assets/manifests/releases/original-flow-v1.json");
const publicRoot = resolve(repositoryRoot, "apps/web/public/content/releases/original-flow-v1");

const hudAssets = [
  "sprites/top_mon_level_01__1480.png",
  "sprites/top_ic_01_gold_24__4677.png",
  "sprites/top_ic_02_gem_24__4214.png",
  "sprites/top_ic_03_element_24__1412.png",
  "sprites/top_ic_04_book_24__3078.png",
  "sprites/top_ic_book__3217.png",
  "sprites/top_ic_rank__5074.png",
  "sprites/top_ic_man__5368.png",
  "sprites/top_ic_quest__4944.png",
  "sprites/top_ic_setting__4198.png",
  "sprites/ic_target__7095.png",
] as const;

describe("village HUD evidence assets", () => {
  it("publishes every source HUD icon used by the web shell with pinned bytes", async () => {
    const release = JSON.parse(await readFile(releasePath, "utf8")) as {
      assets: Array<{ sourcePath: string; bytes: number; sha256: string }>;
    };
    const byPath = new Map(release.assets.map((asset) => [asset.sourcePath, asset]));

    await Promise.all(hudAssets.map(async (sourcePath) => {
      const asset = byPath.get(sourcePath);
      expect(asset, sourcePath).toBeDefined();
      const payload = await readFile(resolve(publicRoot, sourcePath));
      expect(payload.byteLength).toBe(asset?.bytes);
      expect(createHash("sha256").update(payload).digest("hex")).toBe(asset?.sha256);
    }));
  });
});
