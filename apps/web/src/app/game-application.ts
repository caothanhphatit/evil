import { WorldClient, type BindingBlockedFeedback } from "../net/world-client";
import type { OriginalFlowSnapshot } from "../generated/protocol";
import { VisibleEntityWorld } from "../game/visible-world";
import { nextHunterRosterOpen } from "../ui/bottom-menu-state";
import { createCombatHudController } from "./combat-hud-controller";
import { setPanelMessage } from "../ui/panel-message";
import { createHunterRosterActors } from "../ui/hunter-roster-actors";
import { createHunterInfoModal } from "../ui/hunter-info/modal";
import { createHunterWorldCommandMenu } from "../ui/hunter-world-command";
import { canSubmitGearEnhancement, projectGearEnhancement } from "../ui/gear-enhancement";
import { clampQuantity } from "../ui/shop-crafting";
import { projectBuildingEvidence } from "../content/building-evidence";
import { originalUiLabel } from "../content/original-ui-labels";
import { BOUNTY_HUT_ROUTE } from "../routes/bounty-hut";
import { TRADING_POST_ROUTE } from "../routes/trading-post";
import { t } from "../i18n";
import { mountGameShell, originalAsset, type MenuAction } from "./shell";
import { EntryController } from "./entry-controller";
import { createBuildingRenderer, type BuildingRenderingContext } from "./building-renderer";
import { createHunterController, type HunterControllerContext } from "./hunter-controller";
import { createWorldController, type WorldControllerContext } from "./world-controller";
import { createTradePopup, type TradePopupContext } from "./trade-popup";
import { installGameE2eHooks } from "./e2e-hooks";
import { bindBuildingControls, bindCraftInteractions, bindInteractionGuards, bindMenuInteraction, bindOverlayInteractions, bindPopupInteractionGuards } from "./interaction-bindings";
import { initializeBuildingEvidence } from "./building-evidence-loader";
import { showIntentResult as renderIntentResult } from "./intent-feedback";
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
let craftAnimationTimer: number | undefined;
type PendingCraft = { popup: "gear" | "consumable"; recipeId: string };
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
  selectedShopGearInstanceId: null,
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
  pendingCraft: null,
  pendingPurchase: null,
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

bindPopupInteractionGuards(document.querySelectorAll<HTMLElement>(".source-popup"), holdPopupRender, releasePopupRender);

buildingPanelClose.textContent = originalUiLabel("btn_0");
gearCreateClose.textContent = originalUiLabel("btn_0");

