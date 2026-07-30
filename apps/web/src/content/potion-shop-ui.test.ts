import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const repositoryRoot = resolve(import.meta.dirname, "../../../..");

describe("potion shop UI integration", () => {
  it("routes potion retail and crafting through separate reusable shop views", async () => {
    const source = await readFile(resolve(repositoryRoot, "apps/web/src/app/building-renderer.ts"), "utf8");

    expect(source).toContain('selectedBuildingId === POTION_SHOP_BUILDING_ID');
    expect(source).toContain('selectedBuildingId === ALCHEMIST_BUILDING_ID');
    expect(source).toContain("renderPotionCraftingCatalog(system.recipes, currentLevel)");
    expect(source).toContain('classList.toggle("potion-shop-ui"');
  });

  it("uses ConsumCreatePop for potion materials instead of GearCreatePop", async () => {
    const [source, styles] = await Promise.all([
      readFile(resolve(repositoryRoot, "apps/web/src/app/building-renderer.ts"), "utf8"),
      readFile(resolve(repositoryRoot, "apps/web/src/styles.css"), "utf8"),
    ]);

    const renderer = source.slice(source.indexOf("function renderPotionCraftingCatalog"), source.indexOf("function renderTradingPostCatalog"));
    expect(renderer).toContain("consumCreatePop.hidden = false");
    expect(renderer).not.toContain("openGearRecipe");
    expect(styles).toContain(".potion-recipe-grid");
    expect(styles).toContain("grid-template-columns: repeat(3, minmax(0, 1fr))");
    expect(source).toContain('className = "gear-catalog-card potion-catalog-card"');
    expect(styles).toContain(".consum-create-pop.potion-product-ui");
    expect(source).toContain('classList.toggle("service-product-ui", isServiceProduct)');
    expect(styles).toContain("ConsumCreatePop keeps the Alchemist route distinct from ProductCreatePop");
    expect(styles).toContain("height: min(540px, calc(100% - var(--bottom-menu-bottom) - var(--bottom-menu-reserved) - 12px))");
    expect(styles).toContain(".consum-create-pop.potion-product-ui .consum-material-grid { grid-template-columns: repeat(3, minmax(0, 1fr))");
    expect(styles).toContain(".consum-create-pop.potion-product-ui #consum-create-close");
    expect(styles).toContain(".consum-create-pop:not(.service-product-ui):not(.potion-product-ui)");
  });
});
