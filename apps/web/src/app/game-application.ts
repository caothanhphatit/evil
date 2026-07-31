import { WorldClient, type BindingBlockedFeedback, type IntentFeedback } from "../net/world-client";
import type { OriginalFlowSnapshot } from "../generated/protocol";
import { VisibleEntityWorld } from "../game/visible-world";
import { nextHunterRosterOpen } from "../ui/bottom-menu-state";
import { createCombatHudController } from "./combat-hud-controller";
import { setPanelMessage } from "../ui/panel-message";
import { bindOverlayCloseControls } from "../ui/overlay-close-controls";
import { createHunterRosterActors } from "../ui/hunter-roster-actors";
import { createHunterInfoModal } from "../ui/hunter-info/modal";
import { createHunterWorldCommandMenu } from "../ui/hunter-world-command";
import { canSubmitGearEnhancement, projectGearEnhancement } from "../ui/gear-enhancement";
import { clampQuantity } from "../ui/shop-crafting";
import { projectBuildingEvidence } from "../content/building-evidence";
import { loadVerifiedBuildingEvidenceRegistry } from "../content/building-registry";
import { originalUiLabel } from "../content/original-ui-labels";
import { BOUNTY_HUT_ROUTE } from "../routes/bounty-hut";
import { TRADING_POST_ROUTE } from "../routes/trading-post";
import { decodeGearCatalog, loadGearCatalog } from "../content/blacksmith-route";
import { t, type MessageKey } from "../i18n";
import { recordClientEvent } from "../observability/client-telemetry";
import { mountGameShell, originalAsset, type MenuAction } from "./shell";
import { EntryController } from "./entry-controller";
import { createBuildingRenderer, type BuildingRenderingContext } from "./building-renderer";
import { createHunterController, type HunterControllerContext } from "./hunter-controller";
import { createWorldController, type WorldControllerContext } from "./world-controller";
import { createTradePopup, type TradePopupContext } from "./trade-popup";
import "../styles.css";
const mount = document.querySelector<HTMLDivElement>("#app");
if (!mount) throw new Error(t("error.missing_mount"));

const debugUi = new URLSearchParams(window.location.search).has("debug");
const gameShell = mountGameShell(mount);