const client = new WorldClient(
  (snapshot) => worldController.renderSnapshot(snapshot),
  (status) => {
    entryController.updateConnectionStatus(status);
    if (status !== "online" && (buildingContext.pendingCraft || buildingContext.pendingPurchase)) {
      buildingContext.pendingCraft = null;
      buildingContext.pendingPurchase = null;
      buildingRenderer.renderGearCreatePop();
      buildingRenderer.renderConsumCreatePop();
    }
  },
  showIntentResult,
  showBindingBlocked,
  undefined,
  { onWorldFrame: (snapshot) => worldController.renderWorldFrame(snapshot) },
);
const entryController = new EntryController(gameShell, startGameRuntime, () => client.completeBoot());
const hunterInfoActions = {
  useSkill: (hunterId: number, skillId: string) => hunterController.useHunterSkillFromInfo(hunterId, skillId),
  equipWeapon: (hunterId: number, gearInstanceId: string) => hunterController.equipHunterWeaponFromInfo(hunterId, gearInstanceId),
};
const hunterInfoModal = createHunterInfoModal(rosterScreen, hunterInfoActions);
const worldHunterInfoModal = createHunterInfoModal(villageScreen, hunterInfoActions);
function openHunterShop(hunterId: number, shopId: "build_7" | "build_8" | "build_20"): boolean {
  const instance = latestSnapshot?.village.building_system.instances.find((row) => row.building_id === shopId);
  if (!instance) {
    showPanelMessage(t("error.building_missing"), shopId);
    return false;
  }
  buildingContext.purchaseHunterId = hunterId;
  buildingContext.selectedBuildingId = shopId;
  buildingContext.selectedBuildingInstanceId = instance.instance_id;
  buildingContext.selectedBuildingVisual = null;
  buildingContext.buildingPanelMode = "building";
  buildingContext.buildingPanel.hidden = false;
  buildingRenderer.renderBuildingSystem(latestSnapshot);
  return true;
}
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
  openHunterShop: (hunterId, shopId) => { openHunterShop(hunterId, shopId); },
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
installGameE2eHooks(window.location, {
  snapshot: () => latestSnapshot,
  openBuilding: (buildingId) => {
    const instance = latestSnapshot?.village.building_system.instances.find((row) => row.building_id === buildingId);
    if (!instance) return false;
    buildingContext.selectedBuildingId = buildingId;
    buildingContext.selectedBuildingInstanceId = instance.instance_id;
    buildingContext.selectedBuildingVisual = null;
    buildingContext.buildingPanelMode = "building";
    buildingPanel.hidden = false;
    buildingRenderer.renderBuildingSystem(latestSnapshot);
    return true;
  },
  openHunterShop,
  openHunterInfo: (hunterId) => hunterController.showHunterInfoByNumericId(hunterId),
});
bindInteractionGuards();
function showPanelMessage(title: string, detail: string): void {
  setPanelMessage(panelMessage, title, detail);
  panelMessage.hidden = false;
  if (panelMessageTimer !== undefined) window.clearTimeout(panelMessageTimer);
  panelMessageTimer = window.setTimeout(() => { panelMessage.hidden = true; panelMessageTimer = undefined; }, 2800);
}

function showIntentResult(result: Parameters<typeof renderIntentResult>[0]): void {
  renderIntentResult(result, {
    context: buildingContext, gearPopup: gearCreatePop, consumablePopup: consumCreatePop,
    gearSubmit: gearCreateSubmit, consumableSubmit: consumCreateSubmit,
    renderGear: () => buildingRenderer.renderGearCreatePop(),
    renderConsumable: () => buildingRenderer.renderConsumCreatePop(),
    renderBuilding: () => buildingRenderer.renderBuildingSystem(latestSnapshot),
    renderTradeRequest: () => tradePopup.renderTradingRequestEditor(),
    clearTradingRequest: () => { buildingContext.selectedTradingRequest = null; tradingRequestPop.hidden = true; },
    showMessage: showPanelMessage, debugUi,
    setAnimationTimer: (timer) => { craftAnimationTimer = timer; },
    getAnimationTimer: () => craftAnimationTimer,
  });
}

