import type { TownBuilding } from "../assets/visible-world-release";
import type { BuildingSystemSnapshot, MaterialStockSnapshot, OriginalFlowSnapshot, ShopRecipeSnapshot } from "../generated/protocol";
import type { WorldClient } from "../net/world-client";
import type { VisibleEntityWorld } from "../game/visible-world";
import { findBuildingInstanceById } from "../game/building-placement";
import { canSubmitGearEnhancement, GEAR_ENHANCEMENT_MODES, projectGearEnhancement, type GearEnhancementMode, type GearEnhancementView } from "../ui/gear-enhancement";
import { clampQuantity, missingCraftMaterial, remainingSharedCapacity, resolveServiceMaterialId, serviceMaterialRequired, townMaterialQuantity } from "../ui/shop-crafting";
import { formatLevelCosts, listBuildingEvidence, projectBuildingEvidence } from "../content/building-evidence";
import type { EvidenceBuildingRegistry } from "../content/building-registry";
import { originalUiLabel } from "../content/original-ui-labels";
import { BOUNTY_HUT_ROUTE, BOUNTY_TIERS } from "../routes/bounty-hut";
import { TRADING_POST_ROUTE, tradingPostDifficultyOptions, tradingPostStocksForDifficulty } from "../routes/trading-post";
import { projectProductService, productServiceRoute, type ProductServiceInput } from "../content/product-service-routes";
import { ACCESSORY_SHOP_BUILDING_IDS, ALL_GEAR_KINDS, ARMOR_SHOP_BUILDING_IDS, BLACKSMITH_BUILDING_IDS, BLACKSMITH_GEAR_TABS, ENHANCEMENT_FORGE_BUILDING_IDS, JEWELER_BUILDING_IDS, JEWELER_GEAR_TABS, WEAPON_SHOP_BUILDING_IDS, type GearCatalogRecipe, type GearKind } from "../content/blacksmith-route";
import { ALCHEMIST_BUILDING_ID, POTION_SHOP_BUILDING_ID, isPotionBuilding } from "../content/potion-shop-routes";
import { formatNumber, t, type MessageKey } from "../i18n";
import { projectShopPurchase } from "../ui/shop-purchase";
import { originalAsset } from "./shell";

export interface BuildingRenderingContext {
  client: WorldClient;
  debugUi: boolean;
  showPanelMessage(title: string, detail: string): void;
  renderTradingRequestEditor(): void;
  latestSnapshot: OriginalFlowSnapshot | null;
  selectedBuildingId: string | null;
  selectedBuildingInstanceId: string | null;
  selectedBuildingVisual: TownBuilding | null;
  buildingPanelMode: "building" | "construct";
  selectedRecipe: ShopRecipeSnapshot | null;
  selectedServiceMaterialId: string | null;
  selectedServiceQuantity: number;
  serviceTabsByBuilding: Map<string, "production" | "hunters">;
  gearTab: GearKind;
  blacksmithDifficultyGroup: number;
  blacksmithCraftableOnly: boolean;
  gearCatalog: GearCatalogRecipe[];
  gearMaterialIcons: Map<string, string>;
  selectedEnhancementGearKey: string | null;
  selectedEnhancementMode: GearEnhancementMode;
  enhancementView: GearEnhancementView;
  enhancementHunterId: number | null;
  purchaseHunterId: number | null;
  selectedEnhancementOptionalMaterialIds: string[];
  gearPopupMode: "craft" | "detail";
  selectedBountyTier: number;
  selectedTradingPostDifficulty: number;
  selectedTradingRequest: MaterialStockSnapshot | null;
  selectedTradingRequestQuantity: number;
  tradingRequestPending: boolean;
  buildingEvidenceRegistry: EvidenceBuildingRegistry | null;
  buildingEvidenceError: string | null;
  popupInteractionActive: boolean;
  pendingCraft: { popup: "gear" | "consumable"; recipeId: string } | null;
  pendingPurchase: { shopId: string; productId: string } | null;
  buildingPanel: HTMLElement;
  buildingName: HTMLElement;
  buildingPreview: HTMLImageElement;
  buildingLevel: HTMLElement;
  buildingFeature: HTMLElement;
  buildingCondition: HTMLElement;
  buildingLevelContract: HTMLElement;
  buildingCatalog: HTMLElement;
  buildingConstruct: HTMLButtonElement;
  buildingUpgrade: HTMLButtonElement;
  buildingUse: HTMLButtonElement;
  tradingRequestPop: HTMLElement;
  tradingRequestContent: HTMLElement;
  bountyPop: HTMLElement;
  bountyTitle: HTMLElement;
  bountyTierTabs: HTMLElement;
  bountyUpgrade: HTMLButtonElement;
  gearCreatePop: HTMLElement;
  gearCreateTitle: HTMLElement;
  gearCreateIcon: HTMLImageElement;
  gearCreateName: HTMLElement;
  gearCreatePrice: HTMLElement;
  gearCreateDescription: HTMLElement;
  gearLock: HTMLButtonElement;
  gearMaterialTitle: HTMLElement;
  gearMaterialCosts: HTMLElement;
  gearQuantityRow: HTMLElement;
  gearCreateQuantity: HTMLInputElement;
  gearFrameQuantity: HTMLOutputElement;
  gearCreateSubmit: HTMLButtonElement;
  gearCreateSell: HTMLButtonElement;
  consumCreatePop: HTMLElement;
  consumCreateTitle: HTMLElement;
  consumCreateIcon: HTMLImageElement;
  consumCreateIconPlaceholder: HTMLElement;
  consumCreateQuantity: HTMLOutputElement;
  consumCreateQuantityInput: HTMLInputElement;
  consumConversion: HTMLElement;
  consumMaterialTitle: HTMLElement;
  consumMaterialGrid: HTMLElement;
  consumCreateSubmit: HTMLButtonElement;
}