function element<T extends HTMLElement>(selector: string): T { const value = document.querySelector<T>(selector); if (!value) throw new Error(t("error.missing_element", { selector })); return value; }
const villageScreen = element<HTMLElement>("#village-screen");
const rosterScreen = element<HTMLElement>("#roster-screen");
const bottomMenu = element<HTMLElement>("#bottom-menu");
const hunterCapacity = element<HTMLElement>("#hunter-capacity");
const hunterActiveList = element<HTMLElement>("#hunter-active-list");
const hunterRosterStatus = element<HTMLElement>("#hunter-roster-status");
const rosterBack = element<HTMLButtonElement>("#roster-back");
const panelMessage = element<HTMLElement>("#panel-message");
const fpsCounter = element<HTMLElement>("#fps-counter");
const worldViewport = element<HTMLElement>("#world-viewport");
const hunterEnhancementInteractions = element<HTMLElement>("#hunter-enhancement-interactions");
const fieldBack = element<HTMLButtonElement>("#field-back");
const worldModeLabel = element<HTMLElement>("#world-mode-label");
const goldAmount = element<HTMLElement>("#gold-amount");
const hunterPopulation = element<HTMLElement>("#hunter-population");
const buildingPanel = element<HTMLElement>("#building-panel");
const buildingName = element<HTMLElement>("#building-name");
const buildingPreview = element<HTMLImageElement>("#building-preview");
const buildingLevel = element<HTMLElement>("#building-level");
const buildingFeature = element<HTMLElement>("#building-feature");
const buildingCondition = element<HTMLElement>("#building-condition");
const buildingLevelContract = element<HTMLElement>("#building-level-contract");
const buildingCatalog = element<HTMLElement>("#building-catalog");
const buildingConstruct = element<HTMLButtonElement>("#building-construct");
const buildingUpgrade = element<HTMLButtonElement>("#building-upgrade");
const buildingUse = element<HTMLButtonElement>("#building-use");
const buildingPanelClose = element<HTMLButtonElement>("#building-panel-close");
const tradingRequestPop = element<HTMLElement>("#trading-request-pop");
const tradingRequestContent = element<HTMLElement>("#trading-request-content");
const bountyPop = element<HTMLElement>("#bounty-quest-pop");
const bountyTitle = element<HTMLElement>("#bounty-title");
const bountyTierTabs = element<HTMLElement>("#bounty-tier-tabs");
const bountyClose = element<HTMLButtonElement>("#bounty-close");
const bountyCloseBottom = element<HTMLButtonElement>("#bounty-close-bottom");
const bountyUpgrade = element<HTMLButtonElement>("#bounty-upgrade");
const gearCreatePop = element<HTMLElement>("#gear-create-pop");
const gearCreateTitle = element<HTMLElement>("#gear-create-title");
const gearCreateIcon = element<HTMLImageElement>("#gear-create-icon");
const gearCreateName = element<HTMLElement>("#gear-create-name");
const gearCreatePrice = element<HTMLElement>("#gear-create-price");
const gearCreateDescription = element<HTMLElement>("#gear-create-description");
const gearLock = element<HTMLButtonElement>("#gear-lock");
const gearMaterialTitle = element<HTMLElement>("#gear-material-title");
const gearMaterialCosts = element<HTMLElement>("#gear-material-costs");
const gearQuantityRow = element<HTMLElement>("#gear-quantity-row");
const gearCreateQuantity = element<HTMLInputElement>("#gear-create-quantity");
const gearFrameQuantity = element<HTMLOutputElement>("#gear-frame-quantity");
const gearStorageLabel = element<HTMLElement>("#gear-storage-label");
const gearCreateSubmit = element<HTMLButtonElement>("#gear-create-submit");
const gearCreateSell = element<HTMLButtonElement>("#gear-create-sell");
const gearCreateClose = element<HTMLButtonElement>("#gear-create-close");
const consumCreatePop = element<HTMLElement>("#consum-create-pop");
const consumCreateTitle = element<HTMLElement>("#consum-create-title");
const consumCreateIcon = element<HTMLImageElement>("#consum-create-icon");
const consumCreateIconPlaceholder = element<HTMLElement>("#consum-create-icon-placeholder");
const consumCreateQuantity = element<HTMLOutputElement>("#consum-create-quantity");
const consumCreateQuantityInput = element<HTMLInputElement>("#consum-create-quantity-input");
const consumConversion = element<HTMLElement>("#consum-conversion");
const consumMaterialTitle = element<HTMLElement>("#consum-material-title");
const consumMaterialGrid = element<HTMLElement>("#consum-material-grid");
const consumCreateSubmit = element<HTMLButtonElement>("#consum-create-submit");
const consumCreateClose = element<HTMLButtonElement>("#consum-create-close");
const consumMinus = element<HTMLButtonElement>("#consum-minus");
const consumPlus = element<HTMLButtonElement>("#consum-plus");
consumCreateIcon.addEventListener("error", () => {
  consumCreateIcon.hidden = true;
  consumCreateIconPlaceholder.hidden = false;
});
const evidenceDiagnostics = element<HTMLElement>("#evidence-diagnostics");
const combatHud = element<HTMLElement>("#combat-hud");
const equipFixtureItem = element<HTMLButtonElement>("#equip-fixture-item");
let latestSnapshot: OriginalFlowSnapshot | null = null;
let selectedMenuAction: MenuAction | null = null;
let hunterRosterOpen = false;
let hunterRosterPrimed = false;
let nextHunterRosterRefreshAt = 0;
let world: VisibleEntityWorld | null = null;
let runtimeStarted = false;
let panelMessageTimer: number | undefined;
const buildingContext: BuildingRenderingContext = {
  get client() { return client; },
  debugUi,
  showPanelMessage,
  renderTradingRequestEditor: () => tradePopup.renderTradingRequestEditor(),
  get latestSnapshot() { return latestSnapshot; },
  set latestSnapshot(value) { latestSnapshot = value; },
  selectedBuildingId: null,
  selectedBuildingInstanceId: null,
  selectedBuildingVisual: null,
  buildingPanelMode: "building",
  selectedRecipe: null,
  selectedServiceMaterialId: null,
  selectedServiceQuantity: 1,
  serviceTabsByBuilding: new Map<string, "production" | "hunters">(),
  gearTab: "weapon",
  blacksmithDifficultyGroup: 1,
  blacksmithCraftableOnly: false,
  gearCatalog: [],
  gearMaterialIcons: new Map<string, string>(),
  selectedEnhancementGearKey: null,
  selectedEnhancementMode: "single",
  enhancementView: "select",
  enhancementHunterId: null,
  purchaseHunterId: null,
  selectedEnhancementOptionalMaterialIds: [],
  gearPopupMode: "craft",
  selectedBountyTier: 0,
  selectedTradingPostDifficulty: 0,
  selectedTradingRequest: null,
  selectedTradingRequestQuantity: 1,
  tradingRequestPending: false,
  buildingEvidenceRegistry: null,
  buildingEvidenceError: null,
  popupInteractionActive: false,
  buildingPanel,
  buildingName,
  buildingPreview,
  buildingLevel,
  buildingFeature,
  buildingCondition,
  buildingLevelContract,
  buildingCatalog,
  buildingConstruct,
  buildingUpgrade,
  buildingUse,
  tradingRequestPop,
  tradingRequestContent,
  bountyPop,
  bountyTitle,
  bountyTierTabs,
  bountyUpgrade,
  gearCreatePop,
  gearCreateTitle,
  gearCreateIcon,
  gearCreateName,
  gearCreatePrice,
  gearCreateDescription,
  gearLock,
  gearMaterialTitle,
  gearMaterialCosts,
  gearQuantityRow,
  gearCreateQuantity,
  gearFrameQuantity,
  gearStorageLabel,
  gearCreateSubmit,
  gearCreateSell,
  consumCreatePop,
  consumCreateTitle,
  consumCreateIcon,
  consumCreateIconPlaceholder,
  consumCreateQuantity,
  consumCreateQuantityInput,
  consumConversion,
  consumMaterialTitle,
  consumMaterialGrid,
  consumCreateSubmit,
} satisfies BuildingRenderingContext;
const buildingRenderer = createBuildingRenderer(buildingContext);
const tradeContext: TradePopupContext = {
  get client() { return client; },
  get selectedTradingRequest() { return buildingContext.selectedTradingRequest; },
  set selectedTradingRequest(value) { buildingContext.selectedTradingRequest = value; },
  get selectedTradingRequestQuantity() { return buildingContext.selectedTradingRequestQuantity; },
  set selectedTradingRequestQuantity(value) { buildingContext.selectedTradingRequestQuantity = value; },
  get selectedBuildingInstanceId() { return buildingContext.selectedBuildingInstanceId; },
  set selectedBuildingInstanceId(value) { buildingContext.selectedBuildingInstanceId = value; },
  get tradingRequestPending() { return buildingContext.tradingRequestPending; },
  set tradingRequestPending(value) { buildingContext.tradingRequestPending = value; },
  tradingRequestContent,
  tradingRequestPop,
  showPanelMessage,
  resourceIconPath: (resourceId) => buildingRenderer.resourceIconPath(resourceId),
  renderBuildingSystem: (snapshot) => buildingRenderer.renderBuildingSystem(snapshot),
  get latestSnapshot() { return latestSnapshot; },
};
const tradePopup = createTradePopup(tradeContext);
let popupInteractionReleaseTimer: number | undefined;
let popupSnapshotSignature = "";
let selectedHunterId: string | null = null;
let releasedWorldHunterEntityId: string | null = null;
const hunterRosterActors = createHunterRosterActors(hunterActiveList);
let hunterRosterScrollFrame: number | null = null;
hunterActiveList.addEventListener("scroll", () => {
  if (hunterRosterScrollFrame !== null) cancelAnimationFrame(hunterRosterScrollFrame);
  hunterRosterScrollFrame = requestAnimationFrame(() => {
    hunterRosterScrollFrame = null;
    hunterRosterActors.refresh();
  });
}, { passive: true });

