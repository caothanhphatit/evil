import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const repositoryRoot = resolve(import.meta.dirname, "../../../..");

describe("gear building UI integration", () => {
  it("uses one BuildingPop route instead of the legacy shop popup", async () => {
    const source = await readFile(resolve(repositoryRoot, "apps/web/src/app/building-renderer.ts"), "utf8");

    expect(source).not.toContain('id="shop-building-pop"');
    expect(source).not.toContain("renderShopBuildingPop");
    expect(source).toContain('classList.toggle("gear-route-ui"');
    expect(source).toContain('classList.toggle("blacksmith-ui"');
    expect(source).toContain('classList.toggle("display-shop-ui"');
    expect(source).toContain('classList.toggle("jeweler-ui"');
  });

  it("never substitutes a building thumbnail for an unresolved gear icon", async () => {
    const source = await readFile(resolve(repositoryRoot, "apps/web/src/app/building-renderer.ts"), "utf8");
    const gearRenderer = source.slice(
      source.indexOf("function appendGearArt"),
      source.indexOf("function renderTradingPostCatalog"),
    );

    expect(gearRenderer).toContain("if (recipe.icon) icon.src = recipe.icon");
    expect(gearRenderer).not.toContain("selectedBuildingVisual");
    expect(gearRenderer).not.toContain("village/buildings");
  });

  it("keeps the recovered grid, display badge and compact overlay styling", async () => {
    const [source, styles] = await Promise.all([
      Promise.all(["building-renderer.ts", "shell.ts", "world-controller.ts"].map((file) => readFile(resolve(repositoryRoot, `apps/web/src/app/${file}`), "utf8"))).then((parts) => parts.join("\n")),
      readFile(resolve(repositoryRoot, "apps/web/src/styles.css"), "utf8"),
    ]);

    expect(styles).toContain(".blacksmith-grid { position: relative; z-index: 1; flex: 1 1 auto; display: grid; grid-template-columns: repeat(3");
    expect(styles).toContain("grid-auto-rows: 82px");
    expect(styles).toContain("overflow-y: auto");
    expect(source).toContain('createGameDropdown(t("craft.gear_difficulty")');
    const gearCatalogRenderer = source.slice(
      source.indexOf("function renderGearCraftingCatalog"),
      source.indexOf("function renderDisplayShopCatalog"),
    );
    expect(gearCatalogRenderer).not.toContain('document.createElement("select")');
    expect(gearCatalogRenderer).not.toContain("Gear quality tier");
    expect(gearCatalogRenderer).not.toContain("qualityUnlocked");
    expect(gearCatalogRenderer).toContain('card.className = "gear-catalog-card"');
    expect(gearCatalogRenderer).toContain('action.textContent = t("common.create")');
    expect(gearCatalogRenderer).toContain('t("craft.quality.regular")');
    expect(gearCatalogRenderer).toContain('card.setAttribute("aria-label", t("craft.item_quality"');
    expect(styles).toContain("grid-auto-rows: 70px");
    expect(styles).toContain("max-height: 112px");
    expect(source).toContain('count.textContent = t("craft.item_count"');
    expect(source).toContain("function popupDataSignature");
    expect(styles).toContain(".on-display-badge");
    expect(styles).toContain(".gear-create-pop.gear-detail-mode");
    expect(styles).toContain(".shop-item-stats");
    expect(styles).toContain(".shop-buyer-gold");
    expect(styles).toContain(".shop-weapon-comparison");
    expect(styles).toContain(".shop-purchase-economy");
    expect(styles).toContain(".display-card-detail");
    expect(styles).toContain(".display-card-buy");
    expect(styles).toContain(".gear-create-pop[data-quality='4'] .gear-frame");
    expect(styles).toContain(".gear-catalog-card.display-card[data-quality='4'] .gear-item-art");
    expect(source).toContain('card.dataset.quality = String(displayed.quality)');
    expect(source).toContain("card.dataset.gearInstanceId = displayed.gear_instance_id");
    expect(styles).toContain(".gear-create-pop.gear-inspect-mode");
    expect(source).toContain("projectShopPurchase");
    expect(source).toContain('detail.addEventListener("click", () => openGearDetail(recipe, displayed))');
    expect(source).toContain('buy.addEventListener("click", () => openGearPurchase(recipe, displayed))');
    const openDetail = source.slice(source.indexOf("function openGearDetail"), source.indexOf("function appendGearArt"));
    expect(openDetail).not.toContain("buildingPanel.hidden");
    const detailRenderer = source.slice(source.indexOf("function renderGearCreatePop"), source.indexOf("function renderConsumCreatePop"));
    expect(detailRenderer).toContain('t("shop.stat.item_level")');
    expect(detailRenderer).toContain('buyerGold.className = "shop-buyer-gold"');
    expect(detailRenderer).toContain('comparison.className = "shop-weapon-comparison"');
    expect(detailRenderer).toContain('economy.className = "shop-purchase-economy"');
    expect(detailRenderer).toContain('context.gearPopupMode === "purchase"');
    expect(detailRenderer).not.toContain('context.purchaseHunterId = option.hunterId');
    expect(source).toContain('id="gear-quantity-minus"');
    expect(source).toContain('id="gear-create-quantity" type="number"');
    expect(source).toContain('data-gear-delta="1"');
    expect(source).toContain('data-gear-delta="1000"');
    expect(source).not.toContain("data-gear-max");
    expect(styles).toContain(".quantity-input-row input[type='number']");
    expect(source).toContain("gearMaterialIcons.get(cost.material_id)");
    expect(source).toContain('icon.className = "unresolved-material-icon"');
    expect(source).toContain('id="consum-create-icon-placeholder"');
    expect(styles).toContain(".product-icon-unresolved");
  });
});