function showBindingBlocked(result: BindingBlockedFeedback): void {
  showPanelMessage(t("error.coming_soon"), debugUi ? `${result.intent.replaceAll("_", " ")} · ${result.blockers.join(", ")}` : t("error.feature_rebuilding"));
}
function startGameRuntime(): void {
  if (runtimeStarted) return;
  runtimeStarted = true;
  document.querySelectorAll<HTMLImageElement>("img[data-game-src]").forEach((image) => {
    image.src = image.dataset.gameSrc ?? "";
    delete image.dataset.gameSrc;
  });
  client.connect();
  void initializeBuildingEvidence({
    context: buildingContext,
    world,
    snapshot: latestSnapshot,
    debugUi,
    render: (snapshot) => buildingRenderer.renderBuildingSystem(snapshot),
    sync: (visibleWorld, snapshot) => buildingRenderer.syncBuildingPresentation(visibleWorld, snapshot),
    fallbackMessage: t("diagnostics.building_load_failed"),
  });
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
bindMenuInteraction(bottomMenu, handleMenuAction);
  bindOverlayInteractions({ rosterBack, buildingPanelClose, bountyClose, bountyCloseBottom, gearCreateClose, consumCreateClose,
    closeRoster: () => setHunterRosterVisibility(false),
    closeBuilding: () => {
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
    closeBounty: () => { bountyPop.hidden = true; },
    closeGear: () => {
      gearCreatePop.hidden = true;
      if (buildingContext.gearPopupMode !== "craft" && buildingContext.selectedBuildingId) {
        buildingPanel.hidden = false;
        buildingRenderer.renderBuildingSystem(latestSnapshot);
      }
    },
    closeConsumable: () => { consumCreatePop.hidden = true; },
  });
  bindBuildingControls({
    buildingConstruct,
    buildingUpgrade,
    bountyUpgrade,
    buildingId: () => buildingContext.selectedBuildingId,
    instanceId: () => buildingContext.selectedBuildingInstanceId,
    construct: (id) => client.constructBuilding(id),
    upgrade: (id) => client.upgradeBuilding(id),
    upgradeBounty: (id) => client.upgradeBuilding(id),
  });
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

bindCraftInteractions({
  gearCreateQuantity,
  gearCreateSubmit,
  gearCreateSell,
  gearLock,
  consumCreateQuantityInput,
  consumCreateSubmit,
  consumMinus,
  consumPlus,
  gearDeltaButtons: document.querySelectorAll<HTMLButtonElement>("[data-gear-delta]"),
  consumDeltaButtons: document.querySelectorAll<HTMLButtonElement>("[data-consum-delta]"),
  clampQuantity,
  renderGear: () => buildingRenderer.renderGearCreatePop(),
  renderConsumable: () => buildingRenderer.renderConsumCreatePop(),
  getServiceQuantity: () => buildingContext.selectedServiceQuantity,
  setServiceQuantity: (quantity) => { buildingContext.selectedServiceQuantity = quantity; },
  canCraft: () => !buildingContext.pendingCraft && Boolean(buildingContext.selectedBuildingInstanceId && buildingContext.selectedRecipe),
  craftGear: (quantity) => {
    if (!buildingContext.selectedBuildingInstanceId || !buildingContext.selectedRecipe) return false;
    const pending: PendingCraft = { popup: "gear", recipeId: buildingContext.selectedRecipe.id };
    if (!client.craftShopItem(buildingContext.selectedBuildingInstanceId, pending.recipeId, quantity, buildingContext.selectedServiceMaterialId)) return false;
    buildingContext.pendingCraft = pending;
    return true;
  },
  craftConsumable: (quantity) => {
    if (!buildingContext.selectedBuildingInstanceId || !buildingContext.selectedRecipe) return false;
    const materialId = buildingContext.selectedRecipe.kind === "service" ? buildingContext.selectedServiceMaterialId : null;
    if (buildingContext.selectedRecipe.kind === "service" && !materialId) return false;
    const pending: PendingCraft = { popup: "consumable", recipeId: buildingContext.selectedRecipe.id };
    if (!client.craftShopItem(buildingContext.selectedBuildingInstanceId, pending.recipeId, quantity, materialId)) return false;
    buildingContext.pendingCraft = pending;
    return true;
  },
  sellGear: () => {
    if (buildingContext.gearPopupMode !== "purchase" || buildingContext.purchaseHunterId === null || !buildingContext.selectedBuildingId || !buildingContext.selectedRecipe) return;
    const pending = {
      shopId: buildingContext.selectedBuildingId,
      productId: buildingContext.selectedRecipe.id,
      gearInstanceId: buildingContext.selectedShopGearInstanceId,
    };
    if (!client.purchaseShopItem(buildingContext.purchaseHunterId, pending.shopId, pending.productId, pending.gearInstanceId)) return;
    buildingContext.pendingPurchase = pending;
    buildingRenderer.renderGearCreatePop();
  },
  setGearLocked: () => {},
});