function setHunterRosterVisibility(open: boolean): void {
  hunterRosterOpen = open;
  selectedMenuAction = open ? "character" : selectedMenuAction === "character" ? null : selectedMenuAction;
  rosterScreen.classList.toggle("visible", open);
  rosterScreen.setAttribute("aria-hidden", String(!open));
  bottomMenu.querySelector('[data-action="character"]')?.classList.toggle("selected", open);
}

const holdPopupRender = (): void => {
  if (popupInteractionReleaseTimer !== undefined) window.clearTimeout(popupInteractionReleaseTimer);
  buildingContext.popupInteractionActive = true;
};
const releasePopupRender = (): void => {
  popupInteractionReleaseTimer = window.setTimeout(() => {
    const active = document.activeElement;
    if (active instanceof HTMLElement && active.closest(".source-popup") && active.matches("select, input, textarea")) {
      popupInteractionReleaseTimer = undefined;
      return;
    }
    buildingContext.popupInteractionActive = false;
    popupInteractionReleaseTimer = undefined;
  }, 50);
};

document.querySelectorAll<HTMLElement>(".source-popup").forEach((popup) => {
  popup.addEventListener("pointerdown", holdPopupRender, true);
  popup.addEventListener("keydown", (event) => {
    if (event.key === "Enter" || event.key === " ") holdPopupRender();
  }, true);
  popup.addEventListener("keyup", (event) => {
    if (event.key === "Enter" || event.key === " ") releasePopupRender();
  }, true);
  popup.addEventListener("focusin", (event) => {
    if ((event.target as HTMLElement).matches("select, input, textarea")) holdPopupRender();
  }, true);
  popup.addEventListener("focusout", (event) => {
    if ((event.target as HTMLElement).matches("select, input, textarea")) releasePopupRender();
  }, true);
});
window.addEventListener("pointerup", releasePopupRender, true);
window.addEventListener("pointercancel", releasePopupRender, true);

