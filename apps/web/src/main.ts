import { Application } from "pixi.js";
import { WorldClient, type BindingBlockedFeedback, type ConnectionStatus, type IntentFeedback } from "./net/world-client";
import type { BottomMenuIntent, MaterialStockSnapshot, OriginalFlowSnapshot, ShopRecipeSnapshot } from "./generated/protocol";
import { VisibleEntityWorld } from "./game/visible-world";
import type { TownBuilding } from "./assets/visible-world-release";
import { findBuildingInstanceById } from "./game/building-placement";
import { TOWN_CAMERA_CLEAR_COLOR } from "./game/scene-projection";
import { projectCombatHud, type CombatHudState } from "./ui/combat-hud";
import { projectResourceBar } from "./ui/resource-projection";
import { setPanelMessage } from "./ui/panel-message";
import { projectHunterRoster, type HunterView } from "./ui/hunter-roster";
import { createHunterInfoModal } from "./ui/hunter-info/modal";
import { projectHunterInfo } from "./ui/hunter-info/project";
import { formatLevelCosts, listBuildingEvidence, projectBuildingEvidence } from "./content/building-evidence";
import { loadVerifiedBuildingEvidenceRegistry, type EvidenceBuildingRegistry } from "./content/building-registry";
import { originalUiLabel } from "./content/original-ui-labels";
import { BOUNTY_HUT_ROUTE, BOUNTY_TIERS } from "./routes/bounty-hut";
import { TRADING_POST_ROUTE, tradingPostDifficultyOptions, tradingPostStocksForDifficulty } from "./routes/trading-post";
import { productServiceSprite, projectProductService, productServiceRoute, type ProductServiceInput } from "./content/product-service-routes";
import { ACCESSORY_SHOP_BUILDING_IDS, ALL_GEAR_KINDS, ARMOR_SHOP_BUILDING_IDS, BLACKSMITH_BUILDING_IDS, BLACKSMITH_GEAR_TABS, JEWELER_BUILDING_IDS, JEWELER_GEAR_TABS, WEAPON_SHOP_BUILDING_IDS, decodeGearCatalog, loadGearCatalog, type GearCatalogRecipe, type GearKind } from "./content/blacksmith-route";
import { ALCHEMIST_BUILDING_ID, POTION_SHOP_BUILDING_ID, isPotionBuilding } from "./content/potion-shop-routes";
import "./styles.css";

const mount = document.querySelector<HTMLDivElement>("#app");
if (!mount) throw new Error("Missing #app mount point");

const releaseRoot = "/content/releases/original-flow-v1";
const originalAsset = (sourcePath: string): string => `${releaseRoot}/${sourcePath}`;
const debugUi = new URLSearchParams(window.location.search).has("debug");
type MenuAction = BottomMenuIntent | "field";
const menuItems: Array<{ action: MenuAction; label: string; icon: string; enabled: boolean }> = [
  { action: "build", label: "Construct", icon: "sprites/menu_ic_01__6756.png", enabled: true },
  { action: "field", label: "Dungeon", icon: "sprites/menu_ic_02__2060.png", enabled: true },
  { action: "character", label: "Hunters", icon: "sprites/menu_ic_03__6410.png", enabled: true },
  { action: "archive", label: "Storage", icon: "sprites/menu_ic_04__5070.png", enabled: false },
  { action: "store", label: "Shop", icon: "sprites/menu_ic_05__6398.png", enabled: true },
];

mount.innerHTML = `
  <main class="game-shell">
    <section id="boot-screen" class="boot-screen" aria-label="Game intro">
      <img class="boot-background" src="${originalAsset("sprites/intro_bg_new__1695.png")}" alt="" />
      <div class="boot-vignette"></div>
      <img class="boot-logo" src="${originalAsset("sprites/intro_img_glo_new__2141.png")}" alt="Evil Hunter Tycoon" />
      <div class="map-loading" aria-live="polite"><div class="map-loading-track"><i id="map-loading-fill"></i></div><span id="map-loading-label">Loading map 0%</span></div>
      <button id="enter-village" class="enter-village" type="button"><img src="${originalAsset("sprites/intro_glo_touchtostart__7172.png")}" alt="Touch to start" /></button>
      <span id="boot-status" class="boot-status" role="status">Connecting to server...</span>
    </section>
    <section id="village-screen" class="village-screen" aria-label="Village" aria-hidden="true">
      <div id="world-viewport" class="village-world" aria-label="Authoritative entity world"></div>
      <header class="resource-bar" aria-label="Village resources">
        <div class="difficulty-hud"><small>DIFFICULTY</small><span aria-hidden="true">&#9760;</span><b id="world-mode-label">Easy</b></div>
        <div class="resource-ledger">
          <div class="resource-line"><img src="${originalAsset("sprites/top_ic_01_gold_24__4677.png")}" alt="Gold" /><b id="gold-amount">0</b></div>
          <div class="resource-line unresolved"><img src="${originalAsset("sprites/top_ic_02_gem_24__4214.png")}" alt="Gem" /><b>--</b></div>
          <div class="resource-line unresolved"><i class="resource-rune" aria-hidden="true"></i><b>--</b></div>
        </div>
      </header>
      <button id="field-back" class="field-back" type="button">Return to village</button>
      <aside id="evidence-diagnostics" class="evidence-diagnostics" aria-live="polite" hidden><b></b><span></span></aside>
      <section id="combat-hud" class="combat-hud" aria-label="Authoritative migration fixture combat" hidden>
        <header><b>Migration fixture combat</b><span id="combat-evidence"></span></header>
        <div class="combatant"><span>Hunter #1 · <i id="hunter-state"></i> · <i id="hunter-position"></i></span><div class="hp-track"><i id="hunter-hp-fill"></i></div><small id="hunter-hp"></small></div>
        <div class="combatant"><span>Monster #1001 · <i id="monster-state"></i> · <i id="monster-position"></i></span><div class="hp-track monster"><i id="monster-hp-fill"></i></div><small id="monster-hp"></small></div>
        <div class="combat-ledger"><span id="combat-gold"></span><span id="combat-inventory"></span><span id="combat-drops"></span></div>
        <button id="equip-fixture-item" type="button">Equip item 2001</button>
      </section>
      <div id="panel-message" class="panel-message" aria-live="polite" hidden><b></b><span></span></div>
      <section id="building-panel" class="building-panel source-popup" hidden aria-label="BuildingPop">
        <button id="building-panel-close" class="source-red-button" type="button">Close</button>
        <b id="building-name">Building</b>
        <div class="building-source-header"><img id="building-preview" alt="" hidden /><div><span id="building-level"></span><small id="building-feature"></small><small id="building-condition"></small></div></div>
        <div id="building-level-contract" class="building-level-contract"></div>
        <div id="building-catalog" class="building-catalog"></div>
        <div class="building-actions"><button id="building-construct" class="source-green-button" type="button" disabled>Construct</button><button id="building-upgrade" class="source-green-button" type="button" disabled>Upgrade</button><button id="building-use" class="source-green-button" type="button" disabled>Open</button><button id="building-sell" type="button" hidden>Sell</button></div>
      </section>
      <section id="bounty-quest-pop" class="bounty-quest-pop source-popup" hidden aria-label="QuestPop">
        <button id="bounty-close" class="source-red-button" type="button">Close</button>
        <b id="bounty-title">Lv.1 Bounty Hut</b><i class="source-popup-line"></i>
        <p class="bounty-description">Provides bounty quests to help hunters level up fast</p>
        <div id="bounty-tier-tabs" class="bounty-tier-tabs"></div>
        <div id="bounty-quest-list" class="bounty-quest-list"><p class="bounty-unresolved">Quest rows are waiting for the authoritative QuestPop data binding.</p></div>
        <div class="bounty-upgrade-hint">When Upgraded to Lv.2 Adds bounty quest list of [Normal] difficulty<br /><em>Town Hall Lv.5 or higher required.</em></div>
        <div class="source-popup-actions"><button id="bounty-upgrade" class="source-green-button" type="button">Upgrade</button><button id="bounty-close-bottom" class="source-red-button" type="button">Close</button></div>
      </section>
      <section id="gear-create-pop" class="gear-create-pop source-popup" hidden aria-label="GearCreatePop">
        <b id="gear-create-title">Gear Create</b><i class="source-popup-line"></i>
        <div class="gear-create-product"><button id="gear-quantity-minus" class="gear-round-button minus" type="button" aria-label="Decrease quantity"></button><div class="gear-frame"><img id="gear-create-icon" alt="" /><i aria-hidden="true"></i><output id="gear-frame-quantity">1</output></div><button id="gear-quantity-plus" class="gear-round-button plus" type="button" aria-label="Increase quantity"></button><div class="gear-create-meta"><strong id="gear-create-name"></strong><span id="gear-create-price"></span></div><div class="gear-step-buttons"><button type="button" data-gear-delta="-10">-10</button><button type="button" data-gear-delta="10">+10</button><button type="button" data-gear-delta="100">+100</button><button type="button" data-gear-delta="1000">+1000</button></div></div>
        <div id="gear-create-description" class="gear-create-description"><button id="gear-lock" type="button" disabled>Lock</button></div>
        <h3 id="gear-material-title">Select material</h3>
        <div id="gear-material-costs" class="gear-material-costs"></div>
        <label id="gear-quantity-row">Production quantity: <output id="gear-create-quantity-value">1</output><input id="gear-create-quantity" type="range" min="1" max="1000" value="1" /></label>
        <strong id="gear-storage-label" class="gear-storage-label"></strong>
        <div class="source-popup-actions"><button id="gear-create-submit" class="source-green-button" type="button">Craft</button><button id="gear-create-sell" class="source-green-button" type="button">Dismantle</button><button id="gear-create-close" class="source-red-button" type="button">Close</button></div>
      </section>
      <section id="consum-create-pop" class="consum-create-pop source-popup" hidden aria-label="ConsumCreatePop">
        <b id="consum-create-title">Quantity</b><i class="source-popup-line"></i>
        <div class="consum-quantity-panel">
          <button id="consum-minus" class="consum-round-button minus" type="button" aria-label="Decrease quantity"></button>
          <div class="consum-product-frame"><img id="consum-create-icon" alt="" /><output id="consum-create-quantity">1</output></div>
          <button id="consum-plus" class="consum-round-button plus" type="button" aria-label="Increase quantity"></button>
          <div class="consum-step-buttons"><button type="button" data-consum-delta="-10">-10</button><button type="button" data-consum-delta="10">+10</button><button type="button" data-consum-delta="100">+100</button><button type="button" data-consum-delta="1000">+1000</button></div>
          <p id="consum-conversion"></p>
        </div>
        <h3 id="consum-material-title">Select material</h3>
        <div id="consum-material-grid" class="consum-material-grid"></div>
        <div class="source-popup-actions"><button id="consum-create-submit" class="source-green-button" type="button">Produce</button><button id="consum-create-close" class="source-red-button" type="button">Close</button></div>
      </section>
      <div class="guild-chat-bar" aria-label="Guild chat"><span aria-hidden="true">&#9670;</span><b>Guild Chat</b><i></i></div>
      <nav class="bottom-menu" aria-label="Village menu">${menuItems.map((item) => `<button class="menu-button" type="button" data-action="${item.action}" ${item.enabled ? "" : 'disabled title="Feature in development"'}><span class="menu-icon"><img src="${originalAsset(item.icon)}" alt="" /></span><b>${item.label}</b></button>`).join("")}</nav>
      <button id="connection-status" class="connection-status connecting" type="button" aria-label="Server connection status"><i></i><span>Connecting</span></button>
    </section>
    <section id="roster-screen" class="roster-screen" aria-label="Hunter roster" aria-hidden="true">
      <img class="roster-background" src="/content/releases/visible-world-v1/maps/map_new01.png" alt="" />
      <section class="hunter-roster-panel" aria-label="Hunter management">
        <header class="hunter-roster-header"><div class="hunter-roster-actions"><button type="button" disabled>Place the Hunting Grounds</button><button type="button" disabled>Sort Hunters</button></div><div class="hunter-roster-heading"><b>Hunter List</b><span id="hunter-capacity">0 / 8</span></div><button id="roster-back" class="source-red-button" type="button" aria-label="Close Hunter List">Close</button></header>
        <div class="hunter-roster-body"><div id="hunter-active-list" class="hunter-card-grid"></div><section class="hunter-waiting-section"><h2>Waiting</h2><div id="hunter-waiting-list" class="hunter-waiting-list"></div></section></div>
        <footer id="hunter-roster-status" class="hunter-roster-status"></footer>
      </section>
    </section>
    <div id="loading-transition" class="loading-transition" hidden><img src="${originalAsset("sprites/cloud_loading_btn__4266.png")}" alt="" /><span>Loading...</span></div>
  </main>`;

