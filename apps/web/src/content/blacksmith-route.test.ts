import { describe, expect, it } from "vitest";
import { readFile, readdir } from "node:fs/promises";
import { resolve } from "node:path";
import {
  JEWELER_GEAR_TABS,
  GEAR_CREATE_LAYOUT,
  adaptBlacksmithRecipes,
  decodeGearCatalog,
  gearSpriteIsResolved,
  loadGearCatalog,
  pageGearRecipes,
  recipesForTab,
} from "./blacksmith-route";

describe("blacksmith route contract", () => {
  it("preserves the recovered GearCreatePop geometry and controls", () => {
    expect(GEAR_CREATE_LAYOUT).toMatchObject({ controller: "GearCreatePop", width: 450, height: 950 });
    expect(GEAR_CREATE_LAYOUT.grid).toEqual({ columns: 3, rows: 3 });
    expect(GEAR_CREATE_LAYOUT.tabs).toEqual(["weapon", "armor", "gloves", "boots"]);
    expect(JEWELER_GEAR_TABS).toEqual(["ring", "necklace", "belt"]);
  });

  it("packages decoded helmet evidence without adding an unproven visible tab", () => {
    const adapted = adaptBlacksmithRecipes(
      [{ productId: { value: "recipe:helmet:0:rating:0" } }],
      new Map([["gear:helmet:0", { title: "Helmet", description: "", materials: { ids: [], quantities: [] }, price: 10 }]]),
    );
    expect(adapted).toEqual([expect.objectContaining({ kind: "helmet", iconPath: null })]);
  });

  it("adapts all gear recipe kinds without leaking raw IDs", () => {
    const rows = [
      { productId: { value: "recipe:boots:42:rating:4" } },
      { productId: { value: "recipe:weapon:0:rating:0" } },
      { productId: { value: "recipe:service:5:rating:0" } },
    ];
    const gear = new Map([
      ["gear:boots:42", { title: "Fanged Shoes", description: "", materials: { ids: [1], quantities: [10] }, price: 200 }],
      ["gear:weapon:0", { title: "Junk Sword", description: "", materials: { ids: [1], quantities: [10] }, price: 200 }],
    ]);
    const adapted = adaptBlacksmithRecipes(rows, gear);
    expect(adapted).toHaveLength(2);
    expect(adapted.map((x) => x.kind)).toEqual(["boots", "weapon"]);
    expect(adapted.every((x) => !x.id.includes("material:"))).toBe(true);
    expect(adapted.every((x) => !gearSpriteIsResolved(x))).toBe(true);
  });

  it("uses the available panel height for nine cards per page", () => {
    const rows = Array.from({ length: 10 }, (_, i) => ({ kind: "armor" as const, id: String(i), gearId: String(i), rating: 0, title: String(i), description: "", materialIds: [], materialQuantities: [], price: 0, iconPath: null }));
    expect(recipesForTab(rows, "armor")).toHaveLength(10);
    expect(pageGearRecipes(rows, 0)).toHaveLength(9);
    expect(pageGearRecipes(rows, 1)).toHaveLength(1);
  });

  it("decodes registry rows into category and difficulty buckets", () => {
    const field = (value: unknown) => ({ state: "resolved", value });
    const items = [
      { itemId: field("gear:weapon:7"), displayName: field({ en: "Test Axe" }), directionalEconomy: { hunterPaysTownGoldByTier: field([100, 200, 300, 400, 500]) } },
      { itemId: field("material:21"), displayName: field({ en: "Copper Ore" }) },
    ];
    const products = [{
      productId: field("recipe:weapon:7:rating:2"),
      inputs: { rows: [{ itemId: field("material:21"), quantity: field(15) }] },
      outputs: { rows: [{ itemId: field("gear:weapon:7"), quantity: field(1) }] },
    }];

    expect(decodeGearCatalog(items, products)).toEqual([expect.objectContaining({
      id: "recipe:weapon:7:rating:2",
      kind: "weapon",
      index: 7,
      rating: 2,
      productName: "Test Axe",
      salePrice: 300,
      materialCosts: [{ materialId: "material:21", displayName: "Copper Ore", quantity: 15, iconPath: null }],
    })]);
  });

  it("loads every decoded forge recipe without mixing categories or tiers", async () => {
    const registryPath = resolve(import.meta.dirname, "../../../../packages/content/releases/evil-hunter-1.411/building-registry.json");
    const registry = JSON.parse(await readFile(registryPath, "utf8")) as {
      catalogs: { items: { rows: unknown[] }; products: { rows: unknown[] } };
    };
    const rows = decodeGearCatalog(registry.catalogs.items.rows, registry.catalogs.products.rows);

    expect(rows).toHaveLength(3_355);
    expect(rows.filter((row) => row.kind === "weapon")).toHaveLength(1_575);
    expect(rows.filter((row) => row.kind === "weapon" && row.rating === 0)).toHaveLength(315);
    for (const kind of ["armor", "gloves", "boots"] as const) {
      expect(rows.filter((row) => row.kind === kind)).toHaveLength(215);
      expect(rows.filter((row) => row.kind === kind && row.rating === 4)).toHaveLength(43);
    }
    expect(rows.filter((row) => row.kind === "helmet")).toHaveLength(535);
    expect(rows.filter((row) => row.kind === "ring")).toHaveLength(215);
    expect(rows.filter((row) => row.kind === "necklace")).toHaveLength(215);
    expect(rows.filter((row) => row.kind === "belt")).toHaveLength(170);
  });

  it("preserves the source Easy weapon order shown in the runtime capture", async () => {
    const catalogPath = resolve(import.meta.dirname, "../../../../packages/content/releases/evil-hunter-1.411/gear-catalog.json");
    const payload = await readFile(catalogPath, "utf8");
    const rows = await loadGearCatalog(async () => new Response(payload, { status: 200 }));
    const firstPageRows = rows
      .filter((row) => row.kind === "weapon" && row.difficultyGroup === 1 && row.rating === 0)
      .slice(0, 6);
    const firstPage = firstPageRows.map((row) => row.productName);

    expect(firstPage).toEqual([
      "Double-Bladed Axe",
      "Viking Sword",
      "Battle Axe",
      "Battle Maul",
      "War Hammer",
      "Morning Star",
    ]);
    expect(firstPageRows.every((row) => row.iconPath?.startsWith("/content/releases/evil-hunter-1.411/gear-icons/"))).toBe(true);
    const iconRoot = resolve(import.meta.dirname, "../../public/content/releases/evil-hunter-1.411/gear-icons");
    expect((await readdir(iconRoot)).filter((name) => name.endsWith(".png"))).toHaveLength(284);
  });

  it("loads the complete accessory catalog with source-bound images", async () => {
    const catalogPath = resolve(import.meta.dirname, "../../../../packages/content/releases/evil-hunter-1.411/gear-catalog.json");
    const payload = await readFile(catalogPath, "utf8");
    const rows = await loadGearCatalog(async () => new Response(payload, { status: 200 }));
    const accessories = rows.filter((row) => JEWELER_GEAR_TABS.some((kind) => kind === row.kind));

    expect(accessories).toHaveLength(600);
    expect(accessories.every((row) => row.iconPath?.startsWith("/content/releases/evil-hunter-1.411/gear-icons/"))).toBe(true);
    expect(accessories.some((row) => row.kind === "belt" && row.difficultyGroup === 0)).toBe(true);
  });

  it("binds forge materials to the complete source sprite sequence", async () => {
    const catalogPath = resolve(import.meta.dirname, "../../../../packages/content/releases/evil-hunter-1.411/gear-catalog.json");
    const payload = await readFile(catalogPath, "utf8");
    const rows = await loadGearCatalog(async () => new Response(payload, { status: 200 }));
    const costs = [...new Map(rows.flatMap((row) => row.materialCosts).map((cost) => [cost.materialId, cost])).values()];
    const resolved = costs.filter((cost) => cost.iconPath !== null);

    expect(costs).toHaveLength(182);
    expect(resolved).toHaveLength(182);
    await Promise.all(resolved.map(async (cost) => {
      const localPath = resolve(import.meta.dirname, `../../public${cost.iconPath}`);
      expect((await readFile(localPath)).byteLength).toBeGreaterThan(0);
    }));
    expect(resolved.find((cost) => cost.materialId === "material:1")?.iconPath).toBe(
      "/content/releases/evil-hunter-1.411/material-icons/material-1.png",
    );
    expect(resolved.find((cost) => cost.materialId === "material:139")?.iconPath).toBe(
      "/content/releases/evil-hunter-1.411/material-icons/material-139.png",
    );
  });
});