buildingPanelClose.textContent = originalUiLabel("btn_0");
gearCreateClose.textContent = originalUiLabel("btn_0");

const client = new WorldClient(
  (snapshot) => worldController.renderSnapshot(snapshot),
  (status) => entryController.updateConnectionStatus(status),
  showIntentResult,
  showBindingBlocked,
  undefined,
  { onWorldFrame: (snapshot) => worldController.renderWorldFrame(snapshot) },
);
const entryController = new EntryController(gameShell, startGameRuntime, () => client.completeBoot());
const hunterInfoActions = { useSkill: (hunterId: number, skillId: string) => hunterController.useHunterSkillFromInfo(hunterId, skillId) };
const hunterInfoModal = createHunterInfoModal(rosterScreen, hunterInfoActions);
const worldHunterInfoModal = createHunterInfoModal(villageScreen, hunterInfoActions);
const hunterWorldCommandMenu = createHunterWorldCommandMenu(villageScreen, {
  onInfo: (entityId) => hunterController.showWorldHunterInfo(entityId),
  onIntent: (intent) => hunterController.handleHunterWorldCommandIntent(intent),
  onEnhancementRequest: (intent) => hunterController.handleHunterEnhancementRequest(intent),
  onRelease: (entityId) => {
    releasedWorldHunterEntityId = entityId;
    world?.setSelectedEntity(null);
    worldHunterInfoModal.close();
  },
  onUnavailable: (category) => showPanelMessage(t("error.command_unbound"), category),
});
const hunterContext: HunterControllerContext = {
  client,
  get latestSnapshot() { return latestSnapshot; },
  set latestSnapshot(value) { latestSnapshot = value; },
  get selectedHunterId() { return selectedHunterId; },
  set selectedHunterId(value) { selectedHunterId = value; },
  get releasedWorldHunterEntityId() { return releasedWorldHunterEntityId; },
  set releasedWorldHunterEntityId(value) { releasedWorldHunterEntityId = value; },
  hunterRosterActors,
  hunterCapacity,
  hunterActiveList,
  hunterRosterStatus,
  hunterInfoModal,
  worldHunterInfoModal,
  hunterWorldCommandMenu,
  get world() { return world; },
  set world(value) { world = value; },
  worldViewport,
  originalAsset,
  buildingContext,
  setHunterRosterVisibility,
  showPanelMessage,
  openHunterShop: (hunterId, shopId) => {
    const instance = latestSnapshot?.village.building_system.instances.find((row) => row.building_id === shopId);
    if (!instance) {
      showPanelMessage(t("error.building_missing"), shopId);
      return;
    }
    buildingContext.purchaseHunterId = hunterId;
    buildingContext.selectedBuildingId = shopId;
    buildingContext.selectedBuildingInstanceId = instance.instance_id;
    buildingContext.selectedBuildingVisual = null;
    buildingContext.buildingPanelMode = "building";
    buildingContext.buildingPanel.hidden = false;
    buildingRenderer.renderBuildingSystem(latestSnapshot);
  },
};
const hunterController = createHunterController(hunterContext);
let worldController: ReturnType<typeof createWorldController>;
const combatHudController = createCombatHudController(debugUi, combatHud, equipFixtureItem);
const worldContext: WorldControllerContext = {
  client,
  get latestSnapshot() { return latestSnapshot; },
  set latestSnapshot(value) { latestSnapshot = value; },
  get releasedWorldHunterEntityId() { return releasedWorldHunterEntityId; },
  set releasedWorldHunterEntityId(value) { releasedWorldHunterEntityId = value; },
  hunterController,
  hunterWorldCommandMenu,
  worldHunterInfoModal,
  get world() { return world; },
  set world(value) { world = value; },
  buildingContext,
  buildingRenderer,
  entryController,
  hunterRosterActors,
  hunterInfoModal,
  worldViewport,
  showPanelMessage,
  debugUi,
  evidenceDiagnostics,
  fpsCounter,
  hunterEnhancementInteractions,
  get hunterRosterOpen() { return hunterRosterOpen; },
  get hunterRosterPrimed() { return hunterRosterPrimed; },
  set hunterRosterPrimed(value) { hunterRosterPrimed = value; },
  get nextHunterRosterRefreshAt() { return nextHunterRosterRefreshAt; },
  set nextHunterRosterRefreshAt(value) { nextHunterRosterRefreshAt = value; },
  get selectedMenuAction() { return selectedMenuAction; },
  set selectedMenuAction(value) { selectedMenuAction = value as MenuAction | null; },
  bottomMenu,
  worldModeLabel,
  goldAmount,
  hunterPopulation,
  fieldBack,
  get popupSnapshotSignature() { return popupSnapshotSignature; },
  set popupSnapshotSignature(value) { popupSnapshotSignature = value; },
  get popupInteractionActive() { return buildingContext.popupInteractionActive; },
  gearCreatePop,
  consumCreatePop,
  renderCombatHud: combatHudController.render,
  syncEnhancementTaskView: (snapshot) => worldController.syncEnhancementTaskView(snapshot),
};
worldController = createWorldController(worldContext);
document.addEventListener("contextmenu", (event) => {
  const target = event.target as HTMLElement | null;
  if (target?.closest("input, textarea, [contenteditable=\"true\"]")) return;
  event.preventDefault();
});
document.addEventListener("selectstart", (event) => {
  const target = event.target as HTMLElement | null;
  if (!target?.closest("input, textarea, [contenteditable=\"true\"]")) event.preventDefault();
});
document.addEventListener("copy", (event) => {
  const target = event.target as HTMLElement | null;
  if (!target?.closest("input, textarea, [contenteditable=\"true\"]")) event.preventDefault();
});
document.addEventListener("cut", (event) => {
  const target = event.target as HTMLElement | null;
  if (!target?.closest("input, textarea, [contenteditable=\"true\"]")) event.preventDefault();
});
function showPanelMessage(title: string, detail: string): void {
  setPanelMessage(panelMessage, title, detail);
  panelMessage.hidden = false;
  if (panelMessageTimer !== undefined) window.clearTimeout(panelMessageTimer);
  panelMessageTimer = window.setTimeout(() => { panelMessage.hidden = true; panelMessageTimer = undefined; }, 2800);
}