function element<T extends HTMLElement>(selector: string): T { const value = document.querySelector<T>(selector); if (!value) throw new Error(`Missing UI element ${selector}`); return value; }
const bootScreen = element<HTMLElement>("#boot-screen");
const villageScreen = element<HTMLElement>("#village-screen");
const rosterScreen = element<HTMLElement>("#roster-screen");
const hunterCapacity = element<HTMLElement>("#hunter-capacity");
const hunterActiveList = element<HTMLElement>("#hunter-active-list");
const hunterWaitingList = element<HTMLElement>("#hunter-waiting-list");
const hunterWaitingSection = element<HTMLElement>(".hunter-waiting-section");
const hunterRosterStatus = element<HTMLElement>("#hunter-roster-status");
const transition = element<HTMLElement>("#loading-transition");
const panelMessage = element<HTMLElement>("#panel-message");
const connectionStatus = element<HTMLButtonElement>("#connection-status");
const worldViewport = element<HTMLElement>("#world-viewport");
const fieldBack = element<HTMLButtonElement>("#field-back");
const worldModeLabel = element<HTMLElement>("#world-mode-label");
const goldAmount = element<HTMLElement>("#gold-amount");
const enterVillage = element<HTMLButtonElement>("#enter-village");
const bootStatus = element<HTMLElement>("#boot-status");
const mapLoading = element<HTMLElement>(".map-loading");
const mapLoadingFill = element<HTMLElement>("#map-loading-fill");
const mapLoadingLabel = element<HTMLElement>("#map-loading-label");
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
const gearCreateQuantityValue = element<HTMLOutputElement>("#gear-create-quantity-value");
const gearFrameQuantity = element<HTMLOutputElement>("#gear-frame-quantity");
const gearStorageLabel = element<HTMLElement>("#gear-storage-label");
const gearCreateSubmit = element<HTMLButtonElement>("#gear-create-submit");
const gearCreateSell = element<HTMLButtonElement>("#gear-create-sell");
const gearCreateClose = element<HTMLButtonElement>("#gear-create-close");
const consumCreatePop = element<HTMLElement>("#consum-create-pop");
const consumCreateTitle = element<HTMLElement>("#consum-create-title");
const consumCreateIcon = element<HTMLImageElement>("#consum-create-icon");
const consumCreateQuantity = element<HTMLOutputElement>("#consum-create-quantity");
const consumConversion = element<HTMLElement>("#consum-conversion");
const consumMaterialTitle = element<HTMLElement>("#consum-material-title");
const consumMaterialGrid = element<HTMLElement>("#consum-material-grid");
const consumCreateSubmit = element<HTMLButtonElement>("#consum-create-submit");
const consumCreateClose = element<HTMLButtonElement>("#consum-create-close");
const consumMinus = element<HTMLButtonElement>("#consum-minus");
const consumPlus = element<HTMLButtonElement>("#consum-plus");
const evidenceDiagnostics = element<HTMLElement>("#evidence-diagnostics");
const combatHud = element<HTMLElement>("#combat-hud");
const equipFixtureItem = element<HTMLButtonElement>("#equip-fixture-item");
let latestSnapshot: OriginalFlowSnapshot | null = null;
let world: VisibleEntityWorld | null = null;
let connectionState: ConnectionStatus = "connecting";
let bootRequested = false;
let latestCombatHud: CombatHudState | null = null;
let panelMessageTimer: number | undefined;
let mapReady = false;
let mapLoadFailed = false;
let selectedBuildingId: string | null = null;
let selectedBuildingInstanceId: string | null = null;
let selectedBuildingVisual: TownBuilding | null = null;
let buildingPanelMode: "building" | "construct" = "building";
let selectedRecipe: ShopRecipeSnapshot | null = null;
let selectedServiceMaterialId: string | null = null;
let selectedServiceQuantity = 1;
const serviceTabsByBuilding = new Map<string, "production" | "hunters">();
let gearTab: GearKind = "weapon";
let blacksmithDifficultyGroup = 1;
let blacksmithCraftableOnly = false;
let gearCatalog: GearCatalogRecipe[] = [];
const gearMaterialIcons = new Map<string, string>();
let gearPopupMode: "craft" | "detail" = "craft";
let selectedBountyTier = 0;
let selectedTradingPostDifficulty = 0;
let buildingEvidenceRegistry: EvidenceBuildingRegistry | null = null;
let buildingEvidenceError: string | null = null;
let popupInteractionActive = false;
let popupInteractionReleaseTimer: number | undefined;
let popupSnapshotSignature = "";
let selectedHunterId: string | null = null;
const hunterInfoModal = createHunterInfoModal(rosterScreen);