export function createBuildingRenderer(context: BuildingRenderingContext) {
  function popupDataSignature(snapshot: OriginalFlowSnapshot): string {
    if (context.buildingPanel.hidden && context.gearCreatePop.hidden && context.consumCreatePop.hidden && context.bountyPop.hidden) return "closed";
    const system = snapshot.village.building_system;
    return JSON.stringify([
      context.selectedBuildingId,
      context.selectedBuildingInstanceId,
      context.buildingPanelMode,
      system.states,
      system.instances,
      system.recipes,
      system.material_stocks,
      snapshot.hunter_roster.active_hunters.map((hunter) => ({ id: hunter.hunter_id, task: hunter.gear_enhancement_task })),
      snapshot.hunter_roster.product_services,
    ]);
  }
  
  function renderBuildingSystem(snapshot: OriginalFlowSnapshot | null): void {
    const system = snapshot?.village.building_system;
    if (!system) return;
    const evidenceBuildings = listBuildingEvidence(context.buildingEvidenceRegistry);
    if (context.buildingPanelMode === "construct" && (!context.selectedBuildingId || !evidenceBuildings.some((item) => item.id === context.selectedBuildingId))) {
      context.selectedBuildingId = evidenceBuildings[0]?.id ?? null;
      context.selectedBuildingInstanceId = null;
      context.selectedBuildingVisual = null;
    }
    if (context.buildingPanelMode === "construct") {
      context.buildingCatalog.hidden = false;
      context.buildingCatalog.replaceChildren(...evidenceBuildings.map((evidence) => {
        const state = system.states.find((item) => item.id === evidence.id);
        const button = document.createElement("button");
        button.type = "button";
        button.className = evidence.id === context.selectedBuildingId ? "selected" : "";
        button.textContent = `${evidence.name} ${state?.constructed ? t("common.level_short", { level: state.level }) : `Lv.1-${evidence.maxLevel ?? "?"}`}`;
        button.addEventListener("click", () => {
          context.selectedBuildingId = evidence.id;
          context.selectedBuildingInstanceId = null;
          context.selectedBuildingVisual = null;
          context.buildingPanelMode = "construct";
          renderBuildingSystem(context.latestSnapshot);
        });
        return button;
      }));
    } else {
      // A building tap opens that building's detail popup, never the construction catalog.
      context.buildingCatalog.hidden = true;
      context.buildingCatalog.replaceChildren();
    }
    context.buildingPanel.classList.toggle("construct-mode", context.buildingPanelMode === "construct");
    context.buildingPanel.classList.toggle("detail-mode", context.buildingPanelMode === "building");
    const state = system.states.find((item) => item.id === context.selectedBuildingId);
    const selectedInstance = findBuildingInstanceById(system.instances, context.selectedBuildingInstanceId);
    if (context.buildingPanelMode === "building" && context.selectedBuildingInstanceId && !selectedInstance) {
      context.selectedBuildingId = null;
      context.selectedBuildingInstanceId = null;
      context.selectedBuildingVisual = null;
      context.buildingPanel.hidden = true;
      return;
    }
    const evidence = context.selectedBuildingId ? projectBuildingEvidence(context.buildingEvidenceRegistry, context.selectedBuildingId) : null;
    const isBlacksmithRoute = context.buildingPanelMode === "building" && BLACKSMITH_BUILDING_IDS.includes(context.selectedBuildingId as typeof BLACKSMITH_BUILDING_IDS[number]);
    const isEnhancementForgeRoute = context.buildingPanelMode === "building" && ENHANCEMENT_FORGE_BUILDING_IDS.includes(context.selectedBuildingId as typeof ENHANCEMENT_FORGE_BUILDING_IDS[number]);
    const isJewelerRoute = context.buildingPanelMode === "building" && JEWELER_BUILDING_IDS.includes(context.selectedBuildingId as typeof JEWELER_BUILDING_IDS[number]);
    const isCraftingGearRoute = isBlacksmithRoute || isJewelerRoute;
    const isDisplayShopRoute = context.buildingPanelMode === "building" && (
      WEAPON_SHOP_BUILDING_IDS.includes(context.selectedBuildingId as typeof WEAPON_SHOP_BUILDING_IDS[number])
      || ARMOR_SHOP_BUILDING_IDS.includes(context.selectedBuildingId as typeof ARMOR_SHOP_BUILDING_IDS[number])
      || ACCESSORY_SHOP_BUILDING_IDS.includes(context.selectedBuildingId as typeof ACCESSORY_SHOP_BUILDING_IDS[number])
      || context.selectedBuildingId === POTION_SHOP_BUILDING_ID
    );
    const isPotionCraftingRoute = context.buildingPanelMode === "building" && context.selectedBuildingId === ALCHEMIST_BUILDING_ID;
    const isCatalogShopRoute = isCraftingGearRoute || isDisplayShopRoute || isPotionCraftingRoute;
    context.buildingPanel.classList.toggle("service-mode", context.buildingPanelMode === "building" && evidence?.popupRoute === "service");
    context.buildingPanel.classList.toggle("service-building-ui", context.buildingPanelMode === "building" && productServiceRoute(context.selectedBuildingId ?? "") !== null);
    context.buildingPanel.classList.toggle("trading-post-ui", context.buildingPanelMode === "building" && context.selectedBuildingId === TRADING_POST_ROUTE.buildingId);
    if (!(context.buildingPanelMode === "building" && context.selectedBuildingId === TRADING_POST_ROUTE.buildingId)) {
      context.tradingRequestPop.hidden = true;
      context.selectedTradingRequest = null;
      context.tradingRequestPending = false;
    }
    context.buildingPanel.classList.toggle("gear-route-ui", isCatalogShopRoute);
    context.buildingPanel.classList.toggle("blacksmith-ui", isCraftingGearRoute);
    context.buildingPanel.classList.toggle("enhancement-forge-ui", isEnhancementForgeRoute);
    context.buildingPanel.classList.toggle("jeweler-ui", isJewelerRoute);
    context.buildingPanel.classList.toggle("display-shop-ui", isDisplayShopRoute);
    context.buildingPanel.classList.toggle("potion-shop-ui", isPotionBuilding(context.selectedBuildingId));
    context.buildingPanel.classList.toggle("potion-crafting-ui", isPotionCraftingRoute);
    if (!evidence) {
      context.buildingName.textContent = context.selectedBuildingId ?? t("building.evidence_unavailable");
      context.buildingLevel.textContent = context.buildingEvidenceError ?? t("building.loading_evidence");
      context.buildingFeature.textContent = t("building.no_fabricated_data");
      context.buildingCondition.textContent = t("building.actions_disabled");
      const previewPath = context.selectedBuildingVisual?.publicPath ?? "";
      if (previewPath) context.buildingPreview.src = previewPath;
      else context.buildingPreview.removeAttribute("src");
      context.buildingPreview.hidden = !previewPath;
      context.buildingConstruct.disabled = true;
      context.buildingUpgrade.disabled = true;
      context.buildingUse.disabled = true;
      context.buildingLevelContract.replaceChildren();
      return;
    }
    context.buildingName.textContent = evidence.name;
    const spriteId = evidence.spriteAssetId;
    const previewPath = context.selectedBuildingVisual?.publicPath ?? (spriteId ? `/content/releases/visible-world-v1/village/buildings/${spriteId}.png` : "");
    if (previewPath) context.buildingPreview.src = previewPath;
    else context.buildingPreview.removeAttribute("src");
    context.buildingPreview.hidden = !previewPath;
    const currentLevel = selectedInstance?.level ?? (state?.constructed ? state.level : 0);
    if (context.buildingPanelMode === "building") context.buildingName.textContent = `${t("common.level_short", { level: currentLevel })} ${evidence.name}`;
    const targetLevel = Math.min(currentLevel + 1, evidence.maxLevel ?? currentLevel + 1);
    context.buildingLevel.textContent = state?.constructed
      ? t("building.level_summary", { current: currentLevel, maximum: evidence.maxLevel ?? "?", cost: formatLevelCosts(evidence, targetLevel) })
      : t("building.not_constructed", { cost: formatLevelCosts(evidence, 1) });
    const rawSourceSummary = [
      evidence.maxBuild === null ? null : t("building.max_build", { count: evidence.maxBuild }),
      evidence.gridSize === null ? null : t("building.grid_size", { width: evidence.gridSize[0], height: evidence.gridSize[1] }),
    ].filter((entry): entry is string => entry !== null).join(" · ");
    const sourceDescriptions: Partial<Record<string, MessageKey>> = {
      build_3: "building.build_3.description",
      build_7: "building.build_7.description",
      build_8: "building.build_8.description",
      build_9: "building.build_9.description",
      build_10: "building.build_10.description",
      build_11: "building.build_11.description",
      build_20: "building.build_20.description",
      build_21: "building.build_21.description",
      build_12: "building.build_12.description",
      build_13: "building.build_13.description",
      build_14: "building.build_14.description",
      build_19: "building.build_19.description",
    };
    const descriptionKey = sourceDescriptions[evidence.id];
    const featureDescription = descriptionKey ? t(descriptionKey) : evidence.description;
    context.buildingFeature.textContent = context.buildingPanelMode === "building" ? featureDescription
      : rawSourceSummary ? `${featureDescription} · ${rawSourceSummary}` : featureDescription;
    const targetRequirement = evidence.levels.find((entry) => entry.level === targetLevel)?.requiredTownHallLevel ?? null;
    const localizedRequirement = targetRequirement === null
      ? t("building.town_hall_unresolved")
      : originalUiLabel("buildpop_9", "vi", [targetRequirement]);
    context.buildingCondition.textContent = selectedInstance?.condition?.startsWith("building_prerequisite_required:")
      ? localizedRequirement
      : selectedInstance?.condition ?? localizedRequirement;
    context.buildingLevelContract.replaceChildren(...evidence.levels.map((entry) => {
      const row = document.createElement("div");
      const title = document.createElement("strong");
      title.textContent = t("common.level_short", { level: entry.level });
      const requirement = document.createElement("span");
      requirement.textContent = entry.requiredTownHallLevel === null
        ? t("building.town_hall_unresolved")
        : originalUiLabel("buildpop_9", "vi", [entry.requiredTownHallLevel]);
      const costs = document.createElement("small");
      costs.textContent = t("building.required_resources", { resources: entry.costs.length ? entry.costs.join(" · ") : "--" });
      row.append(title, requirement, costs);
      return row;
    }));
    if (context.buildingPanelMode === "building" && evidence.popupRoute === "gear-enhancement") {
      context.buildingLevelContract.hidden = true;
      context.buildingCatalog.hidden = false;
      renderEnhancementForge(system);
    } else if (context.buildingPanelMode === "building" && evidence.id === TRADING_POST_ROUTE.buildingId) {
      context.buildingLevelContract.hidden = true;
      context.buildingCatalog.hidden = false;
      const currentRequest = context.selectedTradingRequest
        ? system.material_stocks.find((stock) => stock.id === context.selectedTradingRequest?.id) ?? null
        : null;
      if (context.selectedTradingRequest && !currentRequest) {
        context.selectedTradingRequest = null;
        context.tradingRequestPending = false;
      } else if (currentRequest) {
        context.selectedTradingRequest = currentRequest;
      }
      renderTradingPostCatalog(system.material_stocks, selectedInstance?.level ?? 1, targetRequirement);
      if (context.selectedTradingRequest) context.renderTradingRequestEditor();
      else context.tradingRequestPop.hidden = true;
    } else if (context.buildingPanelMode === "building" && evidence.popupRoute === "production" && isCatalogShopRoute) {
      context.buildingLevelContract.hidden = true;
      context.buildingCatalog.hidden = false;
      if (isCraftingGearRoute) {
        renderGearCraftingCatalog(system.recipes, evidence.id);
      } else if (isPotionCraftingRoute) {
        renderPotionCraftingCatalog(system.recipes, currentLevel);
      } else {
        renderDisplayShopCatalog(system.recipes, context.selectedBuildingId);
      }
    } else if (context.buildingPanelMode === "building" && evidence.popupRoute === "production") {
      renderBuildingContractError(t("building.production_contract_missing"));
      return;
    } else if (context.buildingPanelMode === "building" && evidence.popupRoute === "service" && productServiceRoute(context.selectedBuildingId ?? "") !== null) {
      if (!context.selectedBuildingId) return;
      context.buildingLevelContract.hidden = true;
      context.buildingCatalog.hidden = false;
      const currentBuildingLevel = selectedInstance?.level ?? 1;
      const route = productServiceRoute(context.selectedBuildingId);
      if (!route) {
        renderBuildingContractError(t("building.service_contract_missing"));
        return;
      }
      const routeCandidates = system.recipes.filter((item) => item.shop_id === context.selectedBuildingId);
      // Project through the decoded route contract before rendering. This keeps
      // service popups isolated to their seven recovered product IDs.
      const routeProjection = projectProductService(context.selectedBuildingId, routeCandidates.map((recipe): ProductServiceInput => ({
        productId: recipe.id,
        productName: recipe.product_name,
        requiredLevel: recipe.required_level,
        effectValue: recipe.effect_value,
        serviceTimeMs: recipe.duration_ms,
        useMoney: recipe.sale_price,
        stock: recipe.stock,
        capacity: recipe.capacity,
        materialCosts: recipe.material_costs.map((cost) => ({ materialId: cost.material_id, displayName: cost.display_name, quantity: cost.quantity, outputQuantity: cost.output_quantity })),
      })), routeCandidates[0]?.capacity ?? 0);
      if (!routeProjection) {
        renderBuildingContractError(t("building.contract_error"));
        return;
      }
      const allowedProductIds = new Set(routeProjection.products.map((product) => product.productId));
      const allRecipes = routeCandidates.filter((item) => allowedProductIds.has(item.id));
      const recipes = allRecipes.filter((item) => item.required_level < currentBuildingLevel);
      const totalStock = recipes.reduce((total, recipe) => total + recipe.stock, 0);
      const tabs = document.createElement("div");
      tabs.className = "service-tabs";
      const activeServiceTab = context.serviceTabsByBuilding.get(route.buildingId) ?? "production";
      const authoritativeService = snapshot.hunter_roster.product_services.find((service) => service.building_id === route.buildingId) ?? null;
      for (const tab of ["production", "hunters"] as const) {
        const button = document.createElement("button");
        button.type = "button";
        button.className = tab === activeServiceTab ? "selected" : "";
        button.textContent = tab === "production" ? t("service.production") : t("service.hunters");
        button.addEventListener("click", () => {
          context.serviceTabsByBuilding.set(route.buildingId, tab);
          renderBuildingSystem(context.latestSnapshot);
        });
        tabs.append(button);
      }
      const capacityLabel = document.createElement("strong");
      capacityLabel.textContent = activeServiceTab === "production"
        ? t("service.stock_total", { stock: totalStock })
        : t("service.hunter_slots", { active: authoritativeService?.active.length ?? 0, slots: authoritativeService?.slots ?? 0 });
      tabs.append(capacityLabel);
      const productList = document.createElement("div");
      productList.className = "service-product-list";
      productList.replaceChildren(...recipes.map((recipe) => {
        const row = document.createElement("div");
        row.className = "service-product-row";
        const icon = document.createElement("img");
        const productIcon = recipe.icon;
        if (productIcon) icon.src = productIcon;
        else icon.hidden = true;
        icon.alt = "";
        const text = document.createElement("span");
        const name = document.createElement("strong");
        name.textContent = recipe.product_name;
        const effect = document.createElement("small");
        effect.textContent = t("service.recover", { value: formatNumber(recipe.effect_value), effect: route?.effectKind ?? recipe.effect_kind, seconds: recipe.duration_ms / 1000 });
        const economy = document.createElement("small");
        if (route) {
          const goldIcon = document.createElement("img");
          goldIcon.className = "inline-currency-icon";
          goldIcon.src = originalAsset("sprites/top_ic_01_gold_24__4677.png");
          goldIcon.alt = "";
          economy.append(document.createTextNode(`${t("service.fee", { fee: formatNumber(recipe.sale_price) })} `), goldIcon);
        } else {
          economy.textContent = t("service.fee_capacity", { fee: formatNumber(recipe.sale_price), stock: recipe.stock, capacity: recipe.capacity });
        }
        text.append(name, effect, economy);
        const action = document.createElement("button");
        action.type = "button";
        action.textContent = t("common.produce");
        row.append(icon, text, action);
        const openProduct = () => {
          context.selectedRecipe = recipe;
          context.selectedServiceMaterialId = null;
          context.selectedServiceQuantity = 1;
          context.consumCreatePop.hidden = false;
          renderConsumCreatePop();
        };
        action.addEventListener("click", openProduct);
        return row;
      }));
      const nextRecipe = allRecipes
        .filter((recipe) => recipe.required_level >= currentBuildingLevel)
        .sort((left, right) => left.required_level - right.required_level)[0];
      const upgradeHint = document.createElement("div");
      upgradeHint.className = "service-upgrade-hint";
      if (nextRecipe) {
        upgradeHint.textContent = t("service.upgrade_product", { level: nextRecipe.required_level + 1, product: nextRecipe.product_name });
        if (targetRequirement !== null) {
          const requirement = document.createElement("em");
          requirement.textContent = originalUiLabel("buildpop_9", "vi", [targetRequirement]);
          upgradeHint.append(requirement);
        }
      } else {
        upgradeHint.textContent = t("service.all_products");
      }
      if (activeServiceTab === "hunters") {
        const hunterList = document.createElement("div");
        hunterList.className = "service-hunter-list";
        if (!authoritativeService?.roster_resolved) {
          const blocked = document.createElement("div");
          blocked.className = "service-hunter-empty";
          blocked.textContent = authoritativeService?.blockers.join(" · ") || t("service.state_unresolved");
          hunterList.append(blocked);
        } else {
          const stockedRecipes = recipes.filter((recipe) => recipe.stock > 0);
          const candidates = authoritativeService.hunters.filter((hunter) => hunter.current_value < hunter.maximum_value || hunter.service_state === "serving");
          for (const hunter of candidates) {
            const row = document.createElement("div");
            row.className = "service-product-row service-hunter-row";
            const text = document.createElement("span");
            const name = document.createElement("strong");
            name.textContent = t("service.hunter_name", { id: hunter.hunter_id });
            const gauge = document.createElement("small");
            gauge.textContent = t("service.hunter_gauge", { effect: route.effectKind, current: formatNumber(hunter.current_value), maximum: formatNumber(hunter.maximum_value) });
            text.append(name, gauge);
            const product = document.createElement("select");
            for (const recipe of stockedRecipes) {
              const option = document.createElement("option");
              option.value = recipe.id;
              option.textContent = t("service.product_bonus", { product: recipe.product_name, value: formatNumber(recipe.effect_value) });
              product.append(option);
            }
            const action = document.createElement("button");
            action.type = "button";
            const activeVisit = authoritativeService.active.find((visit) => visit.hunter_id === hunter.hunter_id);
            action.textContent = activeVisit ? `${Math.ceil(activeVisit.remaining_ms / 1000)}s` : route.buildingId === "build_9" ? t("service.rest") : route.buildingId === "build_12" ? t("service.treat") : t("service.serve");
            action.disabled = hunter.service_state === "serving" || stockedRecipes.length === 0 || authoritativeService.available_slots === 0 || !selectedInstance;
            action.addEventListener("click", () => {
              if (selectedInstance && product.value) context.client.startBuildingService(selectedInstance.instance_id, hunter.hunter_id, product.value);
            });
            row.append(text, product, action);
            hunterList.append(row);
          }
          if (candidates.length === 0) {
            const empty = document.createElement("div");
            empty.className = "service-hunter-empty";
            empty.textContent = route.hunterEmptyLabel;
            hunterList.append(empty);
          }
        }
        context.buildingCatalog.replaceChildren(tabs, hunterList, upgradeHint);
      } else {
        context.buildingCatalog.replaceChildren(tabs, productList, upgradeHint);
      }
    } else if (context.buildingPanelMode === "building" && evidence.popupRoute === "service") {
      renderBuildingContractError(t("building.service_contract_missing"));
      return;
    } else if (context.buildingPanelMode === "building") {
      // Detail popups show the building function; upgrade levels belong to the
      // upgrade action and must not replace the building's main content.
      context.buildingLevelContract.hidden = true;
      context.buildingCatalog.hidden = true;
      context.buildingCatalog.replaceChildren();
    } else {
      context.buildingLevelContract.hidden = false;
    }
    context.buildingLevelContract.title = evidence.constructionBlockedReason ?? "";
    const spriteResolved = evidence.spriteAssetId !== null;
    context.buildingConstruct.hidden = context.buildingPanelMode !== "construct";
    context.buildingUpgrade.hidden = context.buildingPanelMode !== "building" || isEnhancementForgeRoute;
    const isBounty = evidence.id === BOUNTY_HUT_ROUTE.buildingId;
    context.buildingUse.hidden = context.buildingPanelMode !== "building"
      || evidence.id === TRADING_POST_ROUTE.buildingId
      || (evidence.popupRoute !== "request" && !isBounty && !isEnhancementForgeRoute);
    context.buildingConstruct.disabled = !spriteResolved || state?.constructed !== false || state.can_construct !== true;
    context.buildingConstruct.title = !spriteResolved ? t("building.sprite_unresolved") : state?.condition ?? "";
    context.buildingUpgrade.disabled = !spriteResolved || !selectedInstance || selectedInstance.can_upgrade !== true;
    context.buildingUpgrade.title = selectedInstance?.condition ?? "";
    context.buildingUpgrade.textContent = productServiceRoute(evidence.id) || isCatalogShopRoute
      ? originalUiLabel("buildpop_7")
      : `${originalUiLabel("buildpop_7")} · ${formatLevelCosts(evidence, targetLevel)}`;
    const selectedEnhancement = isEnhancementForgeRoute
      ? context.latestSnapshot?.hunter_roster.active_hunters
        .filter((hunter) => context.enhancementHunterId === null || hunter.hunter_id === context.enhancementHunterId)
        .flatMap((hunter) => hunter.gear_enhancements.map((gear) => ({ hunter, gear, key: gear.instance_id ?? `${hunter.hunter_id}:${gear.product_id}` })))
        .find((row) => row.key === context.selectedEnhancementGearKey) ?? null
      : null;
    const enhancementCanSubmit = selectedEnhancement !== null
      && canSubmitGearEnhancement(projectGearEnhancement(selectedEnhancement.gear, context.selectedEnhancementMode));
    context.buildingUse.disabled = isEnhancementForgeRoute
      ? !selectedInstance || selectedEnhancement === null || (context.enhancementView === "configure" && !enhancementCanSubmit)
      : !selectedInstance || (evidence.popupRoute !== "request" && evidence.popupRoute !== "production" && !isBounty);
    context.buildingUse.title = isEnhancementForgeRoute
      ? context.enhancementView === "select"
        ? t("enhancement.select_tooltip")
        : t("enhancement.locked_tooltip")
      : evidence.popupRoute
      ? t("building.popup_resolved_unimplemented")
      : evidence.actionBlockedReason ?? t("building.popup_unresolved");
    const serviceLabels: Record<string, MessageKey> = {
      build_2: "service.revive",
      build_9: "service.rest",
      build_12: "service.treat_hunter",
      build_13: "service.serve_meal",
      build_19: "service.serve_drink",
      build_24: "service.bank",
      build_25: "service.study",
      build_26: "service.restore",
      build_27: "service.encourage",
      build_28: "service.train",
    };
    context.buildingUse.textContent = isEnhancementForgeRoute ? (context.enhancementView === "select" ? t("enhancement.continue") : t("enhancement.action")) : isBounty ? t("service.bounties") : evidence.popupRoute === "request" ? t("service.requests")
      : evidence.popupRoute === "service" ? t(serviceLabels[evidence.id] ?? "service.use")
      : evidence.popupRoute === "production" ? t("common.create") : t("common.open");
  }
  
  function renderBuildingContractError(message: string): void {
    const error = document.createElement("p");
    error.className = "building-contract-error";
    error.textContent = message;
    context.buildingLevelContract.hidden = true;
    context.buildingCatalog.hidden = false;
    context.buildingCatalog.replaceChildren(error);
    context.buildingUse.hidden = true;
  }
  
  function gearKindFromRecipe(recipe: ShopRecipeSnapshot): GearKind | null {
    const match = recipe.id.match(/^recipe:(weapon|armor|gloves|boots|ring|necklace|belt):/);
    const kind = match?.[1] as GearKind | undefined;
    return kind && ALL_GEAR_KINDS.includes(kind) ? kind : null;
  }
  
  function openGearRecipe(recipe: ShopRecipeSnapshot): void {
    context.selectedRecipe = recipe;
    context.gearPopupMode = "craft";
    context.selectedServiceMaterialId = null;
    context.gearCreateQuantity.value = "1";
    context.gearCreatePop.hidden = false;
    renderGearCreatePop();
  }
  
  function openGearDetail(recipe: ShopRecipeSnapshot): void {
    context.selectedRecipe = recipe;
    context.gearPopupMode = "detail";
    context.gearCreatePop.hidden = false;
    renderGearCreatePop();
  }
  
  function appendGearArt(target: HTMLElement, recipe: ShopRecipeSnapshot): void {
    const frame = document.createElement("div");
    frame.className = "gear-item-art";
    const icon = document.createElement("img");
    icon.alt = "";
    if (recipe.icon) icon.src = recipe.icon;
    else icon.hidden = true;
    const placeholder = document.createElement("i");
    placeholder.hidden = Boolean(recipe.icon);
    frame.append(icon, placeholder);
    target.append(frame);
  }
  
  function fullGearRecipes(liveRecipes: readonly ShopRecipeSnapshot[], producerBuildingId: string): ShopRecipeSnapshot[] {
    if (context.gearCatalog.length === 0) return liveRecipes.filter((recipe) => recipe.shop_id === producerBuildingId);
    const allowedKinds = producerBuildingId === JEWELER_BUILDING_IDS[0] ? JEWELER_GEAR_TABS : BLACKSMITH_GEAR_TABS;
    const liveById = new Map<string, ShopRecipeSnapshot[]>();
    for (const recipe of liveRecipes) {
      const rows = liveById.get(recipe.id) ?? [];
      rows.push(recipe);
      liveById.set(recipe.id, rows);
    }
    const familyCapacity = (kind: GearCatalogRecipe["kind"]): number => liveRecipes
      .filter((recipe) => {
        const recipeKind = gearKindFromRecipe(recipe);
        if (kind === "weapon") return recipeKind === "weapon";
        if (JEWELER_GEAR_TABS.some((candidate) => candidate === kind)) {
          return recipeKind !== null && JEWELER_GEAR_TABS.some((candidate) => candidate === recipeKind);
        }
        return recipeKind !== null && BLACKSMITH_GEAR_TABS.some((candidate) => candidate === recipeKind) && recipeKind !== "weapon";
      })
      .reduce((capacity, recipe) => Math.max(capacity, recipe.capacity), 0);
    return context.gearCatalog.filter((entry) => allowedKinds.some((kind) => kind === entry.kind)).map((entry) => {
      const live = liveById.get(entry.id) ?? [];
      return {
        id: entry.id,
        shop_id: producerBuildingId,
        icon: live.find((recipe) => recipe.icon)?.icon ?? entry.iconPath ?? "",
        product_name: entry.productName,
        material_costs: entry.materialCosts.map((cost) => ({
          material_id: cost.materialId,
          display_name: cost.displayName,
          quantity: cost.quantity,
          output_quantity: 1,
        })),
        stock: live.reduce((stock, recipe) => Math.max(stock, recipe.stock), 0),
        sale_price: entry.salePrice,
        kind: "gear",
        required_level: entry.rating,
        duration_ms: 0,
        cooldown_ms: 0,
        effect_value: 0,
        effect_kind: "none",
        capacity: live.reduce((capacity, recipe) => Math.max(capacity, recipe.capacity), familyCapacity(entry.kind)),
      };
    });
  }
  
  function renderGearCraftingCatalog(recipes: readonly ShopRecipeSnapshot[], producerBuildingId: string): void {
    const qualityLabels = [t("craft.quality.regular"), t("craft.quality.sturdy"), t("craft.quality.refined"), t("craft.quality.powerful"), t("craft.quality.supreme")] as const;
    const tabsForBuilding: readonly GearKind[] = producerBuildingId === JEWELER_BUILDING_IDS[0] ? JEWELER_GEAR_TABS : BLACKSMITH_GEAR_TABS;
    if (!tabsForBuilding.includes(context.gearTab)) context.gearTab = tabsForBuilding[0];
    const all = fullGearRecipes(recipes, producerBuildingId).filter((recipe) => gearKindFromRecipe(recipe) === context.gearTab);
    const catalogById = new Map(context.gearCatalog.map((entry) => [entry.id, entry]));
    const buildingLevel = findBuildingInstanceById(
      context.latestSnapshot?.village.building_system.instances ?? [],
      context.selectedBuildingInstanceId,
    )?.level ?? 1;
    const difficultyOptions = producerBuildingId === JEWELER_BUILDING_IDS[0]
      ? [t("difficulty.junk"), t("difficulty.easy"), t("difficulty.normal"), t("difficulty.hard"), t("difficulty.expert"), t("difficulty.nightmare"), t("difficulty.torment")]
      : [t("difficulty.easy"), t("difficulty.normal"), t("difficulty.hard"), t("difficulty.expert"), t("difficulty.nightmare"), t("difficulty.torment")];
    const maxDifficultyGroup = Math.min(6, producerBuildingId === JEWELER_BUILDING_IDS[0]
      ? Math.max(0, buildingLevel - 1)
      : Math.max(1, buildingLevel));
    if (context.blacksmithDifficultyGroup > maxDifficultyGroup) context.blacksmithDifficultyGroup = maxDifficultyGroup;
    const matching = all.filter((recipe) => {
      const staticRow = catalogById.get(recipe.id);
      return (staticRow?.difficultyGroup === undefined || staticRow.difficultyGroup < 0 || staticRow.difficultyGroup === context.blacksmithDifficultyGroup)
        && (!context.blacksmithCraftableOnly || recipe.material_costs.every((cost) => {
        const stock = context.latestSnapshot?.village.building_system.material_stocks.find((item) => item.id === cost.material_id);
        return (stock?.town_quantity ?? 0) >= cost.quantity;
        }));
    });
    const controls = document.createElement("div");
    controls.className = "blacksmith-controls";
    const tabs = document.createElement("div");
    tabs.className = "blacksmith-tabs";
    tabs.style.gridTemplateColumns = `repeat(${tabsForBuilding.length}, minmax(0, 1fr))`;
    for (const tab of tabsForBuilding) {
      const button = document.createElement("button");
      button.type = "button";
      button.setAttribute("aria-label", t(`craft.kind.${tab}` as MessageKey));
      button.dataset.gearTab = tab;
      button.className = tab === context.gearTab ? "selected" : "";
      button.addEventListener("click", () => { context.gearTab = tab; renderBuildingSystem(context.latestSnapshot); });
      tabs.append(button);
    }
    const filters = document.createElement("div");
    filters.className = "blacksmith-filters";
    const difficultyEntries = difficultyOptions.slice(0, maxDifficultyGroup + (producerBuildingId === JEWELER_BUILDING_IDS[0] ? 1 : 0)).map((label, index) => {
      const group = producerBuildingId === JEWELER_BUILDING_IDS[0] ? index : index + 1;
      return { value: String(group), label };
    });
    const difficulty = createGameDropdown(t("craft.gear_difficulty"), String(context.blacksmithDifficultyGroup), difficultyEntries, (value) => {
      context.blacksmithDifficultyGroup = Number(value);
      renderBuildingSystem(context.latestSnapshot);
    });
    filters.append(difficulty);
    const craftable = document.createElement("label");
    craftable.className = "blacksmith-craftable";
    const checkbox = document.createElement("input"); checkbox.type = "checkbox"; checkbox.checked = context.blacksmithCraftableOnly;
    checkbox.addEventListener("change", () => { context.blacksmithCraftableOnly = checkbox.checked; renderBuildingSystem(context.latestSnapshot); });
    craftable.append(checkbox, document.createTextNode(t("craft.craftable_items")));
    controls.append(tabs, filters);
    const grid = document.createElement("div"); grid.className = "blacksmith-grid";
    grid.replaceChildren(...matching.map((recipe) => {
      const card = document.createElement("button"); card.type = "button";
      card.className = "gear-catalog-card";
      card.dataset.rating = String(recipe.required_level);
      const qualityLabel = qualityLabels[recipe.required_level] ?? t("craft.quality_fallback", { quality: recipe.required_level });
      card.setAttribute("aria-label", t("craft.item_quality", { item: recipe.product_name, quality: qualityLabel }));
      appendGearArt(card, recipe);
      const name = document.createElement("strong"); name.textContent = t("craft.item_quality", { item: recipe.product_name, quality: qualityLabel });
      const action = document.createElement("b"); action.textContent = t("common.create");
      card.append(name, action);
      card.addEventListener("click", () => openGearRecipe(recipe));
      return card;
    }));
    if (matching.length === 0) {
      const empty = document.createElement("p");
      empty.className = "blacksmith-empty";
      empty.textContent = t("craft.no_filter_matches");
      grid.append(empty);
    }
    const footer = document.createElement("div");
    footer.className = "blacksmith-catalog-footer";
    const count = document.createElement("span");
    count.textContent = t("craft.item_count", { count: matching.length });
    footer.append(count, craftable);
    const hint = document.createElement("div"); hint.className = "blacksmith-upgrade-hint";
    const nextDifficulty = difficultyOptions[buildingLevel];
    hint.textContent = nextDifficulty
      ? t("craft.gear_upgrade_hint", { level: buildingLevel + 1, difficulty: nextDifficulty, kind: producerBuildingId === JEWELER_BUILDING_IDS[0] ? t("craft.kind.accessories") : t("craft.kind.weapons_armor") })
      : t("craft.all_gear_difficulties");
    context.buildingCatalog.replaceChildren(controls, grid, footer, hint);
  }
  
  function renderEnhancementForge(_system: BuildingSystemSnapshot): void {
    const ownedRows = context.latestSnapshot?.hunter_roster.active_hunters
      .filter((hunter) => context.enhancementHunterId === null || hunter.hunter_id === context.enhancementHunterId)
      .flatMap((hunter) => (
      hunter.gear_enhancements.map((gear) => ({ hunter, gear, key: gear.instance_id ?? `${hunter.hunter_id}:${gear.product_id}` }))
    )) ?? [];
    if (!ownedRows.some((row) => row.key === context.selectedEnhancementGearKey)) context.selectedEnhancementGearKey = null;
    const selected = ownedRows.find((row) => row.key === context.selectedEnhancementGearKey) ?? null;
    const task = context.enhancementHunterId === null
      ? null
      : context.latestSnapshot?.hunter_roster.active_hunters.find((hunter) => hunter.hunter_id === context.enhancementHunterId)?.gear_enhancement_task ?? null;
    if (!selected) context.enhancementView = "select";
    const presentation = selected ? enhancementGearPresentation(selected.gear.product_id, _system) : null;
  
    const shell = document.createElement("section");
    shell.className = "enhancement-forge-shell";
    shell.setAttribute("aria-label", t("enhancement.controls_aria"));
  
    const workspace = document.createElement("div");
    workspace.className = "enhancement-workspace";
    const requiredMaterial = task?.required_materials[0];
    workspace.append(
      createEnhancementMaterialSlot(t("enhancement.optional_material"), t("enhancement.light_stone"), "optional", "material:137", "--/--"),
      createEnhancementGearSlot(selected?.gear.level ?? null, presentation?.name ?? t("enhancement.select_gear"), presentation?.icon ?? null),
      createEnhancementMaterialSlot(t("enhancement.required_material"), requiredMaterial?.material_id === "material:160" ? t("enhancement.ultimate_stone") : t("enhancement.stone"), "required", requiredMaterial?.material_id ?? "material:160", requiredMaterial ? `?/${requiredMaterial.quantity}` : "--/--"),
    );
  
    const stage = document.createElement("div");
    stage.className = "enhancement-stage";
    const hunterActor = document.createElement("div");
    hunterActor.className = "enhancement-stage-actor hunter";
    const hunterSilhouette = document.createElement("i");
    const hunterName = document.createElement("span");
    hunterName.textContent = selected?.hunter.display_name ?? t("common.hunter");
    hunterActor.append(hunterSilhouette, hunterName);
    const anvil = document.createElement("div");
    anvil.className = "enhancement-anvil";
    anvil.setAttribute("aria-hidden", "true");
    const smithActor = document.createElement("div");
    smithActor.className = "enhancement-stage-actor smith";
    const smithSilhouette = document.createElement("i");
    const smithName = document.createElement("span");
    smithName.textContent = t("enhancement.smith");
    smithActor.append(smithSilhouette, smithName);
    stage.append(hunterActor, anvil, smithActor);
  
    const stateBanner = document.createElement("strong");
    stateBanner.className = "enhancement-state-banner";
    stateBanner.textContent = context.enhancementView === "configure" && selected ? t("enhancement.configure") : t("enhancement.select_prompt");
  
    const configureControls = document.createElement("div");
    configureControls.className = "enhancement-configure-controls";
    const cost = document.createElement("div");
    cost.className = "enhancement-cost-row unresolved";
    const goldIcon = document.createElement("img");
    goldIcon.src = originalAsset("sprites/top_ic_01_gold_24__4677.png");
    goldIcon.alt = t("common.gold");
    const nextCost = task?.next_attempt_gold_cost;
    cost.append(document.createTextNode(t("enhancement.hunter_fee")), goldIcon, document.createTextNode(nextCost === null || nextCost === undefined ? t("common.unresolved") : formatNumber(nextCost)));
    if (task?.next_attempt_success_bps !== null && task?.next_attempt_success_bps !== undefined) {
      cost.append(document.createTextNode(` · ${t("enhancement.success_rate", { rate: task.next_attempt_success_bps / 100 })}`));
    }
    const assists = document.createElement("div");
    assists.className = "enhancement-assists";
    const optionalMaterials = [[t("enhancement.light_stone"), "material:137"], [t("enhancement.ore"), "material:154"]] as const;
    for (const [label, materialId] of optionalMaterials) {
      const option = document.createElement("label");
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.value = materialId;
      checkbox.checked = context.selectedEnhancementOptionalMaterialIds.includes(materialId);
      checkbox.disabled = !task || task.blockers.length > 0;
      checkbox.addEventListener("change", () => {
        context.selectedEnhancementOptionalMaterialIds = optionalMaterials
          .filter(([, id]) => id === materialId ? checkbox.checked : context.selectedEnhancementOptionalMaterialIds.includes(id))
          .map(([, id]) => id);
        renderBuildingSystem(context.latestSnapshot);
      });
      option.append(checkbox, document.createTextNode(label));
      assists.append(option);
    }
    const modes = document.createElement("div");
    modes.className = "enhancement-mode-options";
    const labels = {
      single: t("enhancement.mode.single"),
      to_10: t("enhancement.mode.to_10"),
      to_15: t("enhancement.mode.to_15"),
      to_20: t("enhancement.mode.to_20"),
    } as const;
    for (const mode of GEAR_ENHANCEMENT_MODES) {
      const option = document.createElement("label");
      const radio = document.createElement("input");
      radio.type = "radio";
      radio.name = "enhancement-mode";
      radio.value = mode;
      radio.checked = context.selectedEnhancementMode === mode;
      radio.addEventListener("change", () => {
        context.selectedEnhancementMode = mode;
        renderBuildingSystem(context.latestSnapshot);
      });
      option.append(radio, document.createTextNode(labels[mode]));
      modes.append(option);
    }
  
    configureControls.append(cost, assists, modes);
  
    const wallet = document.createElement("div");
    wallet.className = "enhancement-wallet";
    const walletIcon = document.createElement("img");
    walletIcon.src = originalAsset("sprites/top_ic_01_gold_24__4677.png");
    walletIcon.alt = "";
    const walletAmount = document.createElement("b");
    walletAmount.textContent = selected ? formatNumber(selected.hunter.gold) : "--";
    wallet.append(document.createTextNode(t("enhancement.hunter_wallet")), walletIcon, walletAmount);
  
    const inventory = document.createElement("div");
    inventory.className = "enhancement-inventory";
    inventory.setAttribute("aria-label", t("enhancement.owned_gear_aria"));
    for (const owned of ownedRows) {
      const gearPresentation = enhancementGearPresentation(owned.gear.product_id, _system);
      const row = document.createElement("button");
      row.type = "button";
      row.className = "enhancement-owned-row";
      row.classList.toggle("selected", owned.key === context.selectedEnhancementGearKey);
      const frame = document.createElement("span");
      if (gearPresentation.icon) {
        const image = document.createElement("img");
        image.src = gearPresentation.icon;
        image.alt = "";
        frame.append(image);
      } else {
        frame.textContent = "?";
      }
      if (owned.gear.level !== null) {
        const badge = document.createElement("b");
        badge.textContent = `+${owned.gear.level}`;
        frame.append(badge);
      }
      const name = document.createElement("small");
      name.textContent = gearPresentation.name;
      row.append(frame, name);
      row.addEventListener("click", () => {
        context.selectedEnhancementGearKey = owned.key;
        renderBuildingSystem(context.latestSnapshot);
      });
      inventory.append(row);
    }
    if (ownedRows.length === 0) {
      const empty = document.createElement("p");
      empty.className = "enhancement-empty";
      empty.textContent = t("enhancement.no_owned_gear");
      inventory.append(empty);
    }
  
    const capNote = document.createElement("p");
    capNote.className = "enhancement-cap-note";
    capNote.textContent = t("enhancement.max_level");
  
    const evidence = document.createElement("p");
    evidence.className = "enhancement-evidence-note";
    evidence.textContent = t("enhancement.evidence_note");
  
    const processing = document.createElement("section");
    processing.className = "enhancement-processing";
    processing.hidden = context.enhancementView !== "processing";
    processing.innerHTML = `<div class="enhancement-processing-art"></div><strong>${t("enhancement.processing")}</strong>`;
    const result = document.createElement("section");
    result.className = "enhancement-result";
    result.hidden = context.enhancementView !== "result";
    const finalLevel = task?.final_level === null || task?.final_level === undefined ? null : `+${task.final_level}`;
    const spent = task ? t("enhancement.spent_gold", { amount: formatNumber(task.spent_gold) }) : t("enhancement.waiting_snapshot");
    result.innerHTML = `<strong>${t("enhancement.result")}</strong><p>${finalLevel ? t("enhancement.final_level", { level: finalLevel }) : t("enhancement.final_level_unresolved")}<br />${spent}<br />${task?.stop_reason ?? ""}</p>`;
  
    shell.append(workspace, stage, stateBanner);
    if (context.enhancementView === "configure" && selected) shell.append(configureControls);
    shell.append(wallet, inventory, capNote, evidence, processing, result);
    context.buildingCatalog.replaceChildren(shell);
  }
  
  function enhancementGearPresentation(productId: string, system: BuildingSystemSnapshot): { name: string; icon: string | null } {
    const live = system.recipes.find((recipe) => recipe.id === productId);
    const catalog = context.gearCatalog.find((recipe) => recipe.id === productId);
    return {
      name: live?.product_name ?? catalog?.productName ?? productId,
      icon: live?.icon || catalog?.iconPath || null,
    };
  }
  
  function createEnhancementMaterialSlot(titleText: string, nameText: string, kind: "optional" | "required", materialId: string, countText: string): HTMLElement {
    const slot = document.createElement("div");
    slot.className = `enhancement-material-slot ${kind}`;
    const title = document.createElement("strong");
    title.textContent = titleText;
    const frame = document.createElement("span");
    const iconPath = `/content/releases/evil-hunter-1.411/material-icons/${materialId.replace(":", "-")}.png`;
    const icon = document.createElement("img");
    icon.src = iconPath;
    icon.alt = "";
    const count = document.createElement("b");
    count.className = "enhancement-material-count";
    count.textContent = countText;
    frame.append(icon, count);
    const name = document.createElement("small");
    name.textContent = nameText;
    slot.append(title, frame, name);
    return slot;
  }
  
  function createEnhancementGearSlot(level: number | null, nameText: string, iconPath: string | null): HTMLElement {
    const slot = document.createElement("div");
    slot.className = "enhancement-selected-gear";
    const frame = document.createElement("span");
    if (iconPath) {
      const icon = document.createElement("img");
      icon.src = iconPath;
      icon.alt = "";
      frame.append(icon);
    } else {
      frame.textContent = "+";
    }
    if (level !== null) {
      const badge = document.createElement("b");
      badge.textContent = `+${level}`;
      frame.append(badge);
    }
    const name = document.createElement("strong");
    name.textContent = nameText;
    slot.append(frame, name);
    return slot;
  }
  
  function createGameDropdown(
    label: string,
    value: string,
    options: readonly { value: string; label: string }[],
    onChange: (value: string) => void,
  ): HTMLElement {
    const dropdown = document.createElement("div");
    dropdown.className = "game-dropdown";
    const trigger = document.createElement("button");
    trigger.type = "button";
    trigger.className = "game-dropdown-trigger";
    trigger.setAttribute("aria-label", label);
    trigger.setAttribute("aria-expanded", "false");
    trigger.textContent = options.find((option) => option.value === value)?.label ?? options[0]?.label ?? "--";
    const menu = document.createElement("div");
    menu.className = "game-dropdown-menu";
    menu.setAttribute("role", "listbox");
    menu.setAttribute("aria-label", label);
    menu.hidden = true;
    const close = (): void => {
      menu.hidden = true;
      trigger.setAttribute("aria-expanded", "false");
    };
    const closeOtherDropdowns = (): void => {
      document.querySelectorAll<HTMLElement>(".game-dropdown").forEach((candidate) => {
        if (candidate === dropdown) return;
        const candidateMenu = candidate.querySelector<HTMLElement>(".game-dropdown-menu");
        const candidateTrigger = candidate.querySelector<HTMLElement>(".game-dropdown-trigger");
        if (candidateMenu) candidateMenu.hidden = true;
        candidateTrigger?.setAttribute("aria-expanded", "false");
      });
    };
    trigger.addEventListener("click", () => {
      const opening = menu.hidden;
      closeOtherDropdowns();
      menu.hidden = !opening;
      trigger.setAttribute("aria-expanded", String(opening));
    });
    trigger.addEventListener("keydown", (event) => {
      if (event.key === "ArrowDown" || event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        closeOtherDropdowns();
        menu.hidden = false;
        trigger.setAttribute("aria-expanded", "true");
        menu.querySelector<HTMLButtonElement>("button.selected, button")?.focus();
      } else if (event.key === "Escape") {
        close();
      }
    });
    for (const option of options) {
      const item = document.createElement("button");
      item.type = "button";
      item.className = option.value === value ? "selected" : "";
      item.setAttribute("role", "option");
      item.setAttribute("aria-selected", String(option.value === value));
      item.textContent = option.label;
      item.addEventListener("click", () => onChange(option.value));
      item.addEventListener("keydown", (event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          close();
          trigger.focus();
          return;
        }
        if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
        event.preventDefault();
        const items = Array.from(menu.querySelectorAll<HTMLButtonElement>("button"));
        const offset = event.key === "ArrowDown" ? 1 : -1;
        items[(items.indexOf(item) + offset + items.length) % items.length]?.focus();
      });
      menu.append(item);
    }
    dropdown.addEventListener("focusout", (event) => {
      if (!dropdown.contains(event.relatedTarget as Node | null)) close();
    });
    dropdown.append(trigger, menu);
    return dropdown;
  }
  
  function renderDisplayShopCatalog(recipes: readonly ShopRecipeSnapshot[], buildingId: string | null): void {
    const system = context.latestSnapshot?.village.building_system;
    const level = findBuildingInstanceById(system?.instances ?? [], context.selectedBuildingInstanceId)?.level ?? 1;
    const allowed = recipes.filter((recipe) => recipe.shop_id === buildingId && recipe.required_level < level);
    const heading = document.createElement("h3");
    const isPotionShop = buildingId === POTION_SHOP_BUILDING_ID;
    heading.textContent = isPotionShop ? t("shop.potion_display") : t("shop.display_list");
    const grid = document.createElement("div");
    grid.className = isPotionShop ? "display-shop-grid potion-recipe-grid" : "display-shop-grid";
    grid.replaceChildren(...allowed.map((recipe) => {
      if (isPotionShop) {
        const card = document.createElement("button");
        card.type = "button";
        card.className = "gear-catalog-card display-card potion-catalog-card potion-display-card";
        const badge = document.createElement("span");
        badge.className = "potion-stock-badge";
        badge.textContent = `${t("common.stock")}\n${recipe.stock}`;
        appendPotionArt(card, recipe);
        const name = document.createElement("strong");
        name.textContent = recipe.product_name;
        let stat: HTMLElement | null = null;
        if (recipe.effect_value > 0) {
          stat = document.createElement("em");
          stat.className = "display-item-stat";
          stat.textContent = t("shop.effect_preview", { value: formatNumber(recipe.effect_value) });
        }
        const price = document.createElement("small");
        const gold = document.createElement("img");
        gold.src = originalAsset("sprites/top_ic_01_gold_24__4677.png");
        gold.alt = "";
        price.append(gold, document.createTextNode(formatNumber(recipe.sale_price)));
        card.append(badge, name, ...(stat ? [stat] : []), price);
        card.addEventListener("click", () => openGearDetail(recipe));
        return card;
      }
      const card = document.createElement("button");
      card.type = "button";
      card.className = "gear-catalog-card display-card";
      const badge = document.createElement("span");
      badge.className = "on-display-badge";
      badge.textContent = t("shop.on_display");
      appendGearArt(card, recipe);
      const name = document.createElement("strong"); name.textContent = recipe.product_name;
      const displayed = system?.display_items.find((item) => item.shop_id === buildingId && item.product_id === recipe.id);
      if (displayed) {
        const stat = document.createElement("em");
        stat.className = "display-item-stat";
        stat.textContent = t("shop.attack_preview", { attack: formatNumber(displayed.primary_stat) });
        card.append(stat);
      }
      const price = document.createElement("small");
      const gold = document.createElement("img");
      gold.src = originalAsset("sprites/top_ic_01_gold_24__4677.png");
      gold.alt = "";
      price.append(gold, document.createTextNode(formatNumber(recipe.sale_price)));
      card.append(badge, name, price);
      card.addEventListener("click", () => openGearDetail(recipe));
      return card;
    }));
    if (allowed.length === 0) {
      const empty = document.createElement("p");
      empty.className = "display-shop-empty";
      empty.textContent = isPotionShop ? t("shop.no_potions") : t("shop.no_gear");
      grid.append(empty);
    }
    const hint = document.createElement("div");
    hint.className = "blacksmith-upgrade-hint";
    const nextTier = ["shop.tier.sturdy", "shop.tier.refined", "shop.tier.powerful", "shop.tier.supreme"] as const;
    hint.textContent = isPotionShop
      ? t("shop.potion_stock_hint")
      : level < 5
        ? t("shop.display_upgrade_hint", { level: level + 1, tier: t(nextTier[Math.min(level - 1, 3)]), kind: t(buildingId === "build_7" ? "shop.kind.weapons" : "shop.kind.armor") })
        : t("shop.all_tiers");
    context.buildingCatalog.replaceChildren(heading, grid, hint);
  }
  
  function appendPotionArt(target: HTMLElement, recipe: ShopRecipeSnapshot): void {
    const art = document.createElement("div");
    art.className = "gear-item-art potion-item-art";
    const icon = document.createElement("img");
    icon.alt = "";
    if (recipe.icon) icon.src = recipe.icon;
    else icon.hidden = true;
    art.append(icon);
    target.append(art);
  }
  
  function renderPotionCraftingCatalog(recipes: readonly ShopRecipeSnapshot[], buildingLevel: number): void {
    const allowed = recipes.filter((recipe) => recipe.shop_id === ALCHEMIST_BUILDING_ID && recipe.required_level < buildingLevel);
    const heading = document.createElement("h3");
    heading.textContent = t("shop.potion_recipes");
    const grid = document.createElement("div");
    grid.className = "display-shop-grid potion-recipe-grid";
    grid.replaceChildren(...allowed.map((recipe) => {
      const card = document.createElement("button");
      card.type = "button";
      card.className = "gear-catalog-card potion-catalog-card";
      const stock = document.createElement("span");
      stock.className = "potion-stock-badge";
      stock.textContent = `${t("common.stock")}\n${recipe.stock}/${recipe.capacity}`;
      appendPotionArt(card, recipe);
      const name = document.createElement("strong");
      name.textContent = recipe.product_name;
      const action = document.createElement("b");
      action.textContent = t("common.create");
      card.append(stock, name, action);
      card.addEventListener("click", () => {
        context.selectedRecipe = recipe;
        context.selectedServiceMaterialId = null;
        context.selectedServiceQuantity = 1;
        context.consumCreatePop.hidden = false;
        renderConsumCreatePop();
      });
      return card;
    }));
    if (allowed.length === 0) {
      const empty = document.createElement("p");
      empty.className = "display-shop-empty";
      empty.textContent = t("shop.no_potion_recipes");
      grid.append(empty);
    }
    const hint = document.createElement("div");
    hint.className = "blacksmith-upgrade-hint";
    hint.textContent = t("shop.potion_craft_hint");
    context.buildingCatalog.replaceChildren(heading, grid, hint);
  }
  
  function renderTradingPostCatalog(
    stocks: readonly MaterialStockSnapshot[],
    buildingLevel: number,
    nextTownHallRequirement: number | null,
  ): void {
    context.selectedTradingPostDifficulty = Math.min(context.selectedTradingPostDifficulty, Math.max(0, buildingLevel - 1));
    const activeRequests = stocks.reduce((total, stock) => total + stock.requested, 0);
    const toolbar = document.createElement("div");
    toolbar.className = "trading-post-toolbar";
    const count = document.createElement("strong");
    count.textContent = t("trading.request_count", { count: activeRequests });
    const difficulty = document.createElement("select");
    difficulty.setAttribute("aria-label", t("trading.difficulty"));
    tradingPostDifficultyOptions(buildingLevel).forEach(({ label, difficulty: index, unlocked }) => {
      const option = document.createElement("option");
      option.value = String(index);
      option.textContent = label;
      option.disabled = !unlocked;
      option.selected = index === context.selectedTradingPostDifficulty;
      difficulty.append(option);
    });
    difficulty.addEventListener("change", () => {
      context.selectedTradingPostDifficulty = Number(difficulty.value);
      difficulty.blur();
      renderBuildingSystem(context.latestSnapshot);
    });
    toolbar.append(count, difficulty);
  
    const visibleStocks = tradingPostStocksForDifficulty(stocks, context.selectedTradingPostDifficulty);
    const grid = document.createElement("div");
    grid.className = "trading-post-grid";
    grid.replaceChildren(...visibleStocks.map((stock) => {
      const card = document.createElement("article");
      card.className = "trading-post-card";
      const iconFrame = document.createElement("div");
      iconFrame.className = "trading-post-icon";
      const icon = document.createElement("img");
      icon.alt = "";
      if (stock.icon) icon.src = stock.icon;
      else icon.hidden = true;
      const remainingRequest = document.createElement("span");
      remainingRequest.textContent = stock.requested > 0 ? String(stock.requested) : "";
      iconFrame.append(icon, remainingRequest);
      const name = document.createElement("strong");
      name.textContent = stock.display_name;
      const action = document.createElement("button");
      action.type = "button";
      action.className = stock.requested > 0 ? "cancel" : "request";
      action.textContent = stock.requested > 0 ? t("trading.cancel_request") : t("common.request");
      action.addEventListener("click", () => {
        if (!context.selectedBuildingInstanceId) return;
        if (stock.requested > 0) context.client.cancelMaterialRequest(context.selectedBuildingInstanceId, stock.id);
        else {
          context.selectedTradingRequest = stock;
          context.selectedTradingRequestQuantity = 1;
          context.tradingRequestPending = false;
          context.renderTradingRequestEditor();
        }
      });
      card.append(iconFrame, name, action);
      return card;
    }));
  
    const hint = document.createElement("div");
    hint.className = "trading-post-upgrade-hint";
    const nextDifficulty = TRADING_POST_ROUTE.tabs[Math.min(buildingLevel, TRADING_POST_ROUTE.tabs.length - 1)];
    hint.textContent = buildingLevel < TRADING_POST_ROUTE.upgrade.maxLevel
      ? t("trading.upgrade_hint", { level: buildingLevel + 1, difficulty: nextDifficulty })
      : t("trading.all_levels");
    if (nextTownHallRequirement !== null) {
      const requirement = document.createElement("em");
      requirement.textContent = originalUiLabel("buildpop_9", "vi", [nextTownHallRequirement]);
      hint.append(requirement);
    }
    context.buildingCatalog.replaceChildren(toolbar, grid, hint);
  }
  
  function renderBountyPop(): void {
    const system = context.latestSnapshot?.village.building_system;
    if (!system || context.selectedBuildingId !== BOUNTY_HUT_ROUTE.buildingId) return;
    const instance = findBuildingInstanceById(system.instances, context.selectedBuildingInstanceId);
    context.bountyTitle.textContent = `${t("common.level_short", { level: instance?.level ?? 1 })} ${BOUNTY_HUT_ROUTE.title}`;
    context.bountyTierTabs.replaceChildren(...BOUNTY_TIERS.map((tier, index) => {
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = tier.label;
      button.className = index === context.selectedBountyTier ? "selected" : "";
      button.addEventListener("click", () => {
        context.selectedBountyTier = index;
        renderBountyPop();
      });
      return button;
    }));
    context.bountyUpgrade.disabled = !instance || instance.can_upgrade !== true;
    const evidence = projectBuildingEvidence(context.buildingEvidenceRegistry, BOUNTY_HUT_ROUTE.buildingId);
    context.bountyUpgrade.textContent = `${t("common.upgrade")} · ${evidence ? formatLevelCosts(evidence, (instance?.level ?? 1) + 1) : t("bounty.cost_unresolved")}`;
  }
  
  function resourceIconPath(resourceId: string): string | null {
    const paths: Record<string, string> = {
      "currency:gem": "/content/releases/original-flow-v1/sprites/top_ic_02_gem__6963.png",
      "currency:elemental": "/content/releases/original-flow-v1/sprites/top_ic_03_element__4250.png",
    };
    return paths[resourceId] ?? null;
  }
  
  function syncBuildingPresentation(target: VisibleEntityWorld, snapshot: OriginalFlowSnapshot): void {
    target.setBuildingPresentation(snapshot.village.building_system.instances);
  }
  
  function renderGearCreatePop(): void {
    const system = context.latestSnapshot?.village.building_system;
    if (!system || !context.selectedRecipe) return;
    const purchase = context.gearPopupMode === "detail" && context.selectedBuildingId
      ? projectShopPurchase(
        system,
        context.latestSnapshot?.hunter_roster.active_hunters ?? [],
        context.selectedBuildingId,
        context.selectedRecipe.id,
        context.purchaseHunterId,
      )
      : null;
    if (context.gearPopupMode === "detail" && !purchase) {
      context.gearCreatePop.hidden = true;
      return;
    }
    if (purchase) context.selectedRecipe = purchase.recipe;
    const quantity = clampQuantity(context.gearCreateQuantity.value, 1, 1000);
    context.gearCreateQuantity.value = String(quantity);
    const gearKind = gearKindFromRecipe(context.selectedRecipe);
    const gearKindLabel = gearKind ? t(`craft.kind.${gearKind}` as MessageKey) : t("shop.kind.consumable");
    context.gearCreatePop.classList.toggle("gear-detail-mode", context.gearPopupMode === "detail");
    context.gearCreateTitle.textContent = context.gearPopupMode === "detail" ? context.selectedRecipe.product_name : t("craft.create_item", { item: gearKindLabel });
    const displayIcon = purchase?.displayItem?.icon || context.selectedRecipe.icon;
    if (displayIcon) context.gearCreateIcon.src = displayIcon;
    else context.gearCreateIcon.removeAttribute("src");
    context.gearCreateIcon.hidden = !displayIcon;
    context.gearCreateName.textContent = context.selectedRecipe.product_name;
    context.gearCreatePrice.textContent = context.gearPopupMode === "detail"
      ? t("craft.gear_display_price", { kind: gearKindLabel, price: formatNumber(purchase?.displayItem?.sale_price ?? context.selectedRecipe.sale_price) })
      : gearKindLabel;
    context.gearCreateDescription.replaceChildren();
    if (purchase) {
      const stats = document.createElement("dl");
      stats.className = "shop-item-stats";
      const catalogItem = context.gearCatalog.find((item) => item.id === context.selectedRecipe?.id);
      const quality = purchase.displayItem
        ? [t("craft.quality.regular"), t("craft.quality.sturdy"), t("craft.quality.refined"), t("craft.quality.powerful"), t("craft.quality.supreme")][purchase.displayItem.quality]
          ?? t("craft.quality_fallback", { quality: purchase.displayItem.quality })
        : null;
      const qualityLabel = (value: number): string => [
        t("craft.quality.regular"),
        t("craft.quality.sturdy"),
        t("craft.quality.refined"),
        t("craft.quality.powerful"),
        t("craft.quality.supreme"),
      ][value] ?? t("craft.quality_fallback", { quality: value });
      const difficultyKeys = ["difficulty.junk", "difficulty.easy", "difficulty.normal", "difficulty.hard", "difficulty.expert", "difficulty.nightmare", "difficulty.torment"] as const;
      const rows: Array<[string, string]> = purchase.displayItem
        ? [
          [t(purchase.displayItem.gear_kind === "weapon" ? "shop.stat.attack" : "shop.stat.primary"), formatNumber(purchase.displayItem.primary_stat)],
          [t("shop.stat.quality"), quality ?? t("shop.stats_unavailable")],
          ...(catalogItem ? [
            [t("shop.stat.item_level"), formatNumber(catalogItem.itemLevel)],
            [t("shop.stat.difficulty"), t(difficultyKeys[catalogItem.difficultyGroup] ?? "difficulty.torment")],
          ] as Array<[string, string]> : []),
          ...(purchase.displayItem.option_type > 0 ? [[
            t("shop.stat.bonus"),
            t("shop.stat.bonus_unresolved", { value: formatNumber(purchase.displayItem.option_value) }),
          ]] as Array<[string, string]> : []),
        ]
        : context.selectedRecipe.effect_value > 0
          ? [
            [t("shop.stat.effect"), formatNumber(context.selectedRecipe.effect_value)],
            [t("shop.stat.cooldown"), t("shop.seconds", { seconds: formatNumber(context.selectedRecipe.cooldown_ms / 1000) })],
          ]
          : [];
      const selectedHunter = purchase.selectedBuyer
        ? context.latestSnapshot?.hunter_roster.active_hunters.find((hunter) => hunter.hunter_id === purchase.selectedBuyer?.hunterId)
        : null;
      if (purchase.displayItem?.gear_kind === "weapon") {
        const equipped = selectedHunter?.hunter_info.weapons.find((weapon) => weapon.equipped) ?? null;
        const legacyEquipped = equipped
          ? null
          : selectedHunter?.hunter_info.equipment_slots?.find((slot) => slot.slot_id === "weapon") ?? null;
        const comparison = document.createElement("section");
        comparison.className = "shop-weapon-comparison";
        const comparisonCard = (kind: "current" | "candidate", name: string, attack: number | null, detail: string): HTMLElement => {
          const card = document.createElement("article");
          card.className = kind;
          const label = document.createElement("small");
          label.textContent = t(kind === "current" ? "shop.compare.current" : "shop.compare.new");
          const itemName = document.createElement("strong");
          itemName.textContent = name;
          const itemAttack = document.createElement("b");
          itemAttack.textContent = attack === null
            ? t("shop.compare.attack_unavailable")
            : t("shop.attack_preview", { attack: formatNumber(attack) });
          const itemQualityText = document.createElement("span");
          itemQualityText.textContent = detail;
          card.append(label, itemName, itemAttack, itemQualityText);
          return card;
        };
        const currentCard = comparisonCard(
          "current",
          equipped?.display_name_vi ?? equipped?.display_name_en ?? legacyEquipped?.display_name ?? t("shop.compare.none_equipped"),
          equipped?.attack_damage ?? null,
          equipped ? qualityLabel(equipped.quality) : legacyEquipped ? t("shop.compare.legacy_equipped") : t("shop.compare.none_equipped"),
        );
        const candidateCard = comparisonCard(
          "candidate",
          context.selectedRecipe.product_name,
          purchase.displayItem.primary_stat,
          qualityLabel(purchase.displayItem.quality),
        );
        const delta = document.createElement("output");
        delta.className = "shop-weapon-delta";
        if (equipped) {
          const attackDelta = purchase.displayItem.primary_stat - equipped.attack_damage;
          delta.classList.toggle("negative", attackDelta < 0);
          delta.textContent = `${attackDelta >= 0 ? "+" : ""}${formatNumber(attackDelta)} ATK`;
        } else {
          delta.textContent = t("shop.compare.new_attack", { attack: formatNumber(purchase.displayItem.primary_stat) });
        }
        comparison.append(currentCard, delta, candidateCard);
        context.gearCreateDescription.append(comparison);
      }
      for (const [label, value] of rows) {
        const term = document.createElement("dt");
        term.textContent = label;
        const description = document.createElement("dd");
        description.textContent = value;
        stats.append(term, description);
      }
      if (rows.length === 0) {
        const unresolved = document.createElement("p");
        unresolved.textContent = t("shop.stats_unavailable");
        context.gearCreateDescription.append(unresolved);
      } else {
        context.gearCreateDescription.append(stats);
      }
      if (purchase.selectedBuyer) {
        const buyerGold = document.createElement("output");
        buyerGold.className = "shop-buyer-gold";
        buyerGold.dataset.hunterId = String(purchase.selectedBuyer.hunterId);
        buyerGold.textContent = formatNumber(purchase.selectedBuyer.gold);
        buyerGold.setAttribute("aria-label", t("shop.buyer_gold_for", {
          name: purchase.selectedBuyer.displayName,
          gold: formatNumber(purchase.selectedBuyer.gold),
        }));
        context.gearCreateDescription.prepend(buyerGold);
        const price = purchase.displayItem?.sale_price ?? purchase.recipe.sale_price;
        const economy = document.createElement("dl");
        economy.className = "shop-purchase-economy";
        for (const [label, value] of [
          [t("shop.purchase_price"), formatNumber(price)],
          [t("shop.gold_after_purchase"), formatNumber(purchase.selectedBuyer.gold - price)],
        ]) {
          const term = document.createElement("dt");
          term.textContent = label;
          const amount = document.createElement("dd");
          amount.textContent = value;
          economy.append(term, amount);
        }
        context.gearCreateDescription.append(economy);
      }
      const status = document.createElement("p");
      status.className = purchase.blocker ? "shop-purchase-blocker" : "shop-purchase-ready";
      const blockerKeys = {
        buyer_required: "shop.blocker.buyer_required",
        buyer_unavailable: "shop.blocker.buyer_unavailable",
        insufficient_gold: "shop.blocker.insufficient_gold",
        out_of_stock: "shop.blocker.out_of_stock",
        price_unresolved: "shop.blocker.price_unresolved",
      } as const;
      if (purchase.blocker) {
        status.textContent = t(blockerKeys[purchase.blocker]);
        context.gearCreateDescription.append(status);
      }
    }
    context.gearLock.hidden = true;
    context.gearLock.disabled = true;
    context.gearMaterialTitle.hidden = context.gearPopupMode === "detail";
    context.gearMaterialTitle.textContent = t("craft.required_materials");
    context.gearMaterialCosts.hidden = context.gearPopupMode === "detail";
    context.gearQuantityRow.hidden = context.gearPopupMode === "detail";
    context.gearCreateSubmit.hidden = context.gearPopupMode === "detail";
    context.gearCreateSubmit.textContent = t("common.produce");
    context.gearCreateSell.hidden = context.gearPopupMode !== "detail";
    context.gearCreateSell.textContent = t("common.buy");
    context.gearCreateSell.disabled = !purchase?.canPurchase || context.pendingPurchase !== null;
    context.gearCreateSell.title = purchase?.blocker ? t({
      buyer_required: "shop.blocker.buyer_required",
      buyer_unavailable: "shop.blocker.buyer_unavailable",
      insufficient_gold: "shop.blocker.insufficient_gold",
      out_of_stock: "shop.blocker.out_of_stock",
      price_unresolved: "shop.blocker.price_unresolved",
    }[purchase.blocker] as MessageKey) : "";
    let craftable = true;
    context.gearMaterialCosts.replaceChildren(...context.selectedRecipe.material_costs.map((cost) => {
      const stock = system.material_stocks.find((item) => item.id === cost.material_id);
      const row = document.createElement("div");
      const selected = true;
      const iconPath = context.gearMaterialIcons.get(cost.material_id) ?? stock?.icon ?? resourceIconPath(cost.material_id);
      const icon = iconPath ? document.createElement("img") : document.createElement("span");
      if (icon instanceof HTMLImageElement) {
        icon.src = iconPath!;
        icon.alt = "";
      } else {
        icon.className = "unresolved-material-icon";
        icon.textContent = cost.display_name.slice(0, 2).toUpperCase();
        icon.title = t("craft.source_sprite_unavailable", { name: cost.display_name });
      }
      const batches = Math.ceil(quantity / Math.max(1, cost.output_quantity));
      const needed = cost.quantity * batches;
      const available = stock?.town_quantity ?? 0;
      row.className = (selected ? "selected " : "") + (selected && available < needed ? "missing" : "");
      row.append(icon, document.createTextNode(`${cost.display_name}  ${available} / ${needed}`));
      if (selected) craftable &&= available >= needed;
      return row;
    }));
    context.gearFrameQuantity.value = String(quantity);
    context.gearCreateSubmit.disabled = context.pendingCraft !== null || context.gearPopupMode !== "craft" || !craftable;
    context.gearCreateSubmit.title = !craftable ? t("craft.materials_missing_tooltip") : "";
  }
  
  function renderConsumCreatePop(): void {
    const system = context.latestSnapshot?.village.building_system;
    if (!system || !context.selectedRecipe) return;
    const isServiceProduct = context.selectedRecipe.kind === "service"
      || productServiceRoute(context.selectedRecipe.shop_id) !== null;
    const isPotionRecipe = context.selectedRecipe.shop_id === ALCHEMIST_BUILDING_ID;
    if (!isServiceProduct && !isPotionRecipe) return;
    // ConsumCreatePop and ProductCreatePop have different recovered layout contracts.
    context.consumCreatePop.classList.toggle("service-product-ui", isServiceProduct);
    context.consumCreatePop.classList.toggle("potion-product-ui", isPotionRecipe);
    if (isServiceProduct) {
      context.selectedServiceMaterialId = resolveServiceMaterialId(
        context.selectedRecipe.material_costs,
        system.material_stocks,
        context.selectedServiceQuantity,
        context.selectedServiceMaterialId,
      );
    }
    const selectedCost = isServiceProduct
      ? context.selectedRecipe.material_costs.find((cost) => cost.material_id === context.selectedServiceMaterialId)
      : null;
    const outputPerBatch = Math.max(1, selectedCost?.output_quantity ?? 1);
    const inputPerBatch = Math.max(1, selectedCost?.quantity ?? 1);
    const availableInput = context.selectedServiceMaterialId
      ? townMaterialQuantity(system.material_stocks, context.selectedServiceMaterialId)
      : 0;
    const possibleOutput = isServiceProduct
      ? Math.floor(availableInput / inputPerBatch) * outputPerBatch
      : context.selectedRecipe.material_costs.reduce((maximum, cost) => {
        const available = system.material_stocks.find((stock) => stock.id === cost.material_id)?.town_quantity ?? 0;
        return Math.min(maximum, Math.floor(available / Math.max(1, cost.quantity)));
      }, Number.MAX_SAFE_INTEGER);
    const remainingCapacity = remainingSharedCapacity(
      system.recipes,
      context.selectedRecipe,
      (candidate, current) => candidate.shop_id === current.shop_id,
    );
    const serviceCapacity = isServiceProduct ? Number.MAX_SAFE_INTEGER : remainingCapacity;
  
    context.consumCreateTitle.textContent = t("craft.produce_item", { item: context.selectedRecipe.product_name });
    if (context.selectedRecipe.icon) context.consumCreateIcon.src = context.selectedRecipe.icon;
    else context.consumCreateIcon.removeAttribute("src");
    context.consumCreateIcon.hidden = !context.selectedRecipe.icon;
    context.consumCreateIconPlaceholder.hidden = Boolean(context.selectedRecipe.icon);
    context.consumCreateQuantity.value = String(context.selectedServiceQuantity);
    context.consumCreateQuantityInput.value = String(context.selectedServiceQuantity);
    context.consumMaterialTitle.textContent = isPotionRecipe ? t("craft.required_materials") : t("craft.select_material");
    context.consumConversion.textContent = isPotionRecipe
      ? `${t("craft.stock", { stock: context.selectedRecipe.stock, capacity: context.selectedRecipe.capacity > 0 ? context.selectedRecipe.capacity : "∞" })}\n${t("craft.produce_progress", { current: context.selectedServiceQuantity, maximum: Math.min(possibleOutput, serviceCapacity) })}`
      : selectedCost
      ? `${t("craft.conversion", { output: outputPerBatch, product: context.selectedRecipe.product_name, input: inputPerBatch, material: selectedCost.display_name })}\n${t("craft.produce_progress", { current: context.selectedServiceQuantity, maximum: possibleOutput })}`
      : t("craft.conversion_unresolved");
    context.consumMaterialGrid.replaceChildren(...context.selectedRecipe.material_costs.map((cost) => {
      const stock = system.material_stocks.find((item) => item.id === cost.material_id);
      const button = document.createElement("button");
      button.type = "button";
      button.className = isPotionRecipe || cost.material_id === context.selectedServiceMaterialId ? "selected" : "";
      const icon = document.createElement("img");
      const iconPath = stock?.icon || resourceIconPath(cost.material_id);
      if (iconPath) icon.src = iconPath;
      else icon.hidden = true;
      icon.alt = "";
      const count = document.createElement("small");
      count.textContent = String(stock?.town_quantity ?? 0);
      const name = document.createElement("span");
      name.textContent = cost.display_name;
      button.append(icon, count, name);
      if (isServiceProduct) {
        button.addEventListener("click", () => {
          context.selectedServiceMaterialId = cost.material_id;
          renderConsumCreatePop();
        });
      } else {
        button.disabled = true;
      }
      return button;
    }));
    const neededInput = selectedCost ? serviceMaterialRequired(selectedCost, context.selectedServiceQuantity) : 0;
    const missingMaterialId = isServiceProduct
      ? selectedCost && availableInput < neededInput ? selectedCost.material_id : null
      : missingCraftMaterial(context.selectedRecipe.material_costs, system.material_stocks, context.selectedServiceQuantity);
    const capacityExceeded = !isServiceProduct && context.selectedServiceQuantity > remainingCapacity;
    context.consumCreateSubmit.disabled = context.pendingCraft !== null
      || (!selectedCost && isServiceProduct)
      || missingMaterialId !== null
      || capacityExceeded;
    context.consumCreateSubmit.title = missingMaterialId
      ? t("craft.selected_material_missing_tooltip")
      : capacityExceeded
        ? t("craft.capacity_tooltip")
        : "";
    if (context.consumCreateSubmit.disabled) {
      context.consumConversion.textContent += missingMaterialId
        ? `\n${t("craft.missing_material")}`
        : `\n${t("craft.destination_full")}`;
    }
  }
  
  
  return {
    popupDataSignature,
    renderBuildingSystem,
    renderGearCreatePop,
    renderConsumCreatePop,
    renderBountyPop,
    resourceIconPath,
    syncBuildingPresentation,
  };
}