function showIntentResult(result: IntentFeedback): void {
  if (result.intent === "set_material_request") {
    buildingContext.tradingRequestPending = false;
    if (result.accepted) {
      buildingContext.selectedTradingRequest = null;
      tradingRequestPop.hidden = true;
      buildingRenderer.renderBuildingSystem(latestSnapshot);
    } else if (buildingContext.selectedTradingRequest) {
      tradePopup.renderTradingRequestEditor();
    }
  }
  if (!result.accepted) {
    recordClientEvent("warn", "intent_rejected", { intent: result.intent, reason: result.reason });
    const reasons: Record<string, MessageKey> = {
      insufficient_materials: "error.insufficient_materials", material_stock_missing: "error.material_stock_missing", recipe_unknown: "error.recipe_unknown", recipe_building_mismatch: "error.recipe_building_mismatch", product_level_locked: "error.product_level_locked", sale_building_instance_unknown: "error.sale_building_missing", product_capacity_exceeded: "error.product_capacity", product_stock_empty: "error.product_empty", sale_price_unresolved: "error.sale_price_unresolved", building_instance_unknown: "error.building_missing", building_capability_mismatch: "error.capability_mismatch", material_difficulty_unresolved: "error.material_difficulty_unresolved", material_difficulty_locked: "error.material_difficulty_locked", material_quantity_invalid: "error.material_quantity_invalid", material_price_unresolved: "error.material_price_unresolved",
    };
    const titles: Record<string, MessageKey> = { select_bottom_menu: "error.cannot_open_menu", navigate_back: "error.cannot_navigate_back", enter_field: "error.cannot_enter_field", select_entity: "error.cannot_select_entity", set_material_request: "error.cannot_request" };
    const reasonKey = reasons[result.reason ?? ""];
    const detail = reasonKey ? t(reasonKey) : debugUi && result.reason ? `${t("error.try_again")} (${result.reason})` : t("error.try_again");
    showPanelMessage(t(titles[result.intent] ?? "error.cannot_craft"), detail);
  }
}
function showBindingBlocked(result: BindingBlockedFeedback): void {
  showPanelMessage(t("error.coming_soon"), debugUi ? `${result.intent.replaceAll("_", " ")} · ${result.blockers.join(", ")}` : t("error.feature_rebuilding"));
}
async function initializeBuildingEvidence(): Promise<void> {
  try {
    const [registry, decodedCatalog] = await Promise.all([loadVerifiedBuildingEvidenceRegistry(), loadGearCatalog().catch(() => null)]);
    buildingContext.buildingEvidenceRegistry = registry;
    buildingContext.gearCatalog = decodedCatalog ?? decodeGearCatalog(registry.catalogs.items.rows, registry.catalogs.products.rows);
    buildingContext.gearMaterialIcons.clear();
    for (const recipe of buildingContext.gearCatalog) for (const cost of recipe.materialCosts) if (cost.iconPath) buildingContext.gearMaterialIcons.set(cost.materialId, cost.iconPath);
    buildingContext.buildingEvidenceError = null;
    if (world && latestSnapshot) buildingRenderer.syncBuildingPresentation(world, latestSnapshot);
    buildingRenderer.renderBuildingSystem(latestSnapshot);
  } catch (error) {
    recordClientEvent("error", "building_evidence_load_failed", { reason: error instanceof Error ? error.message : "unknown" });
    buildingContext.buildingEvidenceError = debugUi && error instanceof Error ? error.message : t("diagnostics.building_load_failed");
    console.error("Failed to load verified building evidence.", error);
  }
}