const holdPopupRender = (): void => {
  if (popupInteractionReleaseTimer !== undefined) window.clearTimeout(popupInteractionReleaseTimer);
  popupInteractionActive = true;
};
const releasePopupRender = (): void => {
  popupInteractionReleaseTimer = window.setTimeout(() => {
    const active = document.activeElement;
    if (active instanceof HTMLElement && active.closest(".source-popup") && active.matches("select, input, textarea")) {
      popupInteractionReleaseTimer = undefined;
      return;
    }
    popupInteractionActive = false;
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

const client = new WorldClient(renderSnapshot, updateConnectionStatus, showIntentResult, showBindingBlocked);
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
enterVillage.disabled = true;
enterVillage.addEventListener("click", () => {
  if (!mapReady || mapLoadFailed) return;
  if (!client.completeBoot()) return;
  bootRequested = true;
  updateBootState();
});
document.querySelectorAll<HTMLButtonElement>("[data-action]").forEach((button) => button.addEventListener("click", () => {
  document.querySelectorAll("[data-action]").forEach((item) => item.classList.remove("selected"));
  button.classList.add("selected");
  const action = button.dataset.action as MenuAction;
  if (action === "build") {
    buildingPanelMode = "construct";
    buildingPanel.hidden = false;
    client.selectBottomMenu("build");
    renderBuildingSystem(latestSnapshot);
  } else if (action === "field") client.enterField();
  else client.selectBottomMenu(action);
}));
buildingPanelClose.addEventListener("click", () => { buildingPanel.hidden = true; });
buildingConstruct.addEventListener("click", () => { if (selectedBuildingId) client.constructBuilding(selectedBuildingId); });
buildingUpgrade.addEventListener("click", () => { if (selectedBuildingInstanceId) client.upgradeBuilding(selectedBuildingInstanceId); });
buildingUse.addEventListener("click", () => {
  if (!selectedBuildingId || !selectedBuildingInstanceId) return;
  const route = projectBuildingEvidence(buildingEvidenceRegistry, selectedBuildingId)?.popupRoute ?? null;
  if (selectedBuildingId === BOUNTY_HUT_ROUTE.buildingId) {
    bountyPop.hidden = false;
    renderBountyPop();
  } else if (selectedBuildingId === TRADING_POST_ROUTE.buildingId || route === "request") {
    renderBuildingSystem(latestSnapshot);
  } else if (route === "production") {
    renderBuildingSystem(latestSnapshot);
  } else if (route === "service") {
    const recipe = latestSnapshot?.village.building_system.recipes.find((item) => item.shop_id === selectedBuildingId);
    if (recipe) client.craftShopItem(selectedBuildingInstanceId, recipe.id, 1);
  }
});
bountyClose.addEventListener("click", () => { bountyPop.hidden = true; });
bountyCloseBottom.addEventListener("click", () => { bountyPop.hidden = true; });
bountyUpgrade.addEventListener("click", () => { if (selectedBuildingInstanceId) client.upgradeBuilding(selectedBuildingInstanceId); });
gearCreateQuantity.addEventListener("input", () => {
  gearCreateQuantityValue.value = gearCreateQuantity.value;
  renderGearCreatePop();
});
function changeGearQuantity(delta: number): void {
  gearCreateQuantity.value = String(Math.max(1, Math.min(1000, Number(gearCreateQuantity.value) + delta)));
  renderGearCreatePop();
}
element<HTMLButtonElement>("#gear-quantity-minus").addEventListener("click", () => changeGearQuantity(-1));
element<HTMLButtonElement>("#gear-quantity-plus").addEventListener("click", () => changeGearQuantity(1));
document.querySelectorAll<HTMLButtonElement>("[data-gear-delta]").forEach((button) => {
  button.addEventListener("click", () => changeGearQuantity(Number(button.dataset.gearDelta)));
});
gearCreateSubmit.addEventListener("click", () => {
  if (selectedBuildingInstanceId && selectedRecipe) {
    client.craftShopItem(selectedBuildingInstanceId, selectedRecipe.id, Number(gearCreateQuantity.value), selectedServiceMaterialId);
  }
});
gearCreateSell.addEventListener("click", () => {
  // Lock/dismantle require a concrete owned gear instance. Keep the recovered
  // detail control visible but fail closed until that binding exists.
});
gearLock.addEventListener("click", () => {});
gearCreateClose.addEventListener("click", () => { gearCreatePop.hidden = true; });
function changeServiceQuantity(delta: number): void {
  selectedServiceQuantity = Math.max(1, Math.min(1000, selectedServiceQuantity + delta));
  renderConsumCreatePop();
}
consumMinus.addEventListener("click", () => changeServiceQuantity(-1));
consumPlus.addEventListener("click", () => changeServiceQuantity(1));
document.querySelectorAll<HTMLButtonElement>("[data-consum-delta]").forEach((button) => {
  button.addEventListener("click", () => changeServiceQuantity(Number(button.dataset.consumDelta)));
});
consumCreateSubmit.addEventListener("click", () => {
  if (selectedBuildingInstanceId && selectedRecipe) {
    const materialId = selectedRecipe.kind === "service" ? selectedServiceMaterialId : null;
    if (selectedRecipe.kind !== "service" || materialId) {
      client.craftShopItem(selectedBuildingInstanceId, selectedRecipe.id, selectedServiceQuantity, materialId);
    }
  }
});
consumCreateClose.addEventListener("click", () => { consumCreatePop.hidden = true; });
element<HTMLButtonElement>("#roster-back").addEventListener("click", () => client.navigateBack());
fieldBack.addEventListener("click", () => client.navigateBack());
equipFixtureItem.addEventListener("click", () => {
  if (latestCombatHud?.equipEligible) client.equipHunterItem(1, 2001);
});
connectionStatus.addEventListener("click", () => client.requestResync());
client.connect();
void initializeBuildingEvidence();
void initializeWorld().catch((error: unknown) => {
  console.error("Failed to initialize the visible world.", error);
  mapLoadFailed = true;
  transition.hidden = true;
  showPanelMessage("Content unavailable", "The visible-world release could not be loaded.");
  updateBootState();
});

async function initializeBuildingEvidence(): Promise<void> {
  try {
    const [registry, decodedCatalog] = await Promise.all([
      loadVerifiedBuildingEvidenceRegistry(),
      loadGearCatalog().catch(() => null),
    ]);
    buildingEvidenceRegistry = registry;
    gearCatalog = decodedCatalog ?? decodeGearCatalog(
      registry.catalogs.items.rows,
      registry.catalogs.products.rows,
    );
    gearMaterialIcons.clear();
    for (const recipe of gearCatalog) {
      for (const cost of recipe.materialCosts) {
        if (cost.iconPath) gearMaterialIcons.set(cost.materialId, cost.iconPath);
      }
    }
    buildingEvidenceError = null;
    if (world && latestSnapshot) syncBuildingPresentation(world, latestSnapshot);
    renderBuildingSystem(latestSnapshot);
  } catch (error) {
    buildingEvidenceError = error instanceof Error ? error.message : "Building evidence failed to load";
    console.error("Failed to load verified building evidence.", error);
  }
}

async function initializeWorld(): Promise<void> {
  const app = new Application();
  await app.init({ resizeTo: worldViewport, backgroundColor: TOWN_CAMERA_CLEAR_COLOR, backgroundAlpha: 1, antialias: false, autoDensity: true, resolution: Math.min(devicePixelRatio, 2) });
  worldViewport.appendChild(app.canvas);
  const visibleWorld = new VisibleEntityWorld((entityId) => {
    if (!client.selectEntity(entityId)) return;
    showPanelMessage("Entity selected", entityId);
  }, (instance, visual) => {
    selectedBuildingId = instance.building_id;
    selectedBuildingInstanceId = instance.instance_id;
    selectedBuildingVisual = visual;
    buildingPanelMode = "building";
    const evidence = projectBuildingEvidence(buildingEvidenceRegistry, instance.building_id);
    if (!evidence) {
      buildingPanel.hidden = true;
      showPanelMessage("Building binding unresolved", instance.building_id);
      return;
    }
    buildingPanel.hidden = false;
    renderBuildingSystem(latestSnapshot);
  });
  const diagnostics = await visibleWorld.initialize((loaded, total) => {
    const percent = total ? Math.round((loaded / total) * 100) : 0;
    mapLoadingFill.style.width = `${percent}%`;
    mapLoadingLabel.textContent = `Loading map ${percent}%`;
  });
  mapLoading.hidden = true;
  if (debugUi) {
    evidenceDiagnostics.hidden = false;
    setPanelMessage(evidenceDiagnostics, diagnostics.fixture ? "Migration fixture" : "Runtime content", `Unresolved evidence: ${diagnostics.unresolved.join(", ")}`);
  }
  app.stage.addChild(visibleWorld.root);
  const resize = (): void => visibleWorld.resize(worldViewport.clientWidth, worldViewport.clientHeight);
  const resizeObserver = new ResizeObserver(resize);
  resizeObserver.observe(worldViewport);
  resize();
  let dragging = false;
  let dragCaptured = false;
  let lastX = 0;
  let lastY = 0;
  let pointerDownX = 0;
  let pointerDownY = 0;
  worldViewport.addEventListener("pointerdown", (event) => {
    dragging = true;
    dragCaptured = false;
    lastX = event.clientX;
    lastY = event.clientY;
    pointerDownX = event.clientX;
    pointerDownY = event.clientY;
  });
  worldViewport.addEventListener("pointermove", (event) => {
    if (!dragging) return;
    if (!dragCaptured && Math.hypot(event.clientX - pointerDownX, event.clientY - pointerDownY) < 5) return;
    if (!dragCaptured) {
      dragCaptured = true;
      worldViewport.setPointerCapture(event.pointerId);
    }
    visibleWorld.panBy(event.clientX - lastX, event.clientY - lastY);
    lastX = event.clientX;
    lastY = event.clientY;
  });
  worldViewport.addEventListener("pointerup", (event) => {
    dragging = false;
    if (dragCaptured && worldViewport.hasPointerCapture(event.pointerId)) worldViewport.releasePointerCapture(event.pointerId);
    dragCaptured = false;
  });
  worldViewport.addEventListener("pointercancel", () => { dragging = false; dragCaptured = false; });
  worldViewport.addEventListener("wheel", (event) => {
    event.preventDefault();
    visibleWorld.zoomBy(event.deltaY > 0 ? -0.1 : 0.1);
  }, { passive: false });
  app.ticker.add(() => visibleWorld.tick());
  world = visibleWorld;
  if (latestSnapshot) syncBuildingPresentation(visibleWorld, latestSnapshot);
  mapReady = true;
  updateBootState();
  if (latestSnapshot) {
    visibleWorld.setMode(latestSnapshot.screen === "field" ? "field" : "village");
    visibleWorld.update(latestSnapshot.world.entities, latestSnapshot.world.visual_tick);
  }
  window.addEventListener("beforeunload", () => {
    resizeObserver.disconnect();
    visibleWorld.destroy();
    app.destroy(true);
  }, { once: true });
}

function renderSnapshot(snapshot: OriginalFlowSnapshot): void {
  latestSnapshot = snapshot;
  world?.setMode(snapshot.screen === "field" ? "field" : "village");
  if (world) syncBuildingPresentation(world, snapshot);
  world?.update(snapshot.world.entities, snapshot.world.visual_tick);
  if (snapshot.screen !== "boot") bootRequested = false;
  updateBootState();
  const village = snapshot.screen === "village" || snapshot.screen === "field";
  const roster = snapshot.screen === "hunter_roster";
  bootScreen.classList.toggle("leaving", !snapshot.screen || snapshot.screen !== "boot");
  villageScreen.classList.toggle("visible", village);
  villageScreen.classList.toggle("field-mode", snapshot.screen === "field");
  villageScreen.setAttribute("aria-hidden", String(!village));
  rosterScreen.classList.toggle("visible", roster);
  rosterScreen.setAttribute("aria-hidden", String(!roster));
  if (roster) renderHunterRoster(snapshot);
  else if (hunterInfoModal.visible()) hunterInfoModal.close();
  worldModeLabel.textContent = snapshot.screen === "field" ? "Hunt" : "Easy";
  const nextPopupSignature = popupDataSignature(snapshot);
  if (!popupInteractionActive && nextPopupSignature !== popupSnapshotSignature) {
    popupSnapshotSignature = nextPopupSignature;
    if (nextPopupSignature !== "closed") {
      renderBuildingSystem(snapshot);
      if (!gearCreatePop.hidden) renderGearCreatePop();
      if (!consumCreatePop.hidden) renderConsumCreatePop();
    }
  }
  const resources = projectResourceBar(snapshot);
  const displayedGold = snapshot.screen === "village" ? snapshot.village.building_system.town_gold : resources.gold;
  goldAmount.textContent = displayedGold === null ? "--" : String(displayedGold);
  goldAmount.parentElement?.classList.toggle("unresolved", !resources.evidenceBacked);
  fieldBack.hidden = snapshot.screen !== "field";
  renderCombatHud(projectCombatHud(snapshot.screen, snapshot.migration_fixture_combat));
}

function renderHunterRoster(snapshot: OriginalFlowSnapshot): void {
  const roster = projectHunterRoster(snapshot, selectedHunterId);
  selectedHunterId = roster.selectedId;
  hunterCapacity.textContent = `${roster.active.length} / ${roster.capacity}`;
  hunterCapacity.classList.toggle("full", roster.active.length >= roster.capacity);
  hunterActiveList.replaceChildren(...Array.from({ length: roster.capacity }, (_, index) => {
    const hunter = roster.active[index];
    return hunter ? hunterRosterCard(hunter, snapshot) : emptyHunterSlot(index + 1);
  }));
  hunterWaitingList.replaceChildren(...(roster.waiting.length > 0
    ? roster.waiting.map((hunter) => hunterRosterCard(hunter, snapshot))
    : [emptyWaitingRow()]));
  hunterWaitingSection.hidden = roster.waiting.length === 0;
  hunterRosterStatus.textContent = roster.constraintViolation
    ?? (!roster.resolved ? "Waiting for the authoritative Hunter roster from the server." : roster.waiting.length > 0 ? `${roster.waiting.length} Hunter(s) will enter after an active slot is released.` : "All arriving Hunters can enter town.");
  hunterRosterStatus.classList.toggle("error", roster.constraintViolation !== null);
}

function hunterRosterCard(hunter: HunterView, snapshot: OriginalFlowSnapshot): HTMLElement {
  const card = document.createElement("article");
  card.className = `hunter-roster-card${hunter.id === selectedHunterId ? " selected" : ""}`;
  card.dataset.hunterId = hunter.id;
  const heading = document.createElement("header");
  const name = document.createElement("b");
  name.textContent = hunter.name;
  heading.append(name);
  if (hunter.rarityName) {
    const rarity = document.createElement("i");
    rarity.textContent = hunter.rarityName;
    heading.append(rarity);
  }
  const avatar = document.createElement("span");
  avatar.className = "hunter-avatar";
  if (hunter.portrait) {
    const image = document.createElement("img");
    image.src = hunter.portrait;
    image.alt = "";
    avatar.append(image);
  } else avatar.textContent = hunter.classFamily ?? "H";
  const meta = document.createElement("span");
  meta.className = "hunter-card-meta";
  const levelClass = document.createElement("b");
  levelClass.textContent = [hunter.level === null ? null : `Lv.${hunter.level}`, hunter.className ?? hunter.classFamily].filter(Boolean).join(" ") || "Class unavailable";
  const activity = document.createElement("small");
  activity.textContent = hunter.rosterState === "waiting" ? `Waiting #${hunter.queuePosition ?? "-"}` : hunter.action ?? "Activity unavailable";
  meta.append(levelClass, activity);
  const info = document.createElement("button");
  info.type = "button";
  info.className = "hunter-card-info";
  info.textContent = "Info";
  info.addEventListener("click", () => {
    selectedHunterId = hunter.id;
    const raw = rawHunterFor(snapshot, hunter);
    hunterInfoModal.show(projectHunterInfo(raw, hunter));
  });
  card.append(heading, avatar, meta, info);
  return card;
}

function emptyHunterSlot(slot: number): HTMLElement {
  const card = document.createElement("article");
  card.className = "hunter-roster-card empty";
  card.innerHTML = `<span class="hunter-avatar">+</span><b>Empty slot ${slot}</b>`;
  return card;
}

function emptyWaitingRow(): HTMLElement {
  const row = document.createElement("p");
  row.className = "hunter-waiting-empty";
  row.textContent = "No Hunters waiting outside town.";
  return row;
}

function rawHunterFor(snapshot: OriginalFlowSnapshot, hunter: HunterView): unknown {
  const roster = snapshot.hunter_roster as unknown as { active_hunters?: unknown[]; waiting_hunters?: unknown[]; waiting_queue?: unknown[] };
  const rows = [...(roster.active_hunters ?? []), ...(roster.waiting_hunters ?? roster.waiting_queue ?? [])];
  return rows.find((value) => {
    if (typeof value !== "object" || value === null) return false;
    const row = value as Record<string, unknown>;
    return hunter.numericId !== null && (row.hunter_id === hunter.numericId || row.id === hunter.numericId);
  }) ?? {};
}

function popupDataSignature(snapshot: OriginalFlowSnapshot): string {
  if (buildingPanel.hidden && gearCreatePop.hidden && consumCreatePop.hidden && bountyPop.hidden) return "closed";
  const system = snapshot.village.building_system;
  return JSON.stringify([
    selectedBuildingId,
    selectedBuildingInstanceId,
    buildingPanelMode,
    system.states,
    system.instances,
    system.recipes,
    system.material_stocks,
    snapshot.hunter_roster.product_services,
  ]);
}

function renderBuildingSystem(snapshot: OriginalFlowSnapshot | null): void {
  const system = snapshot?.village.building_system;
  if (!system) return;
  const evidenceBuildings = listBuildingEvidence(buildingEvidenceRegistry);
  if (buildingPanelMode === "construct" && (!selectedBuildingId || !evidenceBuildings.some((item) => item.id === selectedBuildingId))) {
    selectedBuildingId = evidenceBuildings[0]?.id ?? null;
    selectedBuildingInstanceId = null;
    selectedBuildingVisual = null;
  }
  if (buildingPanelMode === "construct") {
    buildingCatalog.hidden = false;
    buildingCatalog.replaceChildren(...evidenceBuildings.map((evidence) => {
      const state = system.states.find((item) => item.id === evidence.id);
      const button = document.createElement("button");
      button.type = "button";
      button.className = evidence.id === selectedBuildingId ? "selected" : "";
      button.textContent = `${evidence.name} ${state?.constructed ? `Lv.${state.level}` : `Lv.1-${evidence.maxLevel ?? "?"}`}`;
      button.addEventListener("click", () => {
        selectedBuildingId = evidence.id;
        selectedBuildingInstanceId = null;
        selectedBuildingVisual = null;
        buildingPanelMode = "construct";
        renderBuildingSystem(latestSnapshot);
      });
      return button;
    }));
  } else {
    // A building tap opens that building's detail popup, never the construction catalog.
    buildingCatalog.hidden = true;
    buildingCatalog.replaceChildren();
  }
  buildingPanel.classList.toggle("construct-mode", buildingPanelMode === "construct");
  buildingPanel.classList.toggle("detail-mode", buildingPanelMode === "building");
  const state = system.states.find((item) => item.id === selectedBuildingId);
  const selectedInstance = findBuildingInstanceById(system.instances, selectedBuildingInstanceId);
  if (buildingPanelMode === "building" && selectedBuildingInstanceId && !selectedInstance) {
    selectedBuildingId = null;
    selectedBuildingInstanceId = null;
    selectedBuildingVisual = null;
    buildingPanel.hidden = true;
    return;
  }
  const evidence = selectedBuildingId ? projectBuildingEvidence(buildingEvidenceRegistry, selectedBuildingId) : null;
  const isBlacksmithRoute = buildingPanelMode === "building" && BLACKSMITH_BUILDING_IDS.includes(selectedBuildingId as typeof BLACKSMITH_BUILDING_IDS[number]);
  const isJewelerRoute = buildingPanelMode === "building" && JEWELER_BUILDING_IDS.includes(selectedBuildingId as typeof JEWELER_BUILDING_IDS[number]);
  const isCraftingGearRoute = isBlacksmithRoute || isJewelerRoute;
  const isDisplayShopRoute = buildingPanelMode === "building" && (
    WEAPON_SHOP_BUILDING_IDS.includes(selectedBuildingId as typeof WEAPON_SHOP_BUILDING_IDS[number])
    || ARMOR_SHOP_BUILDING_IDS.includes(selectedBuildingId as typeof ARMOR_SHOP_BUILDING_IDS[number])
    || ACCESSORY_SHOP_BUILDING_IDS.includes(selectedBuildingId as typeof ACCESSORY_SHOP_BUILDING_IDS[number])
    || selectedBuildingId === POTION_SHOP_BUILDING_ID
  );
  const isPotionCraftingRoute = buildingPanelMode === "building" && selectedBuildingId === ALCHEMIST_BUILDING_ID;
  const isCatalogShopRoute = isCraftingGearRoute || isDisplayShopRoute || isPotionCraftingRoute;
  buildingPanel.classList.toggle("service-mode", buildingPanelMode === "building" && evidence?.popupRoute === "service");
  buildingPanel.classList.toggle("service-building-ui", buildingPanelMode === "building" && productServiceRoute(selectedBuildingId ?? "") !== null);
  buildingPanel.classList.toggle("trading-post-ui", buildingPanelMode === "building" && selectedBuildingId === TRADING_POST_ROUTE.buildingId);
  buildingPanel.classList.toggle("gear-route-ui", isCatalogShopRoute);
  buildingPanel.classList.toggle("blacksmith-ui", isCraftingGearRoute);
  buildingPanel.classList.toggle("jeweler-ui", isJewelerRoute);
  buildingPanel.classList.toggle("display-shop-ui", isDisplayShopRoute);
  buildingPanel.classList.toggle("potion-shop-ui", isPotionBuilding(selectedBuildingId));
  buildingPanel.classList.toggle("potion-crafting-ui", isPotionCraftingRoute);
  if (!evidence) {
    buildingName.textContent = selectedBuildingId ?? "Building evidence unavailable";
    buildingLevel.textContent = buildingEvidenceError ?? "Loading verified building evidence...";
    buildingFeature.textContent = "No fabricated building data is shown.";
    buildingCondition.textContent = "All interactions are disabled until the evidence binding loads.";
    const previewPath = selectedBuildingVisual?.publicPath ?? "";
    if (previewPath) buildingPreview.src = previewPath;
    else buildingPreview.removeAttribute("src");
    buildingPreview.hidden = !previewPath;
    buildingConstruct.disabled = true;
    buildingUpgrade.disabled = true;
    buildingUse.disabled = true;
    buildingLevelContract.replaceChildren();
    return;
  }
  buildingName.textContent = evidence.name;
  const spriteId = evidence.spriteAssetId;
  const previewPath = selectedBuildingVisual?.publicPath ?? (spriteId ? `/content/releases/visible-world-v1/village/buildings/${spriteId}.png` : "");
  if (previewPath) buildingPreview.src = previewPath;
  else buildingPreview.removeAttribute("src");
  buildingPreview.hidden = !previewPath;
  const currentLevel = selectedInstance?.level ?? (state?.constructed ? state.level : 0);
  if (buildingPanelMode === "building") buildingName.textContent = "Lv." + currentLevel + " " + evidence.name;
  const targetLevel = Math.min(currentLevel + 1, evidence.maxLevel ?? currentLevel + 1);
  buildingLevel.textContent = state?.constructed
    ? `Level ${currentLevel} / ${evidence.maxLevel ?? "?"} · Next: ${formatLevelCosts(evidence, targetLevel)}`
    : `Not constructed · ${formatLevelCosts(evidence, 1)}`;
  const rawSourceSummary = [
    evidence.maxBuild === null ? null : `maxBuild ${evidence.maxBuild}`,
    evidence.gridSize === null ? null : `size ${evidence.gridSize[0]}×${evidence.gridSize[1]}`,
  ].filter((entry): entry is string => entry !== null).join(" · ");
  const sourceDescriptions: Record<string, string> = {
    build_3: "Helps the town purchase loot from hunters",
    build_7: "Displays and sells weapons crafted at the Blacksmith",
    build_8: "Displays and sells armor crafted at the Blacksmith",
    build_9: "Where hunters rest after becoming exhausted from hunting",
    build_10: "Crafts weapons and armor using materials purchased from hunters",
    build_11: "Displays and sells potions crafted at the Alchemist's Home",
    build_20: "Displays and sells rings, necklaces, and belts crafted at the Jeweler",
    build_21: "Crafts rings, necklaces, and belts using materials purchased from hunters",
    build_12: "Injured hunters can be healed here",
    build_13: "Sells food to fill up hunters' satiety gauge",
    build_14: "Crafts potions from materials purchased from hunters",
    build_19: "Sells cold beverages that raise hunters' morale a little",
  };
  const featureDescription = sourceDescriptions[evidence.id] ?? evidence.description;
  buildingFeature.textContent = buildingPanelMode === "building" ? featureDescription
    : rawSourceSummary ? `${featureDescription} · ${rawSourceSummary}` : featureDescription;
  const targetRequirement = evidence.levels.find((entry) => entry.level === targetLevel)?.requiredTownHallLevel ?? null;
  const localizedRequirement = targetRequirement === null
    ? "Town Hall requirement unresolved"
    : originalUiLabel("buildpop_9", undefined, [targetRequirement]);
  buildingCondition.textContent = selectedInstance?.condition?.startsWith("building_prerequisite_required:")
    ? localizedRequirement
    : selectedInstance?.condition ?? localizedRequirement;
  buildingLevelContract.replaceChildren(...evidence.levels.map((entry) => {
    const row = document.createElement("div");
    const title = document.createElement("strong");
    title.textContent = `Lv.${entry.level}`;
    const requirement = document.createElement("span");
    requirement.textContent = entry.requiredTownHallLevel === null
      ? "Town Hall requirement unresolved"
      : originalUiLabel("buildpop_9", undefined, [entry.requiredTownHallLevel]);
    const costs = document.createElement("small");
    costs.textContent = `${originalUiLabel("buildpop_32")}: ${entry.costs.length ? entry.costs.join(" · ") : "--"}`;
    row.append(title, requirement, costs);
    return row;
  }));
  if (buildingPanelMode === "building" && evidence.id === TRADING_POST_ROUTE.buildingId) {
    buildingLevelContract.hidden = true;
    buildingCatalog.hidden = false;
    renderTradingPostCatalog(system.material_stocks, selectedInstance?.level ?? 1, targetRequirement);
  } else if (buildingPanelMode === "building" && evidence.popupRoute === "production") {
    buildingLevelContract.hidden = true;
    buildingCatalog.hidden = false;
    if (isCraftingGearRoute) {
      renderGearCraftingCatalog(system.recipes, evidence.id);
    } else if (isPotionCraftingRoute) {
      renderPotionCraftingCatalog(system.recipes, currentLevel);
    } else {
      renderDisplayShopCatalog(system.recipes, selectedBuildingId);
    }
  } else if (buildingPanelMode === "building" && evidence.popupRoute === "service") {
    if (!selectedBuildingId) return;
    buildingLevelContract.hidden = true;
    buildingCatalog.hidden = false;
    const currentBuildingLevel = selectedInstance?.level ?? 1;
    const route = productServiceRoute(selectedBuildingId);
    const routeCandidates = system.recipes.filter((item) => item.shop_id === selectedBuildingId);
    // Project through the decoded route contract before rendering. This keeps
    // service popups isolated to their seven recovered product IDs.
    const routeProjection = route ? projectProductService(selectedBuildingId, routeCandidates.map((recipe): ProductServiceInput => ({
      productId: recipe.id,
      productName: recipe.product_name,
      requiredLevel: recipe.required_level,
      effectValue: recipe.effect_value,
      serviceTimeMs: recipe.duration_ms,
      useMoney: recipe.sale_price,
      stock: recipe.stock,
      capacity: recipe.capacity,
      materialCosts: recipe.material_costs.map((cost) => ({ materialId: cost.material_id, displayName: cost.display_name, quantity: cost.quantity, outputQuantity: cost.output_quantity })),
    })), routeCandidates[0]?.capacity ?? 0) : null;
    const allowedProductIds = routeProjection ? new Set(routeProjection.products.map((product) => product.productId)) : new Set<string>();
    const allRecipes = routeProjection
      ? routeCandidates.filter((item) => allowedProductIds.has(item.id))
      : [];
    const recipes = allRecipes.filter((item) => item.required_level < currentBuildingLevel);
    const capacity = recipes[0]?.capacity ?? 0;
    const totalStock = recipes.reduce((total, recipe) => total + recipe.stock, 0);
    const tabs = document.createElement("div");
    tabs.className = "service-tabs";
    const activeServiceTab = serviceTabsByBuilding.get(route?.buildingId ?? selectedBuildingId) ?? "production";
    const authoritativeService = route
      ? snapshot.hunter_roster.product_services.find((service) => service.building_id === route.buildingId) ?? null
      : null;
    if (route) {
      for (const tab of ["production", "hunters"] as const) {
        const button = document.createElement("button");
        button.type = "button";
        button.className = tab === activeServiceTab ? "selected" : "";
        button.textContent = tab === "production" ? "Production" : "Hunters";
        button.addEventListener("click", () => {
          serviceTabsByBuilding.set(route.buildingId, tab);
          renderBuildingSystem(latestSnapshot);
        });
        tabs.append(button);
      }
      const capacityLabel = document.createElement("strong");
      capacityLabel.textContent = activeServiceTab === "production"
        ? `Capacity ${totalStock} / ${capacity}`
        : `Hunters ${authoritativeService?.active.length ?? 0} / ${authoritativeService?.slots ?? 0}`;
      tabs.append(capacityLabel);
    } else {
      tabs.innerHTML = `<b>Production</b><span>Hunters</span><strong>Capacity ${totalStock} / ${capacity}</strong>`;
    }
    const productList = document.createElement("div");
    productList.className = "service-product-list";
    productList.replaceChildren(...recipes.map((recipe) => {
      const serviceRow = route !== null;
      const row = document.createElement(serviceRow ? "div" : "button");
      if (row instanceof HTMLButtonElement) row.type = "button";
      row.className = "service-product-row";
      const icon = document.createElement("img");
      const productIcon = recipe.icon || productServiceSprite(recipe.id);
      if (productIcon) icon.src = productIcon;
      else icon.hidden = true;
      icon.alt = "";
      const text = document.createElement("span");
      const name = document.createElement("strong");
      name.textContent = recipe.product_name;
      const effect = document.createElement("small");
      effect.textContent = "Recover " + recipe.effect_value.toLocaleString() + " " + (route?.effectKind ?? recipe.effect_kind) + " in " + (recipe.duration_ms / 1000) + " secs";
      const economy = document.createElement("small");
      if (route) {
        const goldIcon = document.createElement("img");
        goldIcon.className = "inline-currency-icon";
        goldIcon.src = originalAsset("sprites/top_ic_01_gold_24__4677.png");
        goldIcon.alt = "";
        economy.append(document.createTextNode("Fee "), goldIcon, document.createTextNode(` ${recipe.sale_price} Gold`));
      } else {
        economy.textContent = `Fee ${recipe.sale_price} Gold · Capacity ${recipe.stock}/${recipe.capacity}`;
      }
      text.append(name, effect, economy);
      const action = document.createElement(serviceRow ? "button" : "b");
      if (action instanceof HTMLButtonElement) action.type = "button";
      action.textContent = "Produce";
      row.append(icon, text, action);
      const openProduct = () => {
        selectedRecipe = recipe;
        selectedServiceMaterialId = recipe.kind === "service" ? (recipe.material_costs[0]?.material_id ?? null) : null;
        selectedServiceQuantity = 1;
        consumCreatePop.hidden = false;
        renderConsumCreatePop();
      };
      if (serviceRow) action.addEventListener("click", openProduct);
      else row.addEventListener("click", openProduct);
      return row;
    }));
    const nextRecipe = allRecipes
      .filter((recipe) => recipe.required_level >= currentBuildingLevel)
      .sort((left, right) => left.required_level - right.required_level)[0];
    const upgradeHint = document.createElement("div");
    upgradeHint.className = "service-upgrade-hint";
    if (nextRecipe) {
      upgradeHint.textContent = `When Upgraded to Lv.${nextRecipe.required_level + 1} Able to produce ${nextRecipe.product_name}`;
      if (route && targetRequirement !== null) {
        const requirement = document.createElement("em");
        requirement.textContent = `Town Hall Lv.${targetRequirement} or higher required.`;
        upgradeHint.append(requirement);
      }
    } else {
      upgradeHint.textContent = "All products available at this level";
    }
    if (route && activeServiceTab === "hunters") {
      const hunterList = document.createElement("div");
      hunterList.className = "service-hunter-list";
      if (!authoritativeService?.roster_resolved) {
        const blocked = document.createElement("div");
        blocked.className = "service-hunter-empty";
        blocked.textContent = authoritativeService?.blockers.join(" · ") || "Hunter service state is unresolved.";
        hunterList.append(blocked);
      } else {
        const stockedRecipes = recipes.filter((recipe) => recipe.stock > 0);
        const candidates = authoritativeService.hunters.filter((hunter) => hunter.current_value < hunter.maximum_value || hunter.service_state === "serving");
        for (const hunter of candidates) {
          const row = document.createElement("div");
          row.className = "service-product-row service-hunter-row";
          const text = document.createElement("span");
          const name = document.createElement("strong");
          name.textContent = `Hunter #${hunter.hunter_id}`;
          const gauge = document.createElement("small");
          gauge.textContent = `${route.effectKind} ${hunter.current_value.toLocaleString()} / ${hunter.maximum_value.toLocaleString()}`;
          text.append(name, gauge);
          const product = document.createElement("select");
          for (const recipe of stockedRecipes) {
            const option = document.createElement("option");
            option.value = recipe.id;
            option.textContent = `${recipe.product_name} (+${recipe.effect_value.toLocaleString()})`;
            product.append(option);
          }
          const action = document.createElement("button");
          action.type = "button";
          const activeVisit = authoritativeService.active.find((visit) => visit.hunter_id === hunter.hunter_id);
          action.textContent = activeVisit ? `${Math.ceil(activeVisit.remaining_ms / 1000)}s` : route.buildingId === "build_9" ? "Rest" : route.buildingId === "build_12" ? "Treat" : "Serve";
          action.disabled = hunter.service_state === "serving" || stockedRecipes.length === 0 || authoritativeService.available_slots === 0 || !selectedInstance;
          action.addEventListener("click", () => {
            if (selectedInstance && product.value) client.startBuildingService(selectedInstance.instance_id, hunter.hunter_id, product.value);
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
      buildingCatalog.replaceChildren(tabs, hunterList, upgradeHint);
    } else {
      buildingCatalog.replaceChildren(tabs, productList, upgradeHint);
    }
  } else if (buildingPanelMode === "building") {
    // Detail popups show the building function; upgrade levels belong to the
    // upgrade action and must not replace the building's main content.
    buildingLevelContract.hidden = true;
    buildingCatalog.hidden = true;
    buildingCatalog.replaceChildren();
  } else {
    buildingLevelContract.hidden = false;
  }
  buildingLevelContract.title = evidence.constructionBlockedReason ?? "";
  const spriteResolved = evidence.spriteAssetId !== null;
  buildingConstruct.hidden = buildingPanelMode !== "construct";
  buildingUpgrade.hidden = buildingPanelMode !== "building";
  const isBounty = evidence.id === BOUNTY_HUT_ROUTE.buildingId;
  buildingUse.hidden = buildingPanelMode !== "building" || evidence.id === TRADING_POST_ROUTE.buildingId || (evidence.popupRoute !== "request" && !isBounty);
  buildingConstruct.disabled = !spriteResolved || state?.constructed !== false || state.can_construct !== true;
  buildingConstruct.title = !spriteResolved ? "Building sprite binding unresolved" : state?.condition ?? "";
  buildingUpgrade.disabled = !spriteResolved || !selectedInstance || selectedInstance.can_upgrade !== true;
  buildingUpgrade.title = selectedInstance?.condition ?? "";
  buildingUpgrade.textContent = productServiceRoute(evidence.id) || isCatalogShopRoute
    ? originalUiLabel("buildpop_7")
    : `${originalUiLabel("buildpop_7")} · ${formatLevelCosts(evidence, targetLevel)}`;
  buildingUse.disabled = !selectedInstance || (evidence.popupRoute !== "request" && evidence.popupRoute !== "production" && !isBounty);
  buildingUse.title = evidence.popupRoute
    ? "The popup binding is resolved; its exact recovered hierarchy is not implemented yet."
    : evidence.actionBlockedReason ?? "Popup binding unresolved";
  const serviceLabels: Record<string, string> = {
    build_2: "Revive hunter",
    build_9: "Rest",
    build_12: "Treat hunter",
    build_13: "Serve meal",
    build_19: "Serve drink",
    build_24: "Bank service",
    build_25: "Study",
    build_26: "Restore",
    build_27: "Encourage",
    build_28: "Train",
  };
  buildingUse.textContent = isBounty ? "Bounties" : evidence.popupRoute === "request" ? "Requests"
    : evidence.popupRoute === "service" ? (serviceLabels[evidence.id] ?? "Use service")
    : evidence.popupRoute === "production" ? "Create" : "Open";
}

function gearKindFromRecipe(recipe: ShopRecipeSnapshot): GearKind | null {
  const match = recipe.id.match(/^recipe:(weapon|armor|gloves|boots|ring|necklace|belt):/);
  const kind = match?.[1] as GearKind | undefined;
  return kind && ALL_GEAR_KINDS.includes(kind) ? kind : null;
}

function openGearRecipe(recipe: ShopRecipeSnapshot): void {
  selectedRecipe = recipe;
  gearPopupMode = "craft";
  selectedServiceMaterialId = null;
  gearCreateQuantity.value = "1";
  gearCreateQuantityValue.value = "1";
  gearCreatePop.hidden = false;
  renderGearCreatePop();
}

function openGearDetail(recipe: ShopRecipeSnapshot): void {
  selectedRecipe = recipe;
  gearPopupMode = "detail";
  gearCreatePop.hidden = false;
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
  if (gearCatalog.length === 0) return liveRecipes.filter((recipe) => recipe.shop_id === producerBuildingId);
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
  return gearCatalog.filter((entry) => allowedKinds.some((kind) => kind === entry.kind)).map((entry) => {
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
      effect_value: 0,
      effect_kind: "none",
      capacity: live.reduce((capacity, recipe) => Math.max(capacity, recipe.capacity), familyCapacity(entry.kind)),
    };
  });
}

function renderGearCraftingCatalog(recipes: readonly ShopRecipeSnapshot[], producerBuildingId: string): void {
  const tabsForBuilding: readonly GearKind[] = producerBuildingId === JEWELER_BUILDING_IDS[0] ? JEWELER_GEAR_TABS : BLACKSMITH_GEAR_TABS;
  if (!tabsForBuilding.includes(gearTab)) gearTab = tabsForBuilding[0];
  const all = fullGearRecipes(recipes, producerBuildingId).filter((recipe) => gearKindFromRecipe(recipe) === gearTab);
  const catalogById = new Map(gearCatalog.map((entry) => [entry.id, entry]));
  const buildingLevel = findBuildingInstanceById(
    latestSnapshot?.village.building_system.instances ?? [],
    selectedBuildingInstanceId,
  )?.level ?? 1;
  const unlockedRating = Math.min(4, Math.max(0, buildingLevel - 1));
  const matching = all.filter((recipe) => {
    const rating = Number(recipe.id.match(/:rating:(\d+)$/)?.[1] ?? 0);
    const staticRow = catalogById.get(recipe.id);
    return rating === unlockedRating
      && (staticRow?.difficultyGroup === undefined || staticRow.difficultyGroup < 0 || staticRow.difficultyGroup === blacksmithDifficultyGroup)
      && (!blacksmithCraftableOnly || recipe.material_costs.every((cost) => {
      const stock = latestSnapshot?.village.building_system.material_stocks.find((item) => item.id === cost.material_id);
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
    button.setAttribute("aria-label", tab);
    button.dataset.gearTab = tab;
    button.className = tab === gearTab ? "selected" : "";
    button.addEventListener("click", () => { gearTab = tab; renderBuildingSystem(latestSnapshot); });
    tabs.append(button);
  }
  const filters = document.createElement("div");
  filters.className = "blacksmith-filters";
  const difficultyOptions = producerBuildingId === JEWELER_BUILDING_IDS[0]
    ? ["Junk", "Easy", "Normal", "Hard", "Expert", "Nightmare", "Torment"]
    : ["Easy", "Normal", "Hard", "Expert", "Nightmare", "Torment"];
  const difficultyEntries = difficultyOptions.map((label, index) => {
    const group = producerBuildingId === JEWELER_BUILDING_IDS[0] ? index : index + 1;
    return { value: String(group), label };
  });
  const difficulty = createGameDropdown("Gear difficulty", String(blacksmithDifficultyGroup), difficultyEntries, (value) => {
    blacksmithDifficultyGroup = Number(value);
    renderBuildingSystem(latestSnapshot);
  });
  filters.append(difficulty);
  const craftable = document.createElement("label");
  craftable.className = "blacksmith-craftable";
  const checkbox = document.createElement("input"); checkbox.type = "checkbox"; checkbox.checked = blacksmithCraftableOnly;
  checkbox.addEventListener("change", () => { blacksmithCraftableOnly = checkbox.checked; renderBuildingSystem(latestSnapshot); });
  craftable.append(checkbox, document.createTextNode("Craftable Items"));
  controls.append(tabs, filters);
  const grid = document.createElement("div"); grid.className = "blacksmith-grid";
  grid.replaceChildren(...matching.map((recipe) => {
    const card = document.createElement("button"); card.type = "button"; card.className = "gear-catalog-card";
    appendGearArt(card, recipe);
    const name = document.createElement("strong"); name.textContent = recipe.product_name;
    const action = document.createElement("b"); action.textContent = "Craft";
    card.append(name, action);
    card.addEventListener("click", () => openGearRecipe(recipe)); return card;
  }));
  if (matching.length === 0) {
    const empty = document.createElement("p");
    empty.className = "blacksmith-empty";
    empty.textContent = "No items match this filter.";
    grid.append(empty);
  }
  const footer = document.createElement("div");
  footer.className = "blacksmith-catalog-footer";
  const count = document.createElement("span");
  count.textContent = `${matching.length} items`;
  footer.append(count, craftable);
  const hint = document.createElement("div"); hint.className = "blacksmith-upgrade-hint";
  const nextTier = ["Regular", "Sturdy", "Refined", "Powerful", "Supreme"][buildingLevel];
  hint.textContent = nextTier
    ? `When Upgraded to Lv.${buildingLevel + 1} Able to craft ${nextTier} ${producerBuildingId === JEWELER_BUILDING_IDS[0] ? "accessories" : "weapons and armor"}`
    : "All decoded gear tiers are available";
  buildingCatalog.replaceChildren(controls, grid, footer, hint);
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
  const system = latestSnapshot?.village.building_system;
  const level = findBuildingInstanceById(system?.instances ?? [], selectedBuildingInstanceId)?.level ?? 1;
  const allowed = recipes.filter((recipe) => recipe.shop_id === buildingId && recipe.required_level < level);
  const heading = document.createElement("h3");
  const isPotionShop = buildingId === POTION_SHOP_BUILDING_ID;
  heading.textContent = isPotionShop ? "Potion display" : "Display list";
  const grid = document.createElement("div");
  grid.className = isPotionShop ? "display-shop-grid potion-recipe-grid" : "display-shop-grid";
  grid.replaceChildren(...allowed.map((recipe) => {
    if (isPotionShop) {
      const card = document.createElement("article");
      card.className = "gear-catalog-card display-card potion-catalog-card potion-display-card";
      const badge = document.createElement("span");
      badge.className = "potion-stock-badge";
      badge.textContent = `Stock\n${recipe.stock}`;
      appendPotionArt(card, recipe);
      const name = document.createElement("strong");
      name.textContent = recipe.product_name;
      const price = document.createElement("small");
      const gold = document.createElement("img");
      gold.src = originalAsset("sprites/top_ic_01_gold_24__4677.png");
      gold.alt = "";
      price.append(gold, document.createTextNode(recipe.sale_price.toLocaleString()));
      card.append(badge, name, price);
      return card;
    }
    const card = document.createElement("button");
    card.type = "button";
    card.className = "gear-catalog-card display-card";
    const badge = document.createElement("span");
    badge.className = "on-display-badge";
    badge.textContent = "On\nDisplay";
    appendGearArt(card, recipe);
    const name = document.createElement("strong"); name.textContent = recipe.product_name;
    const price = document.createElement("small");
    const gold = document.createElement("img");
    gold.src = originalAsset("sprites/top_ic_01_gold_24__4677.png");
    gold.alt = "";
    price.append(gold, document.createTextNode(recipe.sale_price.toLocaleString()));
    card.append(badge, name, price);
    card.addEventListener("click", () => openGearDetail(recipe));
    return card;
  }));
  if (allowed.length === 0) {
    const empty = document.createElement("p");
    empty.className = "display-shop-empty";
    empty.textContent = isPotionShop ? "No crafted potions on display." : "No crafted gear on display.";
    grid.append(empty);
  }
  const hint = document.createElement("div");
  hint.className = "blacksmith-upgrade-hint";
  const nextTier = ["Sturdy", "Refined", "Powerful", "Supreme"][Math.min(level - 1, 3)];
  hint.textContent = isPotionShop
    ? "Potions crafted at the Alchemist's Home are stocked here for hunters."
    : level < 5
      ? `When Upgraded to Lv.${level + 1} Able to display ${nextTier} ${buildingId === "build_7" ? "weapons" : "armor"}`
      : "All decoded display tiers are available";
  buildingCatalog.replaceChildren(heading, grid, hint);
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
  heading.textContent = "Potion recipes";
  const grid = document.createElement("div");
  grid.className = "display-shop-grid potion-recipe-grid";
  grid.replaceChildren(...allowed.map((recipe) => {
    const card = document.createElement("button");
    card.type = "button";
    card.className = "gear-catalog-card potion-catalog-card";
    const stock = document.createElement("span");
    stock.className = "potion-stock-badge";
    stock.textContent = `Stock\n${recipe.stock}/${recipe.capacity}`;
    appendPotionArt(card, recipe);
    const name = document.createElement("strong");
    name.textContent = recipe.product_name;
    const action = document.createElement("b");
    action.textContent = "Create";
    card.append(stock, name, action);
    card.addEventListener("click", () => {
      selectedRecipe = recipe;
      selectedServiceMaterialId = null;
      selectedServiceQuantity = 1;
      consumCreatePop.hidden = false;
      renderConsumCreatePop();
    });
    return card;
  }));
  if (allowed.length === 0) {
    const empty = document.createElement("p");
    empty.className = "display-shop-empty";
    empty.textContent = "No potion recipes are unlocked at this level.";
    grid.append(empty);
  }
  const hint = document.createElement("div");
  hint.className = "blacksmith-upgrade-hint";
  hint.textContent = "Create potions here, then sell the finished stock through the Potion Shop.";
  buildingCatalog.replaceChildren(heading, grid, hint);
}

function renderTradingPostCatalog(
  stocks: readonly MaterialStockSnapshot[],
  buildingLevel: number,
  nextTownHallRequirement: number | null,
): void {
  selectedTradingPostDifficulty = Math.min(selectedTradingPostDifficulty, Math.max(0, buildingLevel - 1));
  const activeRequests = stocks.filter((stock) => stock.requested > 0).length;
  const toolbar = document.createElement("div");
  toolbar.className = "trading-post-toolbar";
  const count = document.createElement("strong");
  count.textContent = `Request to Purchase: ${activeRequests}`;
  const difficulty = document.createElement("select");
  difficulty.setAttribute("aria-label", "Trading Post difficulty");
  tradingPostDifficultyOptions(buildingLevel).forEach(({ label, difficulty: index, unlocked }) => {
    const option = document.createElement("option");
    option.value = String(index);
    option.textContent = label;
    option.disabled = !unlocked;
    option.selected = index === selectedTradingPostDifficulty;
    difficulty.append(option);
  });
  difficulty.addEventListener("change", () => {
    selectedTradingPostDifficulty = Number(difficulty.value);
    difficulty.blur();
    renderBuildingSystem(latestSnapshot);
  });
  toolbar.append(count, difficulty);

  const visibleStocks = tradingPostStocksForDifficulty(stocks, selectedTradingPostDifficulty);
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
    const hunterCount = document.createElement("span");
    hunterCount.textContent = stock.hunter_quantity > 0 ? String(stock.hunter_quantity) : "";
    iconFrame.append(icon, hunterCount);
    const name = document.createElement("strong");
    name.textContent = stock.display_name;
    const action = document.createElement("button");
    action.type = "button";
    action.className = stock.requested > 0 ? "cancel" : "request";
    action.textContent = stock.requested > 0 ? "Cancel" : "Request";
    action.addEventListener("click", () => {
      if (!selectedBuildingInstanceId) return;
      if (stock.requested > 0) client.cancelMaterialRequest(selectedBuildingInstanceId, stock.id);
      else client.setMaterialRequest(selectedBuildingInstanceId, stock.id, 1);
    });
    card.append(iconFrame, name, action);
    return card;
  }));

  const hint = document.createElement("div");
  hint.className = "trading-post-upgrade-hint";
  const nextDifficulty = TRADING_POST_ROUTE.tabs[Math.min(buildingLevel, TRADING_POST_ROUTE.tabs.length - 1)];
  hint.textContent = buildingLevel < TRADING_POST_ROUTE.upgrade.maxLevel
    ? `When Upgraded to Lv.${buildingLevel + 1} Adds purchase reservation list of [${nextDifficulty}] difficulty`
    : "All decoded Trading Post levels unlocked";
  if (nextTownHallRequirement !== null) {
    const requirement = document.createElement("em");
    requirement.textContent = `Town Hall Lv.${nextTownHallRequirement} or higher required.`;
    hint.append(requirement);
  }
  buildingCatalog.replaceChildren(toolbar, grid, hint);
}

function renderBountyPop(): void {
  const system = latestSnapshot?.village.building_system;
  if (!system || selectedBuildingId !== BOUNTY_HUT_ROUTE.buildingId) return;
  const instance = findBuildingInstanceById(system.instances, selectedBuildingInstanceId);
  bountyTitle.textContent = `Lv.${instance?.level ?? 1} ${BOUNTY_HUT_ROUTE.title}`;
  bountyTierTabs.replaceChildren(...BOUNTY_TIERS.map((tier, index) => {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = tier.label;
    button.className = index === selectedBountyTier ? "selected" : "";
    button.addEventListener("click", () => {
      selectedBountyTier = index;
      renderBountyPop();
    });
    return button;
  }));
  bountyUpgrade.disabled = !instance || instance.can_upgrade !== true;
  const evidence = projectBuildingEvidence(buildingEvidenceRegistry, BOUNTY_HUT_ROUTE.buildingId);
  bountyUpgrade.textContent = `Upgrade · ${evidence ? formatLevelCosts(evidence, (instance?.level ?? 1) + 1) : "Cost unresolved"}`;
}

function resourceIconPath(resourceId: string): string | null {
  const paths: Record<string, string> = {
    "material:1": "/content/releases/original-flow-v1/sprites/shop_product_26__6294.png",
    "material:16": "/content/releases/original-flow-v1/sprites/shop_product_251__3130.png",
    "currency:gem": "/content/releases/original-flow-v1/sprites/top_ic_02_gem__6963.png",
    "currency:elemental": "/content/releases/original-flow-v1/sprites/top_ic_03_element__4250.png",
  };
  return paths[resourceId] ?? null;
}

function resolvedBuildingSpriteIds(): string[] {
  return listBuildingEvidence(buildingEvidenceRegistry)
    .filter((building) => building.spriteAssetId !== null)
    .map((building) => building.id);
}

function syncBuildingPresentation(target: VisibleEntityWorld, snapshot: OriginalFlowSnapshot): void {
  target.setBuildingPresentation(snapshot.village.building_system.instances, resolvedBuildingSpriteIds());
}

function renderGearCreatePop(): void {
  const system = latestSnapshot?.village.building_system;
  if (!system || !selectedRecipe) return;
  const quantity = Number(gearCreateQuantity.value);
  const gearKind = gearKindFromRecipe(selectedRecipe) ?? "weapon";
  const gearKindLabel = gearKind.charAt(0).toUpperCase() + gearKind.slice(1);
  gearCreatePop.classList.toggle("gear-detail-mode", gearPopupMode === "detail");
  gearCreateTitle.textContent = gearPopupMode === "detail" ? selectedRecipe.product_name : `Craft ${gearKindLabel}`;
  if (selectedRecipe.icon) gearCreateIcon.src = selectedRecipe.icon;
  else gearCreateIcon.removeAttribute("src");
  gearCreateIcon.hidden = !selectedRecipe.icon;
  gearCreateName.textContent = selectedRecipe.product_name;
  gearCreatePrice.textContent = gearPopupMode === "detail"
    ? `${gearKindLabel} · On Display · ${selectedRecipe.sale_price.toLocaleString()} Gold`
    : `${gearKindLabel} · Shop stock ${selectedRecipe.stock}/${selectedRecipe.capacity}`;
  gearCreateDescription.replaceChildren();
  if (gearPopupMode === "detail") {
    const unresolved = document.createElement("p");
    unresolved.textContent = "Gear options and rune slots are unavailable until this displayed item is bound to an owned gear instance.";
    gearCreateDescription.append(unresolved, gearLock);
  }
  gearLock.hidden = gearPopupMode !== "detail";
  gearLock.disabled = true;
  gearMaterialTitle.hidden = gearPopupMode === "detail";
  gearMaterialCosts.hidden = gearPopupMode === "detail";
  gearQuantityRow.hidden = gearPopupMode === "detail";
  gearStorageLabel.hidden = gearPopupMode === "detail";
  gearCreateSubmit.hidden = gearPopupMode === "detail";
  gearCreateSubmit.textContent = "Produce";
  gearCreateSell.hidden = gearPopupMode !== "detail";
  gearCreateSell.disabled = true;
  let craftable = true;
  gearMaterialCosts.replaceChildren(...selectedRecipe.material_costs.map((cost) => {
    const stock = system.material_stocks.find((item) => item.id === cost.material_id);
    const row = document.createElement("div");
    const selected = selectedRecipe?.kind !== "service" || cost.material_id === selectedServiceMaterialId;
    const iconPath = gearMaterialIcons.get(cost.material_id) ?? stock?.icon ?? resourceIconPath(cost.material_id);
    const icon = iconPath ? document.createElement("img") : document.createElement("span");
    if (icon instanceof HTMLImageElement) {
      icon.src = iconPath!;
      icon.alt = "";
    } else {
      icon.className = "unresolved-material-icon";
      icon.textContent = cost.display_name.slice(0, 2).toUpperCase();
      icon.title = `${cost.display_name}: source sprite unavailable`;
    }
    const batches = Math.ceil(quantity / Math.max(1, cost.output_quantity));
    const needed = cost.quantity * batches;
    const available = stock?.town_quantity ?? 0;
    row.className = (selected ? "selected " : "") + (selected && available < needed ? "missing" : "");
    row.append(icon, document.createTextNode(`${cost.display_name}  ${available} / ${needed}`));
    if (selected) craftable &&= available >= needed;
    if (selectedRecipe?.kind === "service") {
      row.addEventListener("click", () => {
        selectedServiceMaterialId = cost.material_id;
        renderGearCreatePop();
      });
    }
    return row;
  }));
  gearCreateQuantityValue.value = String(quantity);
  gearFrameQuantity.value = String(quantity);
  gearStorageLabel.textContent = `Remaining storage: ${Math.max(0, selectedRecipe.capacity - selectedRecipe.stock)}`;
  gearCreateSubmit.disabled = gearPopupMode !== "craft" || !craftable;
}

function renderConsumCreatePop(): void {
  const system = latestSnapshot?.village.building_system;
  if (!system || !selectedRecipe) return;
  const isServiceProduct = selectedRecipe.kind === "service";
  const isPotionRecipe = selectedRecipe.shop_id === ALCHEMIST_BUILDING_ID;
  if (!isServiceProduct && !isPotionRecipe) return;
  consumCreatePop.classList.toggle("service-product-ui", isServiceProduct || isPotionRecipe);
  consumCreatePop.classList.toggle("potion-product-ui", isPotionRecipe);
  if (isServiceProduct && (!selectedServiceMaterialId || !selectedRecipe.material_costs.some((cost) => cost.material_id === selectedServiceMaterialId))) {
    selectedServiceMaterialId = selectedRecipe.material_costs[0]?.material_id ?? null;
  }
  const selectedCost = isServiceProduct
    ? selectedRecipe.material_costs.find((cost) => cost.material_id === selectedServiceMaterialId)
    : null;
  const selectedStock = system.material_stocks.find((stock) => stock.id === selectedServiceMaterialId);
  const outputPerBatch = Math.max(1, selectedCost?.output_quantity ?? 1);
  const inputPerBatch = Math.max(1, selectedCost?.quantity ?? 1);
  const availableInput = selectedStock?.town_quantity ?? 0;
  const possibleOutput = isServiceProduct
    ? Math.floor(availableInput / inputPerBatch) * outputPerBatch
    : selectedRecipe.material_costs.reduce((maximum, cost) => {
      const available = system.material_stocks.find((stock) => stock.id === cost.material_id)?.town_quantity ?? 0;
      return Math.min(maximum, Math.floor(available / Math.max(1, cost.quantity)));
    }, Number.MAX_SAFE_INTEGER);
  const remainingCapacity = Math.max(0, selectedRecipe.capacity - system.recipes
    .filter((recipe) => recipe.shop_id === selectedRecipe?.shop_id)
    .reduce((total, recipe) => total + recipe.stock, 0));

  consumCreateTitle.textContent = `Quantity ${selectedRecipe.product_name}`;
  if (selectedRecipe.icon) consumCreateIcon.src = selectedRecipe.icon;
  else consumCreateIcon.removeAttribute("src");
  consumCreateIcon.hidden = !selectedRecipe.icon;
  consumCreateQuantity.value = String(selectedServiceQuantity);
  consumMaterialTitle.textContent = isPotionRecipe ? "Required materials" : "Select material";
  consumConversion.textContent = isPotionRecipe
    ? `Stock ${selectedRecipe.stock}/${selectedRecipe.capacity}\nProduce ${selectedServiceQuantity}/${Math.min(possibleOutput, remainingCapacity)}`
    : selectedCost
    ? `Able to produce ${outputPerBatch} ${selectedRecipe.product_name} per ${inputPerBatch} ${selectedCost.display_name}\nProduce ${selectedServiceQuantity}/${Math.min(possibleOutput, remainingCapacity)}`
    : "Material conversion unresolved";
  consumMaterialGrid.replaceChildren(...selectedRecipe.material_costs.map((cost) => {
    const stock = system.material_stocks.find((item) => item.id === cost.material_id);
    const button = document.createElement("button");
    button.type = "button";
    button.className = isPotionRecipe || cost.material_id === selectedServiceMaterialId ? "selected" : "";
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
        selectedServiceMaterialId = cost.material_id;
        renderConsumCreatePop();
      });
    } else {
      button.disabled = true;
    }
    return button;
  }));
  const batches = Math.ceil(selectedServiceQuantity / outputPerBatch);
  const neededInput = inputPerBatch * batches;
  consumCreateSubmit.disabled = isServiceProduct
    ? !selectedCost || availableInput < neededInput || selectedServiceQuantity > remainingCapacity
    : selectedRecipe.material_costs.some((cost) => {
      const available = system.material_stocks.find((stock) => stock.id === cost.material_id)?.town_quantity ?? 0;
      return available < cost.quantity * selectedServiceQuantity;
    }) || selectedServiceQuantity > remainingCapacity;
}

function renderCombatHud(state: CombatHudState): void {
  latestCombatHud = state;
  combatHud.hidden = !debugUi || !state.visible;
  if (!state.visible) return;
  element<HTMLElement>("#combat-evidence").textContent = `${state.evidenceLabel} · tick ${state.tick} · ${state.fighting ? "fighting" : "idle"}`;
  element<HTMLElement>("#hunter-state").textContent = state.hunter.state;
  element<HTMLElement>("#hunter-position").textContent = `position ${state.hunter.position}`;
  element<HTMLElement>("#hunter-hp").textContent = `${state.hunter.hp} / ${state.hunter.maxHp} HP`;
  element<HTMLElement>("#hunter-hp-fill").style.width = `${state.hunter.percent}%`;
  element<HTMLElement>("#monster-state").textContent = state.monster.state;
  element<HTMLElement>("#monster-position").textContent = `position ${state.monster.position}`;
  element<HTMLElement>("#monster-hp").textContent = `${state.monster.hp} / ${state.monster.maxHp} HP`;
  element<HTMLElement>("#monster-hp-fill").style.width = `${state.monster.percent}%`;
  element<HTMLElement>("#combat-gold").textContent = `Gold ${state.gold}`;
  element<HTMLElement>("#combat-inventory").textContent = `${state.inventory}${state.equipped ? " · item 2001 equipped" : ""}`;
  element<HTMLElement>("#combat-drops").textContent = state.drops;
  equipFixtureItem.disabled = !state.equipEligible;
  equipFixtureItem.textContent = state.equipped ? "Item 2001 equipped" : "Equip item 2001";
}

function showIntentResult(result: IntentFeedback): void {
  if (!result.accepted) {
    const reasons: Record<string, string> = {
      insufficient_materials: "Trading Post chưa có đủ nguyên liệu trong kho thị trấn.",
      material_stock_missing: "Hãy đặt request ở Trading Post trước khi craft.",
      recipe_unknown: "Recipe này chưa được bind vào dữ liệu runtime.",
      recipe_building_mismatch: "Recipe không thuộc nhà đang mở.",
      product_level_locked: "Nâng cấp công trình để mở tier trang bị này.",
      sale_building_instance_unknown: "Cần xây shop trưng bày tương ứng trước khi chế tạo.",
      product_capacity_exceeded: "Kho sản phẩm của nhà đã đầy.",
      product_stock_empty: "Shop đã hết món này.",
      sale_price_unresolved: "Giá bán của món này chưa được bind từ source.",
    };
    showPanelMessage("Không thể craft", reasons[result.reason ?? ""] ?? result.reason ?? "Please try again.");
  }
}
function showBindingBlocked(result: BindingBlockedFeedback): void {
  showPanelMessage("Coming soon", debugUi ? `${result.intent.replaceAll("_", " ")} · ${result.blockers.join(", ")}` : "This feature is still being rebuilt.");
}
function showPanelMessage(title: string, detail: string): void {
  setPanelMessage(panelMessage, title, detail);
  panelMessage.hidden = false;
  if (panelMessageTimer !== undefined) window.clearTimeout(panelMessageTimer);
  panelMessageTimer = window.setTimeout(() => {
    panelMessage.hidden = true;
    panelMessageTimer = undefined;
  }, 2800);
}
function updateConnectionStatus(status: ConnectionStatus): void {
  connectionState = status;
  const labels: Record<ConnectionStatus, string> = { connecting: "Connecting", online: "Server online", reconnecting: "Reconnecting", offline: "Offline" };
  connectionStatus.className = `connection-status ${status}`;
  connectionStatus.querySelector("span")!.textContent = labels[status];
  updateBootState();
}
function updateBootState(): void {
  enterVillage.disabled = bootRequested || !mapReady || mapLoadFailed;
  if (!bootRequested) {
    transition.hidden = true;
    bootStatus.textContent = mapLoadFailed ? "Map unavailable" : !mapReady ? "Loading map..." : connectionState === "online" ? "Ready" : "Connecting to server...";
    return;
  }
  const dispatching = connectionState === "online";
  transition.hidden = !dispatching;
  bootStatus.textContent = dispatching ? "Entering village..." : "Waiting for server...";
}
window.addEventListener("beforeunload", () => client.disconnect(), { once: true });
