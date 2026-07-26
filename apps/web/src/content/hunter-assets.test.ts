import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { loadHunterAssetCatalog, type HunterFileAsset } from "./hunter-assets";

const repositoryRoot = resolve(import.meta.dirname, "../../../..");
const packageCatalogPath = resolve(repositoryRoot, "packages/content/releases/evil-hunter-1.411/hunter-assets.json");
const publicCatalogPath = resolve(repositoryRoot, "apps/web/public/content/releases/evil-hunter-1.411/hunter-assets/catalog.json");

describe("Hunter asset catalog", () => {
  it("publishes the confirmed portrait and Spine visual inventories without semantic guesses", async () => {
    const payload = await readFile(packageCatalogPath, "utf8");
    const catalog = await loadHunterAssetCatalog(async () => new Response(payload, { status: 200 }));

    expect(catalog.portraits).toHaveLength(320);
    expect(catalog.portraits.filter((row) => row.family === "female")).toHaveLength(160);
    expect(catalog.portraits.filter((row) => row.family === "male")).toHaveLength(160);
    expect(catalog.visualCatalog.aggregateSkins).toHaveLength(11);
    expect(catalog.visualCatalog.weaponSkins).toHaveLength(1_059);
    expect(catalog.visualCatalog.animations).toHaveLength(70);
    expect(catalog.visualCatalog.animations.every((row) => row.gameplaySemantics === "unresolved")).toBe(true);
    expect(catalog.visualCatalog.aggregateSkins.every((row) => row.compositionBinding === "unresolved")).toBe(true);
    expect(catalog.visualCatalog.weaponSkins.every((row) => row.equipmentBinding === "unresolved")).toBe(true);
  });

  it("keeps the confirmed Hunter HUD, detail, skill, trait, and equipment families complete", async () => {
    const payload = await readFile(packageCatalogPath, "utf8");
    const catalog = await loadHunterAssetCatalog(async () => new Response(payload, { status: 200 }));

    expect(catalog.uiAssets.traits).toHaveLength(69);
    expect(catalog.uiAssets["equipment-hud"]).toHaveLength(20);
    expect(catalog.uiAssets.skills).toHaveLength(215);
    expect(catalog.uiAssets["hunter-hud"]).toHaveLength(28);
    expect(catalog.uiAssets["hunter-info-status"]).toHaveLength(12);
    expect(catalog.uiAssets["hunter-info-tabs"]).toHaveLength(4);
    expect(catalog.uiAssets["hunter-info-equipment"]).toHaveLength(24);
    expect(catalog.uiAssets["hunter-info-experience"]).toHaveLength(3);
    expect(catalog.uiAssets["hunter-info-growth"]).toHaveLength(24);
    expect(catalog.uiAssets["hunter-info-riding-pet"]).toHaveLength(8);
    expect(catalog.uiAssets["hunter-info-status"].map((row) => row.sourceName)).toEqual(expect.arrayContaining([
      "h_detail_ic_01",
      "h_detail_ic_09",
      "h_detail_stone_box",
    ]));
    expect(catalog.uiAssets["hunter-info-growth"].filter((row) => /^growth_ic_\d+$/.test(row.sourceName))).toHaveLength(15);
    expect(catalog.uiAssets["hunter-info-riding-pet"].map((row) => row.sourceName)).toContain("ride_pet_pasture");
    expect(Object.values(catalog.uiAssets).flat().every((row) => row.semanticBinding === "unresolved")).toBe(true);
  });

  it("packages every file with the indexed byte count and SHA-256", async () => {
    const payload = await readFile(packageCatalogPath, "utf8");
    const catalog = await loadHunterAssetCatalog(async () => new Response(payload, { status: 200 }));
    const files: HunterFileAsset[] = [catalog.spineBundle, catalog.portraits, ...Object.values(catalog.uiAssets)].flat();

    expect(files).toHaveLength(730);
    await Promise.all(files.map(async (asset) => {
      const bytes = await readFile(resolve(repositoryRoot, `apps/web/public${asset.publicPath}`));
      expect(bytes.byteLength).toBe(asset.bytes);
      expect(createHash("sha256").update(bytes).digest("hex")).toBe(asset.sha256);
      expect(asset.evidence.confidence).toBe("confirmed");
    }));
  });

  it("emits the same deterministic catalog to package and public release paths", async () => {
    const [packaged, published] = await Promise.all([readFile(packageCatalogPath), readFile(publicCatalogPath)]);
    expect(published.equals(packaged)).toBe(true);
  });
});