function startGameRuntime(): void {
  if (runtimeStarted) return;
  runtimeStarted = true;
  document.querySelectorAll<HTMLImageElement>("img[data-game-src]").forEach((image) => {
    image.src = image.dataset.gameSrc ?? "";
    delete image.dataset.gameSrc;
  });
  client.connect();
  void initializeBuildingEvidence();
  void worldController.initializeWorld().catch((error: unknown) => {
    console.error("Failed to initialize the visible world.", error);
    entryController.fail(t("loading.world_failure"));
  });
}

function handleMenuAction(button: HTMLButtonElement): void {
  hunterWorldCommandMenu.close();
  worldHunterInfoModal.close();
  const action = button.dataset.action as MenuAction;
  const triggerIsBottomMenu = button.closest(".bottom-menu") !== null;
  if (action === "character") {
    const open = nextHunterRosterOpen(triggerIsBottomMenu, hunterRosterOpen);
    setHunterRosterVisibility(open);
    if (open && latestSnapshot && !hunterRosterPrimed) {
      hunterController.renderHunterRoster(latestSnapshot);
      hunterRosterPrimed = true;
    }
    return;
  }
  const togglesActiveBottomTab = triggerIsBottomMenu && selectedMenuAction === action;
  if (togglesActiveBottomTab) {
    selectedMenuAction = null;
    document.querySelectorAll(`.bottom-menu [data-action="${action}"]`).forEach((item) => item.classList.remove("selected"));
    if (action === "build") {
      buildingPanel.hidden = true;
    } else if (action === "field" && latestSnapshot?.screen === "field") {
      client.navigateBack();
    }
    return;
  }
  document.querySelectorAll(".bottom-menu [data-action]").forEach((item) => item.classList.remove("selected"));
  document.querySelectorAll(`.bottom-menu [data-action="${button.dataset.action}"]`).forEach((item) => item.classList.add("selected"));
  selectedMenuAction = action;
  if (hunterRosterOpen || rosterScreen.classList.contains("visible")) setHunterRosterVisibility(false);
  if (action === "build") {
    buildingContext.buildingPanelMode = "construct";
    buildingPanel.hidden = false;
    client.selectBottomMenu("build");
    buildingRenderer.renderBuildingSystem(latestSnapshot);
  } else if (action === "field") client.enterField();
  else client.selectBottomMenu(action);
}

// Delegate the persistent bar so its controls survive DOM refreshes/HMR without stale listeners.
bottomMenu.addEventListener("click", (event) => {
const target = event.target as HTMLElement | null;
const button = target?.closest<HTMLButtonElement>("button[data-action]");
if (!button || !bottomMenu.contains(button) || button.disabled) return;
handleMenuAction(button);
  });
  document.querySelectorAll<HTMLButtonElement>("[data-action]").forEach((button) => {
if (!button.closest(".bottom-menu")) button.addEventListener("click", () => handleMenuAction(button));
  });
  bindOverlayCloseControls([
    { overlay: "hunter-roster", controls: [rosterBack], close: () => setHunterRosterVisibility(false) },
    {
      overlay: "building-panel",
      controls: [buildingPanelClose],
      close: () => {
        buildingPanel.hidden = true;
        tradingRequestPop.hidden = true;
        buildingContext.selectedTradingRequest = null;
        buildingContext.tradingRequestPending = false;
        buildingContext.enhancementView = "select";
        buildingContext.selectedEnhancementGearKey = null;
        buildingContext.enhancementHunterId = null;
        buildingContext.selectedEnhancementOptionalMaterialIds = [];
        if (selectedMenuAction === "build") selectedMenuAction = null;
        bottomMenu.querySelector('[data-action="build"]')?.classList.remove("selected");
      },
    },
    { overlay: "bounty-quest", controls: [bountyClose, bountyCloseBottom], close: () => { bountyPop.hidden = true; } },
    { overlay: "gear-create", controls: [gearCreateClose], close: () => { gearCreatePop.hidden = true; } },
    { overlay: "consumable-create", controls: [consumCreateClose], close: () => { consumCreatePop.hidden = true; } },
  ]);
  buildingConstruct.addEventListener("click", () => { if (buildingContext.selectedBuildingId) client.constructBuilding(buildingContext.selectedBuildingId); });
  buildingUpgrade.addEventListener("click", () => { if (buildingContext.selectedBuildingInstanceId) client.upgradeBuilding(buildingContext.selectedBuildingInstanceId); });
  buildingUse.addEventListener("click", () => {
if (!buildingContext.selectedBuildingId || !buildingContext.selectedBuildingInstanceId) return;
const route = projectBuildingEvidence(buildingContext.buildingEvidenceRegistry, buildingContext.selectedBuildingId)?.popupRoute ?? null;
if (buildingContext.selectedBuildingId === BOUNTY_HUT_ROUTE.buildingId) {
  bountyPop.hidden = false;
  buildingRenderer.renderBountyPop();
} else if (buildingContext.selectedBuildingId === TRADING_POST_ROUTE.buildingId || route === "request") {
  buildingRenderer.renderBuildingSystem(latestSnapshot);
} else if (route === "gear-enhancement") {
  const selected = latestSnapshot?.hunter_roster.active_hunters.flatMap((hunter) => (
    hunter.gear_enhancements.map((gear) => ({ hunter, gear, key: gear.instance_id ?? `${hunter.hunter_id}:${gear.product_id}` }))
  )).find((row) => row.key === buildingContext.selectedEnhancementGearKey);
  if (!selected) {
    showPanelMessage(t("enhancement.no_selection_title"), t("enhancement.no_selection_detail"));
    return;
  }
  if (buildingContext.enhancementView === "select") {
    buildingContext.enhancementView = "configure";
    buildingRenderer.renderBuildingSystem(latestSnapshot);
    return;
  }
  const preview = projectGearEnhancement(selected.gear, buildingContext.selectedEnhancementMode);
  if (!canSubmitGearEnhancement(preview)) {
    showPanelMessage(t("enhancement.unavailable_title"), t("enhancement.evidence_unverified"));
    return;
  }
  if (!selected.gear.instance_id) {
    showPanelMessage(t("enhancement.unavailable_title"), t("enhancement.instance_unavailable"));
    return;
  }
  client.enhanceHunterGear(selected.hunter.hunter_id, selected.gear.instance_id, buildingContext.selectedEnhancementMode, buildingContext.selectedEnhancementOptionalMaterialIds);
} else if (route === "production") {
  buildingRenderer.renderBuildingSystem(latestSnapshot);
} else if (route === "service") {
  const recipe = latestSnapshot?.village.building_system.recipes.find((item) => item.shop_id === buildingContext.selectedBuildingId);
  if (recipe) client.craftShopItem(buildingContext.selectedBuildingInstanceId, recipe.id, 1);
}
  });
  bountyUpgrade.addEventListener("click", () => { if (buildingContext.selectedBuildingInstanceId) client.upgradeBuilding(buildingContext.selectedBuildingInstanceId); });

gearCreateQuantity.addEventListener("input", () => {
  if (gearCreateQuantity.value === "") return;
  gearCreateQuantity.value = String(clampQuantity(gearCreateQuantity.value, 1, 1000));
  buildingRenderer.renderGearCreatePop();
});
gearCreateQuantity.addEventListener("change", () => {
  gearCreateQuantity.value = String(clampQuantity(gearCreateQuantity.value, 1, 1000));
  buildingRenderer.renderGearCreatePop();
});
function changeGearQuantity(delta: number): void {
  gearCreateQuantity.value = String(clampQuantity(Number(gearCreateQuantity.value) + delta, 1, 1000));
  buildingRenderer.renderGearCreatePop();
}
element<HTMLButtonElement>("#gear-quantity-minus").addEventListener("click", () => changeGearQuantity(-1));
element<HTMLButtonElement>("#gear-quantity-plus").addEventListener("click", () => changeGearQuantity(1));
document.querySelectorAll<HTMLButtonElement>("[data-gear-delta]").forEach((button) => {
  button.addEventListener("click", () => changeGearQuantity(Number(button.dataset.gearDelta)));
});
gearCreateSubmit.addEventListener("click", () => {
  if (buildingContext.selectedBuildingInstanceId && buildingContext.selectedRecipe) {
    client.craftShopItem(buildingContext.selectedBuildingInstanceId, buildingContext.selectedRecipe.id, Number(gearCreateQuantity.value), buildingContext.selectedServiceMaterialId);
  }
});
gearCreateSell.addEventListener("click", () => {
  if (buildingContext.gearPopupMode !== "detail"
    || buildingContext.purchaseHunterId === null
    || !buildingContext.selectedBuildingId
    || !buildingContext.selectedRecipe) return;
  client.purchaseShopItem(
    buildingContext.purchaseHunterId,
    buildingContext.selectedBuildingId,
    buildingContext.selectedRecipe.id,
  );
});
gearLock.addEventListener("click", () => {});
function changeServiceQuantity(delta: number): void {
  buildingContext.selectedServiceQuantity = clampQuantity(buildingContext.selectedServiceQuantity + delta, 1, 1000);
  buildingRenderer.renderConsumCreatePop();
}
consumMinus.addEventListener("click", () => changeServiceQuantity(-1));
consumPlus.addEventListener("click", () => changeServiceQuantity(1));
document.querySelectorAll<HTMLButtonElement>("[data-consum-delta]").forEach((button) => {
  button.addEventListener("click", () => changeServiceQuantity(Number(button.dataset.consumDelta)));
});
consumCreateQuantityInput.addEventListener("input", () => {
  if (consumCreateQuantityInput.value === "") return;
  buildingContext.selectedServiceQuantity = clampQuantity(consumCreateQuantityInput.value, 1, 1000);
  buildingRenderer.renderConsumCreatePop();
});
consumCreateQuantityInput.addEventListener("change", () => {
  buildingContext.selectedServiceQuantity = clampQuantity(consumCreateQuantityInput.value, 1, 1000);
  buildingRenderer.renderConsumCreatePop();
});
consumCreateSubmit.addEventListener("click", () => {
  if (!buildingContext.selectedBuildingInstanceId || !buildingContext.selectedRecipe) return;
  const materialId = buildingContext.selectedRecipe.kind === "service" ? buildingContext.selectedServiceMaterialId : null;
  if (buildingContext.selectedRecipe.kind !== "service" || materialId) {
    client.craftShopItem(buildingContext.selectedBuildingInstanceId, buildingContext.selectedRecipe.id, buildingContext.selectedServiceQuantity, materialId);
  }
});
