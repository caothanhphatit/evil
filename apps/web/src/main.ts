import { Application } from "pixi.js";
import { WorldClient, type BindingBlockedFeedback, type ConnectionStatus, type IntentFeedback } from "./net/world-client";
import type { BottomMenuIntent, BuildingSystemSnapshot, MaterialStockSnapshot, OriginalFlowSnapshot, ShopRecipeSnapshot } from "./generated/protocol";
import { VisibleEntityWorld } from "./game/visible-world";
import type { TownBuilding } from "./assets/visible-world-release";
import { findBuildingInstanceById } from "./game/building-placement";
import { TOWN_CAMERA_CLEAR_COLOR } from "./game/scene-projection";
import { projectCombatHud, type CombatHudState } from "./ui/combat-hud";
import { nextHunterRosterOpen } from "./ui/bottom-menu-state";
import { projectResourceBar } from "./ui/resource-projection";
import { setPanelMessage } from "./ui/panel-message";
import { hunterClassTone, hunterPercent, hunterRarityPresentation, hunterWorldEntityId, projectHunterRoster, type HunterView } from "./ui/hunter-roster";
import { createHunterRosterActors } from "./ui/hunter-roster-actors";
import { createHunterInfoModal } from "./ui/hunter-info/modal";
import { createHunterWorldCommandMenu, type HunterGearEnhancementRequestIntent, type HunterWorldCommandIntent } from "./ui/hunter-world-command";
import { createOpenHunterEnhancementIntent, projectHunterEnhancementInteraction } from "./ui/hunter-enhancement-entry";
import { projectAuthoritativeMonsterField } from "./ui/monster-field";
import { projectHunterInfo } from "./ui/hunter-info/project";
import { canSubmitGearEnhancement, GEAR_ENHANCEMENT_MODES, projectGearEnhancement, type GearEnhancementMode, type GearEnhancementView } from "./ui/gear-enhancement";
import { clampQuantity, missingCraftMaterial, remainingSharedCapacity, resolveServiceMaterialId, serviceMaterialRequired, townMaterialQuantity } from "./ui/shop-crafting";
import { formatLevelCosts, listBuildingEvidence, projectBuildingEvidence } from "./content/building-evidence";
import { loadVerifiedBuildingEvidenceRegistry, type EvidenceBuildingRegistry } from "./content/building-registry";
import { originalUiLabel } from "./content/original-ui-labels";
import { BOUNTY_HUT_ROUTE, BOUNTY_TIERS } from "./routes/bounty-hut";
import { TRADING_POST_ROUTE, tradingPostDifficultyOptions, tradingPostStocksForDifficulty } from "./routes/trading-post";
import { projectProductService, productServiceRoute, type ProductServiceInput } from "./content/product-service-routes";
import { installShellViewportController } from "./ui/viewport-controller";
import { BROWSER_ACCOUNTS_STORAGE_KEY, parseBrowserAccounts, serializeBrowserAccounts, type BrowserAccount } from "./ui/browser-accounts";
import { projectEntryPresentation, type EntryPhase } from "./ui/entry-flow";
import { ACCESSORY_SHOP_BUILDING_IDS, ALL_GEAR_KINDS, ARMOR_SHOP_BUILDING_IDS, BLACKSMITH_BUILDING_IDS, BLACKSMITH_GEAR_TABS, ENHANCEMENT_FORGE_BUILDING_IDS, JEWELER_BUILDING_IDS, JEWELER_GEAR_TABS, WEAPON_SHOP_BUILDING_IDS, decodeGearCatalog, loadGearCatalog, type GearCatalogRecipe, type GearKind } from "./content/blacksmith-route";
import { ALCHEMIST_BUILDING_ID, POTION_SHOP_BUILDING_ID, isPotionBuilding } from "./content/potion-shop-routes";
import { formatNumber, t, type MessageKey } from "./i18n";
import "./styles.css";

const mount = document.querySelector<HTMLDivElement>("#app");
if (!mount) throw new Error(t("error.missing_mount"));

const releaseRoot = "/content/releases/original-flow-v1";
const originalAsset = (sourcePath: string): string => `${releaseRoot}/${sourcePath}`;
const debugUi = new URLSearchParams(window.location.search).has("debug");
type MenuAction = BottomMenuIntent | "field";
const menuItems: Array<{ action: MenuAction; label: MessageKey; icon: string; enabled: boolean }> = [
  { action: "build", label: "menu.build", icon: "sprites/menu_ic_01__6756.png", enabled: true },
  { action: "field", label: "menu.field", icon: "sprites/menu_ic_02__2060.png", enabled: true },
  { action: "character", label: "menu.hunters", icon: "sprites/menu_ic_03__6410.png", enabled: true },
  { action: "archive", label: "menu.storage", icon: "sprites/menu_ic_04__5070.png", enabled: false },
  { action: "store", label: "menu.shop", icon: "sprites/menu_ic_05__6398.png", enabled: true },
];

mount.innerHTML = `
  <main class="game-shell entry-login">
    <section id="login-screen" class="login-screen" aria-label="${t("login.aria")}">
      <section class="basic-login-card">
        <span class="basic-login-brand">${t("app.brand")}</span>
        <h1>${t("login.title")}</h1>
        <p>${t("login.subtitle")}</p>
        <label for="account-select">${t("login.profile")}</label>
        <select id="account-select"></select>
        <button id="remove-account" class="account-remove-button" type="button">${t("login.remove_profile")}</button>
        <button id="enter-village" class="enter-village" type="button">${t("login.sign_in")}</button>
        <button id="register-account-toggle" class="basic-login-link" type="button">${t("login.register_profile")}</button>
        <form id="register-account-form" class="register-account-form" hidden>
          <label for="register-account-name">${t("login.display_name")}</label><input id="register-account-name" maxlength="24" autocomplete="nickname" required />
          <label for="register-account-email">${t("login.email")}</label><input id="register-account-email" type="email" maxlength="120" autocomplete="email" required />
          <div class="boot-auth-actions"><button id="register-account-submit" class="boot-primary-button" type="submit">${t("login.create_profile")}</button><button id="register-account-cancel" class="boot-secondary-button" type="button">${t("common.cancel")}</button></div>
        </form>
        <small class="boot-account-note">${t("login.browser_only_note")}</small>
        <div class="login-readiness" hidden aria-live="polite"><i id="map-loading-fill"></i><span id="map-loading-label">${t("login.waiting_to_load")}</span></div>
      </section>
      <span id="boot-status" class="boot-status" role="status">${t("login.ready")}</span>
    </section>
    <section id="game-loading-screen" class="game-loading-screen" hidden aria-live="polite" aria-label="${t("loading.aria")}">
      <div class="game-loading-content"><span class="game-loading-kicker">${t("app.brand")}</span><h1>${t("loading.title")}</h1><p id="game-loading-status">${t("loading.sync_profile")}</p><div class="game-loading-track"><i id="game-loading-fill"></i></div><b id="game-loading-percent">0%</b><button id="game-loading-retry" type="button" hidden>${t("loading.reload")}</button></div>
    </section>
    <section id="village-screen" class="village-screen" aria-label="${t("world.village")}" aria-hidden="true">
      <div id="world-viewport" class="village-world" aria-label="${t("world.entity_world")}"></div>
      <div id="hunter-enhancement-interactions" class="hunter-enhancement-interactions" aria-label="${t("world.enhancement_interactions")}"></div>
      <header class="resource-bar" aria-label="${t("world.resources")}">
        <div class="difficulty-hud"><img data-game-src="${originalAsset("sprites/top_mon_level_01__1480.png")}" alt="${t("world.easy_difficulty")}" /><b id="world-mode-label" class="sr-only">${t("world.easy")}</b></div>
        <div class="resource-ledger">
          <div class="resource-line"><img data-game-src="${originalAsset("sprites/top_ic_01_gold_24__4677.png")}" alt="${t("world.gold")}" /><b id="gold-amount">0</b></div>
          <div class="resource-line unresolved"><img data-game-src="${originalAsset("sprites/top_ic_02_gem_24__4214.png")}" alt="${t("world.gem")}" /><b>--</b></div>
          <div class="resource-line unresolved"><img data-game-src="${originalAsset("sprites/top_ic_03_element_24__1412.png")}" alt="${t("world.elemental")}" /><b>--</b></div>
          <div class="resource-line unresolved"><img data-game-src="${originalAsset("sprites/top_ic_04_book_24__3078.png")}" alt="${t("world.book")}" /><b>--</b></div>
        </div>
      </header>
      <nav class="top-quick-actions" aria-label="${t("world.shortcuts")}">
        <button type="button" disabled title="${t("world.book_unavailable")}"><img data-game-src="${originalAsset("sprites/top_ic_book__3217.png")}" alt="${t("world.book")}" /></button>
        <button type="button" disabled title="${t("world.rank_unavailable")}"><img data-game-src="${originalAsset("sprites/top_ic_rank__5074.png")}" alt="${t("world.rank")}" /></button>
        <button type="button" data-action="character" title="${t("world.hunters")}"><img data-game-src="${originalAsset("sprites/top_ic_man__5368.png")}" alt="${t("world.hunters")}" /><b id="hunter-population">0/8</b></button>
        <button type="button" data-action="build" title="${t("world.construct")}"><img data-game-src="${originalAsset("sprites/menu_ic_01__6756.png")}" alt="${t("world.construct")}" /></button>
        <button type="button" disabled title="${t("world.settings_unavailable")}"><img data-game-src="${originalAsset("sprites/top_ic_setting__4198.png")}" alt="${t("world.settings")}" /></button>
      </nav>
      <button class="quest-shortcut" type="button" disabled title="${t("world.quest_unavailable")}"><img data-game-src="${originalAsset("sprites/top_ic_quest__4944.png")}" alt="${t("world.quests")}" /></button>
      <button id="field-back" class="field-back" type="button">${t("world.return_village")}</button>
      <aside id="evidence-diagnostics" class="evidence-diagnostics" aria-live="polite" hidden><b></b><span></span></aside>
      <section id="combat-hud" class="combat-hud" aria-label="${t("combat.aria")}" hidden>
        <header><b>${t("combat.title")}</b><span id="combat-evidence"></span></header>
        <div class="combatant"><span>${t("combat.hunter", { id: 1 })} · <i id="hunter-state"></i> · <i id="hunter-position"></i></span><div class="hp-track"><i id="hunter-hp-fill"></i></div><small id="hunter-hp"></small></div>
        <div class="combatant"><span>${t("combat.monster", { id: 1001 })} · <i id="monster-state"></i> · <i id="monster-position"></i></span><div class="hp-track monster"><i id="monster-hp-fill"></i></div><small id="monster-hp"></small></div>
        <div class="combat-ledger"><span id="combat-gold"></span><span id="combat-inventory"></span><span id="combat-drops"></span></div>
        <button id="equip-fixture-item" type="button">${t("combat.equip_item", { id: 2001 })}</button>
      </section>
      <div id="panel-message" class="panel-message" aria-live="polite" hidden><b></b><span></span></div>
      <section id="building-panel" class="building-panel source-popup" hidden aria-label="${t("building.aria")}">
        <button id="building-panel-close" class="source-red-button" type="button">${t("common.close")}</button>
        <b id="building-name">${t("building.default_name")}</b>
        <div class="building-source-header"><img id="building-preview" alt="" hidden /><div><span id="building-level"></span><small id="building-feature"></small><small id="building-condition"></small></div></div>
        <div id="building-level-contract" class="building-level-contract"></div>
        <div id="building-catalog" class="building-catalog"></div>
        <div class="building-actions"><button id="building-construct" class="source-green-button" type="button" disabled>${t("building.construct")}</button><button id="building-upgrade" class="source-green-button" type="button" disabled>${t("common.upgrade")}</button><button id="building-use" class="source-green-button" type="button" disabled>${t("common.open")}</button><button id="building-sell" type="button" hidden>${t("common.sell")}</button></div>
      </section>
      <section id="bounty-quest-pop" class="bounty-quest-pop source-popup" hidden aria-label="${t("bounty.popup_aria")}">
        <button id="bounty-close" class="source-red-button" type="button">${t("common.close")}</button>
        <b id="bounty-title">${t("common.level_short", { level: 1 })} ${t("bounty.title")}</b><i class="source-popup-line"></i>
        <p class="bounty-description">${t("bounty.description")}</p>
        <div id="bounty-tier-tabs" class="bounty-tier-tabs"></div>
        <div id="bounty-quest-list" class="bounty-quest-list"><p class="bounty-unresolved">${t("bounty.rows_unavailable")}</p></div>
        <div class="bounty-upgrade-hint">${t("bounty.upgrade_hint")}<br /><em>${originalUiLabel("buildpop_9", "vi", [5])}</em></div>
        <div class="source-popup-actions"><button id="bounty-upgrade" class="source-green-button" type="button">${t("common.upgrade")}</button><button id="bounty-close-bottom" class="source-red-button" type="button">${t("common.close")}</button></div>
      </section>
      <section id="gear-create-pop" class="gear-create-pop source-popup" hidden aria-label="${t("craft.gear_popup_aria")}">
        <b id="gear-create-title">${t("craft.gear_create")}</b><i class="source-popup-line"></i>
        <div class="gear-create-product">
          <div class="gear-frame"><img id="gear-create-icon" alt="" /><i aria-hidden="true"></i><output id="gear-frame-quantity">1</output></div>
          <div class="gear-product-side">
            <div class="gear-create-meta"><strong id="gear-create-name"></strong><span id="gear-create-price"></span></div>
            <div id="gear-quantity-row" class="gear-quantity-controls">
              <div class="quantity-stepper">
                <button id="gear-quantity-minus" class="gear-round-button minus" type="button" aria-label="${t("craft.decrease_quantity")}"></button>
                <input id="gear-create-quantity" type="number" min="1" max="1000" step="1" inputmode="numeric" pattern="[0-9]*" value="1" aria-label="${t("craft.production_quantity")}" />
                <button id="gear-quantity-plus" class="gear-round-button plus" type="button" aria-label="${t("craft.increase_quantity")}"></button>
              </div>
              <div class="quantity-step-buttons"><button type="button" data-gear-delta="1">+1</button><button type="button" data-gear-delta="10">+10</button><button type="button" data-gear-delta="100">+100</button><button type="button" data-gear-delta="1000">+1000</button></div>
            </div>
          </div>
        </div>
        <div id="gear-create-description" class="gear-create-description"><button id="gear-lock" type="button" disabled>${t("common.lock")}</button></div>
        <h3 id="gear-material-title">${t("craft.required_materials")}</h3>
        <div id="gear-material-costs" class="gear-material-costs"></div>
        <strong id="gear-storage-label" class="gear-storage-label"></strong>
        <div class="source-popup-actions"><button id="gear-create-submit" class="source-green-button" type="button">${t("common.create")}</button><button id="gear-create-sell" class="source-green-button" type="button">${t("common.dismantle")}</button><button id="gear-create-close" class="source-red-button" type="button">${t("common.close")}</button></div>
      </section>
      <section id="consum-create-pop" class="consum-create-pop source-popup" hidden aria-label="${t("craft.consumable_popup_aria")}">
        <b id="consum-create-title">${t("craft.quantity_title")}</b><i class="source-popup-line"></i>
        <div class="consum-quantity-panel">
          <div class="consum-product-frame"><img id="consum-create-icon" alt="" /><span id="consum-create-icon-placeholder" class="product-icon-unresolved" aria-label="${t("craft.product_image_unavailable")}">?</span><output id="consum-create-quantity">1</output></div>
          <div class="consum-quantity-controls">
            <span>${t("common.quantity")}</span>
            <div class="quantity-stepper">
              <button id="consum-minus" class="consum-round-button minus" type="button" aria-label="${t("craft.decrease_quantity")}"></button>
              <input id="consum-create-quantity-input" type="number" min="1" max="1000" step="1" inputmode="numeric" pattern="[0-9]*" value="1" aria-label="${t("craft.production_quantity")}" />
              <button id="consum-plus" class="consum-round-button plus" type="button" aria-label="${t("craft.increase_quantity")}"></button>
            </div>
            <div class="quantity-step-buttons"><button type="button" data-consum-delta="1">+1</button><button type="button" data-consum-delta="10">+10</button><button type="button" data-consum-delta="100">+100</button><button type="button" data-consum-delta="1000">+1000</button></div>
          </div>
          <p id="consum-conversion"></p>
        </div>
        <h3 id="consum-material-title">${t("craft.select_material")}</h3>
        <div id="consum-material-grid" class="consum-material-grid"></div>
        <div class="source-popup-actions"><button id="consum-create-submit" class="source-green-button" type="button">${t("common.produce")}</button><button id="consum-create-close" class="source-red-button" type="button">${t("common.close")}</button></div>
      </section>
      <button id="connection-status" class="connection-status connecting" type="button" aria-label="${t("world.server_status")}"><i></i><span>${t("world.connecting")}</span></button>
      <span id="fps-counter" class="fps-counter" aria-live="off">FPS --</span>
    </section>
    <section id="roster-screen" class="roster-screen" aria-label="${t("roster.aria")}" aria-hidden="true">
      <img class="roster-background" data-game-src="/content/releases/visible-world-v1/maps/map_new01.png" alt="" />
      <section class="hunter-roster-panel bottom-menu-panel" aria-label="${t("roster.management")}">
        <header class="hunter-roster-header"><div class="hunter-roster-actions"><button type="button" disabled>${t("roster.place_hunting_grounds")}</button><button type="button" disabled>${t("roster.sort")}</button></div><div class="hunter-roster-heading"><b>${t("roster.title")}</b><span id="hunter-capacity">0 / 8</span></div><button id="roster-back" class="source-red-button" type="button" aria-label="${t("roster.close_aria")}">${t("common.close")}</button></header>
        <div class="hunter-roster-body"><div id="hunter-active-list" class="hunter-card-grid"></div></div>
        <footer id="hunter-roster-status" class="hunter-roster-status"></footer>
      </section>
    </section>
    <nav id="bottom-menu" class="bottom-menu persistent-bottom-menu" aria-label="${t("menu.aria")}" hidden>${menuItems.map((item) => `<button class="menu-button" type="button" data-action="${item.action}" ${item.enabled ? "" : `disabled title="${t("common.feature_in_development")}"`}><span class="menu-icon"><img data-game-src="${originalAsset(item.icon)}" alt="" /></span><b>${t(item.label)}</b></button>`).join("")}</nav>
    <div id="loading-transition" class="loading-transition" hidden><img data-game-src="${originalAsset("sprites/cloud_loading_btn__4266.png")}" alt="" /><span>${t("loading.loading")}</span></div>
  </main>`;

const gameShell = mount.querySelector<HTMLElement>(".game-shell");
if (!gameShell) throw new Error(t("error.missing_shell"));
installShellViewportController(gameShell);

function element<T extends HTMLElement>(selector: string): T { const value = document.querySelector<T>(selector); if (!value) throw new Error(t("error.missing_element", { selector })); return value; }
const loginScreen = element<HTMLElement>("#login-screen");
const villageScreen = element<HTMLElement>("#village-screen");
const rosterScreen = element<HTMLElement>("#roster-screen");
const bottomMenu = element<HTMLElement>("#bottom-menu");
const hunterCapacity = element<HTMLElement>("#hunter-capacity");
const hunterActiveList = element<HTMLElement>("#hunter-active-list");
const hunterRosterStatus = element<HTMLElement>("#hunter-roster-status");
const transition = element<HTMLElement>("#loading-transition");
const panelMessage = element<HTMLElement>("#panel-message");
const connectionStatus = element<HTMLButtonElement>("#connection-status");
const fpsCounter = element<HTMLElement>("#fps-counter");
const worldViewport = element<HTMLElement>("#world-viewport");
const hunterEnhancementInteractions = element<HTMLElement>("#hunter-enhancement-interactions");
const fieldBack = element<HTMLButtonElement>("#field-back");
const worldModeLabel = element<HTMLElement>("#world-mode-label");
const goldAmount = element<HTMLElement>("#gold-amount");
const hunterPopulation = element<HTMLElement>("#hunter-population");
const enterVillage = element<HTMLButtonElement>("#enter-village");
const accountSelect = element<HTMLSelectElement>("#account-select");
const removeAccountButton = element<HTMLButtonElement>("#remove-account");
const registerAccountToggle = element<HTMLButtonElement>("#register-account-toggle");
const registerAccountForm = element<HTMLFormElement>("#register-account-form");
const registerAccountName = element<HTMLInputElement>("#register-account-name");
const registerAccountEmail = element<HTMLInputElement>("#register-account-email");
const registerAccountCancel = element<HTMLButtonElement>("#register-account-cancel");
const gameLoadingScreen = element<HTMLElement>("#game-loading-screen");
const gameLoadingStatus = element<HTMLElement>("#game-loading-status");
const gameLoadingFill = element<HTMLElement>("#game-loading-fill");
const gameLoadingPercent = element<HTMLElement>("#game-loading-percent");
const gameLoadingRetry = element<HTMLButtonElement>("#game-loading-retry");
const bootStatus = element<HTMLElement>("#boot-status");
const loginReadiness = element<HTMLElement>(".login-readiness");
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
let connectionState: ConnectionStatus = "connecting";
let bootRequested = false;
let entryPhase: EntryPhase = "login";
let runtimeStarted = false;
let gameLoadingHideTimer: number | undefined;
let gameLoadingTimeoutTimer: number | undefined;
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
let selectedEnhancementGearKey: string | null = null;
let selectedEnhancementMode: GearEnhancementMode = "single";
let enhancementView: GearEnhancementView = "select";
let enhancementHunterId: number | null = null;
let selectedEnhancementOptionalMaterialIds: string[] = [];
let gearPopupMode: "craft" | "detail" = "craft";
let selectedBountyTier = 0;
let selectedTradingPostDifficulty = 0;
let selectedTradingRequest: MaterialStockSnapshot | null = null;
let selectedTradingRequestQuantity = 1;
let tradingRequestPending = false;
let buildingEvidenceRegistry: EvidenceBuildingRegistry | null = null;
let buildingEvidenceError: string | null = null;
let popupInteractionActive = false;
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

const client = new WorldClient(
  renderSnapshot,
  updateConnectionStatus,
  showIntentResult,
  showBindingBlocked,
  undefined,
  { onWorldFrame: renderWorldFrame },
);
const hunterInfoActions = { useSkill: useHunterSkillFromInfo };
const hunterInfoModal = createHunterInfoModal(rosterScreen, hunterInfoActions);
const worldHunterInfoModal = createHunterInfoModal(villageScreen, hunterInfoActions);
const hunterWorldCommandMenu = createHunterWorldCommandMenu(villageScreen, {
  onInfo: showWorldHunterInfo,
  onIntent: handleHunterWorldCommandIntent,
  onEnhancementRequest: handleHunterEnhancementRequest,
  onRelease: (entityId) => {
    releasedWorldHunterEntityId = entityId;
    world?.setSelectedEntity(null);
    worldHunterInfoModal.close();
  },
  onUnavailable: (category) => showPanelMessage(t("error.command_unbound"), category),
});
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
let browserAccounts: BrowserAccount[] = parseBrowserAccounts(window.localStorage.getItem(BROWSER_ACCOUNTS_STORAGE_KEY));
function renderBrowserAccounts(): void {
  accountSelect.replaceChildren(...browserAccounts.map((account) => {
    const option = document.createElement("option");
    option.value = account.id;
    option.textContent = account.kind === "demo"
      ? t("login.profile_option", { name: account.displayName, email: t("login.demo_kind") })
      : t("login.profile_option", { name: account.displayName, email: account.email });
    return option;
  }));
  const saved = window.localStorage.getItem("evil.browser.selected-account.v1");
  accountSelect.value = browserAccounts.some((account) => account.id === saved) ? saved! : browserAccounts[0]!.id;
}
renderBrowserAccounts();
accountSelect.addEventListener("change", () => window.localStorage.setItem("evil.browser.selected-account.v1", accountSelect.value));
removeAccountButton.addEventListener("click", () => {
  if (accountSelect.value === "demo-hunter-lab") return;
  browserAccounts = browserAccounts.filter((account) => account.id !== accountSelect.value);
  window.localStorage.setItem(BROWSER_ACCOUNTS_STORAGE_KEY, serializeBrowserAccounts(browserAccounts));
  renderBrowserAccounts();
});
registerAccountToggle.addEventListener("click", () => {
  registerAccountForm.hidden = !registerAccountForm.hidden;
  if (!registerAccountForm.hidden) registerAccountName.focus();
});
registerAccountCancel.addEventListener("click", () => { registerAccountForm.hidden = true; });
registerAccountForm.addEventListener("submit", (event) => {
  event.preventDefault();
  const displayName = registerAccountName.value.trim();
  const email = registerAccountEmail.value.trim().toLowerCase();
  if (displayName.length < 2 || !email.includes("@")) return;
  const account: BrowserAccount = { id: crypto.randomUUID(), displayName, email, kind: "browser", createdAt: new Date().toISOString() };
  browserAccounts = [...browserAccounts, account];
  window.localStorage.setItem(BROWSER_ACCOUNTS_STORAGE_KEY, serializeBrowserAccounts(browserAccounts));
  window.localStorage.setItem("evil.browser.selected-account.v1", account.id);
  renderBrowserAccounts();
  registerAccountForm.reset();
  registerAccountForm.hidden = true;
});
enterVillage.disabled = false;
enterVillage.addEventListener("click", () => {
  if (mapLoadFailed || entryPhase !== "login") return;
  window.localStorage.setItem("evil.browser.selected-account.v1", accountSelect.value);
  entryPhase = "loading";
  gameLoadingScreen.hidden = false;
  gameLoadingStatus.textContent = t("loading.account", { account: accountSelect.selectedOptions[0]?.textContent ?? t("loading.profile_fallback") });
  gameLoadingFill.style.width = "3%";
  gameLoadingPercent.textContent = "3%";
  bootRequested = true;
  gameLoadingRetry.hidden = true;
  if (gameLoadingTimeoutTimer !== undefined) window.clearTimeout(gameLoadingTimeoutTimer);
  gameLoadingTimeoutTimer = window.setTimeout(() => {
    if (entryPhase === "loading") failGameLoading(t("loading.server_timeout"));
  }, 30_000);
  updateBootState();
  startGameRuntime();
  client.completeBoot();
});

function startGameRuntime(): void {
  if (runtimeStarted) return;
  runtimeStarted = true;
  document.querySelectorAll<HTMLImageElement>("img[data-game-src]").forEach((image) => {
    image.src = image.dataset.gameSrc ?? "";
    delete image.dataset.gameSrc;
  });
  client.connect();
  void initializeBuildingEvidence();
  void initializeWorld().catch((error: unknown) => {
    console.error("Failed to initialize the visible world.", error);
    transition.hidden = true;
    failGameLoading(t("loading.world_failure"));
  });
}

function failGameLoading(message: string): void {
  mapLoadFailed = true;
  if (gameLoadingTimeoutTimer !== undefined) window.clearTimeout(gameLoadingTimeoutTimer);
  gameLoadingTimeoutTimer = undefined;
  gameLoadingStatus.textContent = message;
  gameLoadingPercent.textContent = t("loading.error_title");
  gameLoadingFill.style.width = "100%";
  gameLoadingRetry.hidden = false;
  updateBootState();
}

gameLoadingRetry.addEventListener("click", () => window.location.reload());

async function preloadUiAssets(): Promise<void> {
  const images = [...document.images].map((image) => {
    if (image.complete) return Promise.resolve();
    return new Promise<void>((resolve) => {
      image.addEventListener("load", () => resolve(), { once: true });
      image.addEventListener("error", () => resolve(), { once: true });
    });
  });
  await Promise.allSettled(images);
  await document.fonts.ready;
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
      renderHunterRoster(latestSnapshot);
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
    buildingPanelMode = "construct";
    buildingPanel.hidden = false;
    client.selectBottomMenu("build");
    renderBuildingSystem(latestSnapshot);
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
buildingPanelClose.addEventListener("click", () => {
  buildingPanel.hidden = true;
  selectedTradingRequest = null;
  tradingRequestPending = false;
  enhancementView = "select";
  selectedEnhancementGearKey = null;
  enhancementHunterId = null;
  selectedEnhancementOptionalMaterialIds = [];
  if (selectedMenuAction === "build") selectedMenuAction = null;
  bottomMenu.querySelector('[data-action="build"]')?.classList.remove("selected");
});
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
  } else if (route === "gear-enhancement") {
    const selected = latestSnapshot?.hunter_roster.active_hunters.flatMap((hunter) => (
      hunter.gear_enhancements.map((gear) => ({ hunter, gear, key: gear.instance_id ?? `${hunter.hunter_id}:${gear.product_id}` }))
    )).find((row) => row.key === selectedEnhancementGearKey);
    if (!selected) {
      showPanelMessage(t("enhancement.no_selection_title"), t("enhancement.no_selection_detail"));
      return;
    }
    if (enhancementView === "select") {
      enhancementView = "configure";
      renderBuildingSystem(latestSnapshot);
      return;
    }
    const preview = projectGearEnhancement(selected.gear, selectedEnhancementMode);
    if (!canSubmitGearEnhancement(preview)) {
      showPanelMessage(t("enhancement.unavailable_title"), t("enhancement.evidence_unverified"));
      return;
    }
    if (!selected.gear.instance_id) {
      showPanelMessage(t("enhancement.unavailable_title"), t("enhancement.instance_unavailable"));
      return;
    }
    client.enhanceHunterGear(selected.hunter.hunter_id, selected.gear.instance_id, selectedEnhancementMode, selectedEnhancementOptionalMaterialIds);
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
function renderTradingRequestEditor(): void {
  const stock = selectedTradingRequest;
  if (!stock) return;
  const editor = document.createElement("section");
  editor.id = "trading-request-editor";
  editor.className = "trading-request-editor";
  editor.setAttribute("aria-label", t("trading.request_editor_aria"));

  const product = document.createElement("div");
  product.className = "trading-request-product";
  const frame = document.createElement("div");
  frame.className = "trading-request-frame";
  const icon = document.createElement("img");
  icon.alt = "";
  icon.src = stock.icon || resourceIconPath(stock.id) || "";
  icon.hidden = !icon.src;
  const count = document.createElement("output");
  count.textContent = String(selectedTradingRequestQuantity);
  frame.append(icon, count);

  const controls = document.createElement("div");
  controls.className = "trading-request-controls";
  const label = document.createElement("span");
  label.textContent = t("common.quantity");
  const stepper = document.createElement("div");
  stepper.className = "quantity-stepper";
  const minus = document.createElement("button");
  minus.id = "trading-request-minus";
  minus.className = "consum-round-button minus";
  minus.type = "button";
  minus.setAttribute("aria-label", t("craft.decrease_quantity"));
  const input = document.createElement("input");
  input.id = "trading-request-quantity-input";
  input.type = "number";
  input.min = "1";
  input.max = "10000";
  input.step = "1";
  input.inputMode = "numeric";
  input.value = String(selectedTradingRequestQuantity);
  input.setAttribute("aria-label", t("trading.quantity_aria"));
  const plus = document.createElement("button");
  plus.id = "trading-request-plus";
  plus.className = "consum-round-button plus";
  plus.type = "button";
  plus.setAttribute("aria-label", t("craft.increase_quantity"));
  stepper.append(minus, input, plus);

  const quickSteps = document.createElement("div");
  quickSteps.className = "quantity-step-buttons";
  [1, 10, 100, 1000].forEach((delta) => {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = `+${delta}`;
    button.addEventListener("click", () => {
      selectedTradingRequestQuantity = clampQuantity(selectedTradingRequestQuantity + delta, 1, 10_000);
      renderTradingRequestEditor();
    });
    quickSteps.append(button);
  });
  const max = document.createElement("button");
  max.type = "button";
  max.textContent = "∞";
  max.addEventListener("click", () => {
    selectedTradingRequestQuantity = 10_000;
    renderTradingRequestEditor();
  });
  quickSteps.append(max);
  controls.append(label, stepper, quickSteps);
  product.append(frame, controls);

  const name = document.createElement("strong");
  name.id = "trading-request-name";
  name.textContent = stock.display_name;
  const total = document.createElement("p");
  total.innerHTML = `${t("trading.request_quantity", { quantity: selectedTradingRequestQuantity })}<br />${t("trading.estimated_total")}<br /><span>${formatNumber(stock.unit_price * selectedTradingRequestQuantity)} ${t("common.gold")}</span>`;
  const actions = document.createElement("div");
  actions.className = "source-popup-actions";
  const submit = document.createElement("button");
  submit.id = "trading-request-submit";
  submit.className = "source-green-button";
  submit.type = "button";
  submit.disabled = tradingRequestPending || !selectedBuildingInstanceId;
  submit.textContent = tradingRequestPending ? t("common.requesting") : t("common.request");
  const back = document.createElement("button");
  back.id = "trading-request-close";
  back.className = "source-red-button";
  back.type = "button";
  back.textContent = t("common.back");

  const updateQuantity = (value: string | number): void => {
    selectedTradingRequestQuantity = clampQuantity(value, 1, 10_000);
    renderTradingRequestEditor();
  };
  minus.addEventListener("click", () => updateQuantity(selectedTradingRequestQuantity - 1));
  plus.addEventListener("click", () => updateQuantity(selectedTradingRequestQuantity + 1));
  input.addEventListener("change", () => updateQuantity(input.value));
  submit.addEventListener("click", () => {
    if (!selectedBuildingInstanceId || !selectedTradingRequest) {
      showPanelMessage(t("error.cannot_request"), t("error.trading_unavailable"));
      return;
    }
    if (!client.setMaterialRequest(selectedBuildingInstanceId, selectedTradingRequest.id, selectedTradingRequestQuantity)) {
      showPanelMessage(t("error.request_send_failed"), t("error.connection_not_ready"));
      return;
    }
    tradingRequestPending = true;
    renderTradingRequestEditor();
  });
  back.addEventListener("click", () => {
    selectedTradingRequest = null;
    tradingRequestPending = false;
    renderBuildingSystem(latestSnapshot);
  });
  actions.append(submit, back);
  editor.append(product, name, total, actions);
  buildingCatalog.replaceChildren(editor);
}
gearCreateQuantity.addEventListener("input", () => {
  if (gearCreateQuantity.value === "") return;
  gearCreateQuantity.value = String(clampQuantity(gearCreateQuantity.value, 1, 1000));
  renderGearCreatePop();
});
gearCreateQuantity.addEventListener("change", () => {
  gearCreateQuantity.value = String(clampQuantity(gearCreateQuantity.value, 1, 1000));
  renderGearCreatePop();
});
function changeGearQuantity(delta: number): void {
  gearCreateQuantity.value = String(clampQuantity(Number(gearCreateQuantity.value) + delta, 1, 1000));
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
  selectedServiceQuantity = clampQuantity(selectedServiceQuantity + delta, 1, 1000);
  renderConsumCreatePop();
}
consumMinus.addEventListener("click", () => changeServiceQuantity(-1));
consumPlus.addEventListener("click", () => changeServiceQuantity(1));
document.querySelectorAll<HTMLButtonElement>("[data-consum-delta]").forEach((button) => {
  button.addEventListener("click", () => changeServiceQuantity(Number(button.dataset.consumDelta)));
});
consumCreateQuantityInput.addEventListener("input", () => {
  if (consumCreateQuantityInput.value === "") return;
  selectedServiceQuantity = clampQuantity(consumCreateQuantityInput.value, 1, 1000);
  renderConsumCreatePop();
});
consumCreateQuantityInput.addEventListener("change", () => {
  selectedServiceQuantity = clampQuantity(consumCreateQuantityInput.value, 1, 1000);
  renderConsumCreatePop();
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
element<HTMLButtonElement>("#roster-back").addEventListener("click", () => {
  setHunterRosterVisibility(false);
});
fieldBack.addEventListener("click", () => {
  hunterWorldCommandMenu.close();
  worldHunterInfoModal.close();
  client.navigateBack();
});
equipFixtureItem.addEventListener("click", () => {
  if (latestCombatHud?.equipEligible) client.equipHunterItem(1, 2001);
});
connectionStatus.addEventListener("click", () => client.requestResync());
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
    buildingEvidenceError = debugUi && error instanceof Error ? error.message : t("diagnostics.building_load_failed");
    console.error("Failed to load verified building evidence.", error);
  }
}

async function initializeWorld(): Promise<void> {
  const app = new Application();
  await app.init({ resizeTo: worldViewport, backgroundColor: TOWN_CAMERA_CLEAR_COLOR, backgroundAlpha: 1, antialias: false, autoDensity: true, resolution: Math.min(devicePixelRatio, 2) });
  worldViewport.appendChild(app.canvas);
  const visibleWorld = new VisibleEntityWorld((entityId, screenPoint) => {
    releasedWorldHunterEntityId = null;
    const entity = latestSnapshot?.world.entities.find((candidate) => candidate.descriptor.entity_id === entityId);
    if (!entity || entity.descriptor.kind === "monster") return;
    if (!client.selectEntity(entityId)) return;
    if (entity?.descriptor.kind === "hunter") {
      const hunter = hunterForWorldEntity(latestSnapshot, entityId);
      hunterWorldCommandMenu.selectHunter({
        entityId,
        displayName: hunter?.name ?? entityId,
        screenPoint: screenPoint ?? { x: worldViewport.clientWidth / 2, y: worldViewport.clientHeight / 2 },
      });
      return;
    }
    hunterWorldCommandMenu.close();
    worldHunterInfoModal.close();
    showPanelMessage(t("error.entity_selected"), entityId);
  }, (instance, visual) => {
    hunterWorldCommandMenu.close();
    worldHunterInfoModal.close();
    if (ENHANCEMENT_FORGE_BUILDING_IDS.includes(instance.building_id as typeof ENHANCEMENT_FORGE_BUILDING_IDS[number])) {
      showPanelMessage(t("enhancement.forge_title"), t("enhancement.forge_instruction"));
      return;
    }
    selectedBuildingId = instance.building_id;
    selectedBuildingInstanceId = instance.instance_id;
    selectedBuildingVisual = visual;
    buildingPanelMode = "building";
    const evidence = projectBuildingEvidence(buildingEvidenceRegistry, instance.building_id);
    if (!evidence) {
      buildingPanel.hidden = true;
      showPanelMessage(t("error.building_binding_unresolved"), instance.building_id);
      return;
    }
    buildingPanel.hidden = false;
    renderBuildingSystem(latestSnapshot);
  }, (regionId, nextLevel) => {
    if (!client.setMonsterRegionDensity(regionId, nextLevel)) return;
    showPanelMessage(t("error.monster_density"), `${regionId}: ${["I", "II", "III"][nextLevel - 1] ?? nextLevel}`);
  });
  const diagnostics = await visibleWorld.initialize((loaded, total) => {
    const percent = total ? Math.round((loaded / total) * 100) : 0;
    const gamePercent = Math.min(88, Math.max(5, Math.round(5 + percent * .83)));
    mapLoadingFill.style.width = `${percent}%`;
    mapLoadingLabel.textContent = t("loading.map_progress", { percent });
    gameLoadingFill.style.width = `${gamePercent}%`;
    gameLoadingPercent.textContent = `${gamePercent}%`;
    gameLoadingStatus.textContent = t("loading.world_assets");
  });
  mapLoadingLabel.textContent = t("loading.preparing_game");
  mapLoadingFill.style.width = "100%";
  gameLoadingFill.style.width = "92%";
  gameLoadingPercent.textContent = "92%";
  gameLoadingStatus.textContent = t("loading.preparing_hunters");
  await Promise.all([hunterRosterActors.preload(), preloadUiAssets()]);
  if (debugUi) {
    evidenceDiagnostics.hidden = false;
    setPanelMessage(evidenceDiagnostics, diagnostics.fixture ? t("diagnostics.fixture") : t("diagnostics.runtime"), t("diagnostics.unresolved", { items: diagnostics.unresolved.join(", ") }));
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
    if (latestSnapshot) renderHunterEnhancementInteractions(latestSnapshot);
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
    if (latestSnapshot) renderHunterEnhancementInteractions(latestSnapshot);
  }, { passive: false });
  let fpsUpdatedAt = performance.now();
  app.ticker.add(() => {
    visibleWorld.tick();
    const now = performance.now();
    if (now - fpsUpdatedAt < 500) return;
    fpsUpdatedAt = now;
    fpsCounter.textContent = t("world.fps", { value: Math.round(app.ticker.FPS) });
  });
  world = visibleWorld;
  if (latestSnapshot) {
    syncBuildingPresentation(visibleWorld, latestSnapshot);
    visibleWorld.setMode(latestSnapshot.screen === "field" ? "field" : "village");
    visibleWorld.setMonsterDensityLevels(projectAuthoritativeMonsterField(latestSnapshot.monster_world).farms);
    visibleWorld.update(
      latestSnapshot.world.entities,
      latestSnapshot.world.visual_tick,
      latestSnapshot.world.combat_presentations,
      latestSnapshot.world.drops,
    );
  }
  // Generate the static render cache and paint the initial authoritative frame
  // while the boot screen still covers the world canvas.
  app.render();
  await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
  app.render();
  loginReadiness.hidden = true;
  mapReady = true;
  if (entryPhase === "loading" && latestSnapshot) scheduleGameReveal(latestSnapshot);
  updateBootState();
  window.addEventListener("beforeunload", () => {
    resizeObserver.disconnect();
    visibleWorld.destroy();
    app.destroy(true);
  }, { once: true });
}

function renderSnapshot(snapshot: OriginalFlowSnapshot): void {
  latestSnapshot = snapshot;
  scheduleGameReveal(snapshot);
  world?.setMode(snapshot.screen === "field" ? "field" : "village");
  world?.setMonsterDensityLevels(projectAuthoritativeMonsterField(snapshot.monster_world).farms);
  if (world) syncBuildingPresentation(world, snapshot);
  world?.update(snapshot.world.entities, snapshot.world.visual_tick, snapshot.world.combat_presentations, snapshot.world.drops);
  renderHunterEnhancementInteractions(snapshot);
  if (snapshot.world.selected_entity_id !== releasedWorldHunterEntityId) releasedWorldHunterEntityId = null;
  world?.setSelectedEntity(snapshot.world.selected_entity_id === releasedWorldHunterEntityId
    ? null
    : snapshot.world.selected_entity_id);
  updateBootState();
  const village = syncEntryScreens(snapshot);
  const roster = hunterRosterOpen;
  const commandHunterEntityId = hunterWorldCommandMenu.selectedEntityId();
  const commandHunter = commandHunterEntityId ? hunterForWorldEntity(snapshot, commandHunterEntityId) : null;
  const commandHunterVisible = commandHunterEntityId === null || snapshot.world.entities.some((entity) => (
    entity.descriptor.entity_id === commandHunterEntityId && entity.descriptor.kind === "hunter"
  ));
  if (!village || roster || !commandHunterVisible) {
    hunterWorldCommandMenu.close();
    worldHunterInfoModal.close();
  } else if (worldHunterInfoModal.visible() && commandHunter) {
    worldHunterInfoModal.refresh(projectHunterInfo(rawHunterFor(snapshot, commandHunter), commandHunter));
  }
  const activeMenuAction = roster ? "character" : selectedMenuAction;
  bottomMenu.querySelectorAll<HTMLElement>("[data-action]").forEach((item) => {
    item.classList.toggle("selected", item.dataset.action === activeMenuAction);
  });
  const now = performance.now();
  if (!hunterRosterPrimed && snapshot.screen === "village") {
    renderHunterRoster(snapshot);
    hunterRosterPrimed = true;
    nextHunterRosterRefreshAt = now + 500;
  } else if (roster && now >= nextHunterRosterRefreshAt) {
    renderHunterRoster(snapshot);
    hunterRosterPrimed = true;
    nextHunterRosterRefreshAt = now + 500;
  } else if (!roster && hunterInfoModal.visible()) hunterInfoModal.close();
  syncEnhancementTaskView(snapshot);
  worldModeLabel.textContent = snapshot.screen === "field" ? t("world.hunt") : t("world.easy");
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
  const population = projectHunterRoster(snapshot, null);
  hunterPopulation.textContent = `${population.active.length}/${population.capacity}`;
  fieldBack.hidden = snapshot.screen !== "field";
  renderCombatHud(projectCombatHud(snapshot.screen, snapshot.migration_fixture_combat));
}

function scheduleGameReveal(snapshot: OriginalFlowSnapshot): void {
  if (!mapReady || entryPhase !== "loading" || snapshot.screen === "boot") return;
  bootRequested = false;
  gameLoadingFill.style.width = "100%";
  gameLoadingPercent.textContent = "100%";
  gameLoadingStatus.textContent = t("loading.settlement_ready");
  if (gameLoadingTimeoutTimer !== undefined) window.clearTimeout(gameLoadingTimeoutTimer);
  gameLoadingTimeoutTimer = undefined;
  if (gameLoadingHideTimer !== undefined) return;
  gameLoadingHideTimer = window.setTimeout(() => {
    entryPhase = "game";
    gameLoadingHideTimer = undefined;
    updateBootState();
    syncEntryScreens(latestSnapshot ?? snapshot);
  }, 850);
}

function syncEntryScreens(snapshot: OriginalFlowSnapshot): boolean {
  const entry = projectEntryPresentation(entryPhase);
  const village = entry.renderWorld && (snapshot.screen === "village" || snapshot.screen === "field");
  const roster = hunterRosterOpen;
  bottomMenu.hidden = !entry.enableGameUi || snapshot.screen === "boot";
  loginScreen.classList.toggle("leaving", !entry.showLogin);
  villageScreen.classList.toggle("visible", village || roster);
  villageScreen.classList.toggle("field-mode", snapshot.screen === "field");
  villageScreen.setAttribute("aria-hidden", String(!village && !roster));
  rosterScreen.classList.toggle("visible", roster);
  rosterScreen.setAttribute("aria-hidden", String(!roster));
  return village;
}

function syncEnhancementTaskView(snapshot: OriginalFlowSnapshot): void {
  if (enhancementHunterId === null) return;
  const hunter = snapshot.hunter_roster.active_hunters.find((row) => row.hunter_id === enhancementHunterId);
  const task = hunter?.gear_enhancement_task;
  if (!task) {
    enhancementHunterId = null;
    enhancementView = "select";
    selectedEnhancementGearKey = null;
    selectedEnhancementOptionalMaterialIds = [];
    if (selectedBuildingId === "build_15") buildingPanel.hidden = true;
    return;
  }
  enhancementView = task.status === "configuring" ? "configure"
    : task.status === "processing" ? "processing"
      : task.status === "result" ? "result" : "select";
  if (task.selected_gear_instance_id) selectedEnhancementGearKey = task.selected_gear_instance_id;
  if (task.mode) selectedEnhancementMode = task.mode;
  selectedEnhancementOptionalMaterialIds = task.optional_material_ids;
}

function renderWorldFrame(snapshot: OriginalFlowSnapshot): void {
  latestSnapshot = snapshot;
  world?.update(
    snapshot.world.entities,
    snapshot.world.visual_tick,
    snapshot.world.combat_presentations,
    snapshot.world.drops,
  );
  renderHunterEnhancementInteractions(snapshot);
}

function renderHunterEnhancementInteractions(snapshot: OriginalFlowSnapshot): void {
  hunterEnhancementInteractions.replaceChildren();
  if (snapshot.screen !== "village" || !world) return;
  const roster = projectHunterRoster(snapshot, null);
  const viewsById = new Map(roster.active.map((hunter) => [hunter.numericId, hunter]));
  for (const hunter of snapshot.hunter_roster.active_hunters) {
    const task = hunter.gear_enhancement_task;
    const hunterView = viewsById.get(hunter.hunter_id);
    if (!task || !hunterView) continue;
    const entityId = hunterWorldEntityId(snapshot, hunterView);
    if (!entityId) continue;
    const entity = snapshot.world.entities.find((candidate) => candidate.descriptor.entity_id === entityId);
    const state = projectHunterEnhancementInteraction({
      hunterEntityId: entityId,
      workflow: "gear_enhancement",
      phase: task.status === "traveling" || task.status === "waiting_for_interaction" ? task.status : null,
      buildingId: "build_15",
      buildingInstanceId: task.building_instance_id,
    });
    const point = world.screenPointForEntity(entityId);
    if (state.mode === "hidden" || !point) continue;
    if (state.mode === "traveling") {
      const indicator = document.createElement("span");
      indicator.className = "hunter-enhancement-travel-indicator";
      indicator.style.setProperty("--interaction-x", `${point.x}px`);
      indicator.style.setProperty("--interaction-y", `${point.y}px`);
      indicator.setAttribute("aria-label", t("enhancement.traveling_aria", { name: hunter.display_name }));
      indicator.textContent = t("world.hunter_initials");
      hunterEnhancementInteractions.append(indicator);
      continue;
    }
    if (!task.interaction_ready || entity?.interaction_prompt_key !== "hunter_enhancement_ready") continue;
    const button = document.createElement("button");
    button.type = "button";
    button.className = "hunter-enhancement-interaction";
    button.style.setProperty("--interaction-x", `${point.x}px`);
    button.style.setProperty("--interaction-y", `${point.y}px`);
    button.setAttribute("aria-label", t("enhancement.interact_aria", { name: hunter.display_name }));
    const icon = document.createElement("span");
    icon.textContent = t("world.hunter_initials");
    button.append(icon);
    button.addEventListener("click", () => {
      const intent = createOpenHunterEnhancementIntent(state);
      if (!intent) return;
      const instance = findBuildingInstanceById(snapshot.village.building_system.instances, intent.buildingInstanceId);
      if (!instance || instance.building_id !== "build_15") {
        showPanelMessage(t("enhancement.open_failed"), t("enhancement.forge_unavailable"));
        return;
      }
      enhancementHunterId = hunter.hunter_id;
      enhancementView = task.status === "result" ? "result" : "select";
      selectedEnhancementGearKey = null;
      selectedEnhancementMode = "single";
      selectedEnhancementOptionalMaterialIds = task.optional_material_ids;
      selectedBuildingId = "build_15";
      selectedBuildingInstanceId = instance.instance_id;
      selectedBuildingVisual = null;
      buildingPanelMode = "building";
      buildingPanel.hidden = false;
      renderBuildingSystem(latestSnapshot);
    });
    hunterEnhancementInteractions.append(button);
  }
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
  void hunterRosterActors.render(roster.active.slice(0, roster.capacity)).catch((error: unknown) => {
    console.error("Could not render current Hunter roster actors.", error);
    hunterRosterStatus.textContent = t("roster.actor_load_failed");
    hunterRosterStatus.classList.add("error");
  });
  hunterRosterStatus.textContent = roster.constraintViolation
    ?? (!roster.resolved ? t("roster.waiting_server") : "");
  hunterRosterStatus.classList.toggle("error", roster.constraintViolation !== null);
  if (hunterInfoModal.visible()) {
    const selected = [...roster.active, ...roster.waiting].find((hunter) => hunter.id === selectedHunterId);
    if (selected) hunterInfoModal.refresh(projectHunterInfo(rawHunterFor(snapshot, selected), selected));
  }
}

function hunterRosterCard(hunter: HunterView, snapshot: OriginalFlowSnapshot): HTMLElement {
  const rarity = hunterRarityPresentation(hunter.rarityId, hunter.rarityName);
  const card = document.createElement("article");
  card.className = `hunter-roster-card${rarity ? ` rarity-${rarity.key}` : " rarity-unresolved"}${hunter.id === selectedHunterId ? " selected" : ""}`;
  card.dataset.hunterId = hunter.id;
  card.setAttribute("aria-selected", String(hunter.id === selectedHunterId));
  const heading = document.createElement("header");
  const nameRow = document.createElement("span");
  nameRow.className = "hunter-card-name";
  const name = document.createElement("b");
  name.textContent = hunter.name;
  nameRow.append(name);
  if (rarity) {
    const badge = document.createElement("i");
    badge.textContent = rarity.letter;
    badge.title = hunter.rarityName ?? rarity.key;
    badge.setAttribute("aria-label", hunter.rarityName ?? rarity.key);
    nameRow.append(badge);
  }
  const levelClass = document.createElement("small");
  levelClass.className = "hunter-card-level-class";
  const level = document.createElement("span");
  level.textContent = hunter.level === null ? t("common.level_unavailable") : t("common.level_short", { level: hunter.level });
  const className = document.createElement("em");
  className.className = `class-${hunterClassTone(hunter.classFamily)}`;
  className.textContent = hunter.className ?? hunter.classFamily ?? t("roster.class_unavailable");
  levelClass.append(level, className);
  heading.append(nameRow, levelClass);
  const avatar = document.createElement("span");
  avatar.className = "hunter-avatar";
  if (hunter.portrait) {
    const image = document.createElement("img");
    image.src = hunter.portrait;
    image.alt = "";
    avatar.append(image);
  } else avatar.classList.add("composed");
  const meta = document.createElement("span");
  meta.className = "hunter-card-meta";
  const activity = document.createElement("small");
  activity.textContent = hunter.rosterState === "waiting"
    ? t("roster.waiting_position", { position: hunter.queuePosition ?? "-" })
    : hunter.hunt
      ? `${t(`hunter.activity.${hunter.hunt.status}` as MessageKey)}${hunter.hunt.zoneId ? ` · ${hunter.hunt.zoneId}` : ""}`
      : hunter.action ?? t("roster.activity_unavailable");
  if (hunter.action?.toLowerCase() === "dead") activity.className = "danger";
  else if (hunter.action?.toLowerCase() === "idle") activity.className = "positive";
  meta.append(activity);
  if (hunter.hunt && hunter.hunt.requiredTicks > 0) {
    const progress = document.createElement("i");
    progress.className = "hunter-card-hunt-progress";
    const fill = document.createElement("i");
    fill.style.width = `${hunterPercent(hunter.hunt.progressTicks, hunter.hunt.requiredTicks) ?? 0}%`;
    progress.append(fill);
    meta.append(progress);
  }
  const info = document.createElement("button");
  info.type = "button";
  info.className = "hunter-card-info";
  info.textContent = t("roster.info");
  info.setAttribute("aria-label", t("roster.info_aria", { name: hunter.name }));
  info.addEventListener("click", () => {
    info.blur();
    selectHunterCard(hunter.id);
    const raw = rawHunterFor(snapshot, hunter);
    hunterInfoModal.show(projectHunterInfo(raw, hunter));
  });
  const target = document.createElement("button");
  target.type = "button";
  target.className = "hunter-card-target";
  const worldEntityId = hunterWorldEntityId(snapshot, hunter);
  target.disabled = worldEntityId === null;
  target.title = worldEntityId ? t("roster.locate") : t("roster.not_visible");
  target.setAttribute("aria-label", worldEntityId ? t("roster.locate_aria", { name: hunter.name }) : t("roster.not_visible_aria", { name: hunter.name }));
  const targetIcon = document.createElement("img");
  targetIcon.src = originalAsset("sprites/ic_target__7095.png");
  targetIcon.alt = "";
  target.append(targetIcon);
  target.addEventListener("click", () => {
    if (!worldEntityId) return;
    selectHunterCard(hunter.id);
    setHunterRosterVisibility(false);
    hunterInfoModal.close();
    if (!client.selectEntity(worldEntityId)) return;
    if (!world?.focusEntity(worldEntityId)) return;
    // Locate uses the same post-click command bubble as selecting the actor in
    // the world; after camera focus the actor is centered in the viewport.
    hunterWorldCommandMenu.selectHunter({
      entityId: worldEntityId,
      displayName: hunter.name,
      screenPoint: { x: worldViewport.clientWidth / 2, y: worldViewport.clientHeight / 2 },
    });
  });
  const actions = document.createElement("footer");
  actions.append(info, target);
  card.append(heading, avatar, meta, actions);
  return card;
}

function selectHunterCard(id: string): void {
  selectedHunterId = id;
  document.querySelectorAll<HTMLElement>(".hunter-roster-card[data-hunter-id]").forEach((card) => {
    const selected = card.dataset.hunterId === id;
    card.classList.toggle("selected", selected);
    card.setAttribute("aria-selected", String(selected));
  });
}

function hunterForWorldEntity(snapshot: OriginalFlowSnapshot | null, entityId: string): HunterView | null {
  if (!snapshot) return null;
  const roster = projectHunterRoster(snapshot, selectedHunterId);
  return roster.active.find((hunter) => hunterWorldEntityId(snapshot, hunter) === entityId) ?? null;
}

function showWorldHunterInfo(entityId: string): void {
  const hunter = hunterForWorldEntity(latestSnapshot, entityId);
  if (!hunter || !latestSnapshot) {
    showPanelMessage(t("error.hunter_binding_unresolved"), entityId);
    return;
  }
  selectedHunterId = hunter.id;
  worldHunterInfoModal.show(projectHunterInfo(rawHunterFor(latestSnapshot, hunter), hunter));
}

function useHunterSkillFromInfo(hunterId: number, skillId: string): void {
  if (client.useHunterSkill(hunterId, skillId, null)) {
    showPanelMessage(t("feedback.skill_sent"), t("feedback.skill_checking"));
  } else {
    showPanelMessage(t("feedback.skill_failed"), t("feedback.server_not_ready"));
  }
}

function handleHunterWorldCommandIntent(intent: HunterWorldCommandIntent): void {
  worldViewport.dispatchEvent(new CustomEvent<HunterWorldCommandIntent>("hunter-world-command-intent", {
    detail: intent,
    bubbles: true,
  }));
  const hunter = hunterForWorldEntity(latestSnapshot, intent.hunterEntityId);
  if (hunter?.numericId === null || hunter?.numericId === undefined) {
    showPanelMessage(t("feedback.command_failed"), t("feedback.hunter_id_unbound"));
    return;
  }
  if (intent.type === "sell_hunter_loot") {
    if (client.sellHunterLoot(hunter.numericId)) {
      showPanelMessage(t("feedback.sale_sent"), t("feedback.sale_checking"));
    } else {
      showPanelMessage(t("feedback.sale_failed"), t("feedback.server_not_ready"));
    }
    return;
  }
  const region = { map_new01: t("hunter.command.colony"), background_08: t("hunter.command.dead_land"), background_11: t("hunter.command.demon_world") }[intent.regionId];
  if (client.assignHunterHunt(hunter.numericId, intent.regionId)) {
    showPanelMessage(t("feedback.hunt_sent"), t("feedback.awaiting_server", { region }));
  } else {
    showPanelMessage(t("feedback.command_failed"), t("feedback.server_not_ready"));
  }
}

function handleHunterEnhancementRequest(intent: HunterGearEnhancementRequestIntent): void {
  const hunter = hunterForWorldEntity(latestSnapshot, intent.hunterEntityId);
  if (hunter?.numericId === null || hunter?.numericId === undefined) {
    showPanelMessage(t("feedback.command_failed"), t("feedback.hunter_id_unbound"));
    return;
  }
  if (!client.startHunterEnhancement(hunter.numericId)) {
    showPanelMessage(t("feedback.command_failed"), t("feedback.server_not_ready"));
  }
}

function emptyHunterSlot(slot: number): HTMLElement {
  const card = document.createElement("article");
  card.className = "hunter-roster-card empty";
  card.innerHTML = `<span class="hunter-avatar">+</span><b>${t("roster.empty_slot", { slot })}</b>`;
  return card;
}



function rawHunterFor(snapshot: OriginalFlowSnapshot, hunter: HunterView): unknown {
  const roster = snapshot.hunter_roster as unknown as { active_hunters?: unknown[]; waiting_hunters?: unknown[] };
  const rows = [...(roster.active_hunters ?? []), ...(roster.waiting_hunters ?? [])];
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
    snapshot.hunter_roster.active_hunters.map((hunter) => ({ id: hunter.hunter_id, task: hunter.gear_enhancement_task })),
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
      button.textContent = `${evidence.name} ${state?.constructed ? t("common.level_short", { level: state.level }) : `Lv.1-${evidence.maxLevel ?? "?"}`}`;
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
  const isEnhancementForgeRoute = buildingPanelMode === "building" && ENHANCEMENT_FORGE_BUILDING_IDS.includes(selectedBuildingId as typeof ENHANCEMENT_FORGE_BUILDING_IDS[number]);
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
  buildingPanel.classList.toggle("enhancement-forge-ui", isEnhancementForgeRoute);
  buildingPanel.classList.toggle("jeweler-ui", isJewelerRoute);
  buildingPanel.classList.toggle("display-shop-ui", isDisplayShopRoute);
  buildingPanel.classList.toggle("potion-shop-ui", isPotionBuilding(selectedBuildingId));
  buildingPanel.classList.toggle("potion-crafting-ui", isPotionCraftingRoute);
  if (!evidence) {
    buildingName.textContent = selectedBuildingId ?? t("building.evidence_unavailable");
    buildingLevel.textContent = buildingEvidenceError ?? t("building.loading_evidence");
    buildingFeature.textContent = t("building.no_fabricated_data");
    buildingCondition.textContent = t("building.actions_disabled");
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
  if (buildingPanelMode === "building") buildingName.textContent = `${t("common.level_short", { level: currentLevel })} ${evidence.name}`;
  const targetLevel = Math.min(currentLevel + 1, evidence.maxLevel ?? currentLevel + 1);
  buildingLevel.textContent = state?.constructed
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
  buildingFeature.textContent = buildingPanelMode === "building" ? featureDescription
    : rawSourceSummary ? `${featureDescription} · ${rawSourceSummary}` : featureDescription;
  const targetRequirement = evidence.levels.find((entry) => entry.level === targetLevel)?.requiredTownHallLevel ?? null;
  const localizedRequirement = targetRequirement === null
    ? t("building.town_hall_unresolved")
    : originalUiLabel("buildpop_9", "vi", [targetRequirement]);
  buildingCondition.textContent = selectedInstance?.condition?.startsWith("building_prerequisite_required:")
    ? localizedRequirement
    : selectedInstance?.condition ?? localizedRequirement;
  buildingLevelContract.replaceChildren(...evidence.levels.map((entry) => {
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
  if (buildingPanelMode === "building" && evidence.popupRoute === "gear-enhancement") {
    buildingLevelContract.hidden = true;
    buildingCatalog.hidden = false;
    renderEnhancementForge(system);
  } else if (buildingPanelMode === "building" && evidence.id === TRADING_POST_ROUTE.buildingId) {
    buildingLevelContract.hidden = true;
    buildingCatalog.hidden = false;
    const currentRequest = selectedTradingRequest
      ? system.material_stocks.find((stock) => stock.id === selectedTradingRequest?.id) ?? null
      : null;
    if (selectedTradingRequest && !currentRequest) {
      selectedTradingRequest = null;
      tradingRequestPending = false;
    } else if (currentRequest) {
      selectedTradingRequest = currentRequest;
    }
    if (selectedTradingRequest) renderTradingRequestEditor();
    else renderTradingPostCatalog(system.material_stocks, selectedInstance?.level ?? 1, targetRequirement);
  } else if (buildingPanelMode === "building" && evidence.popupRoute === "production" && isCatalogShopRoute) {
    buildingLevelContract.hidden = true;
    buildingCatalog.hidden = false;
    if (isCraftingGearRoute) {
      renderGearCraftingCatalog(system.recipes, evidence.id);
    } else if (isPotionCraftingRoute) {
      renderPotionCraftingCatalog(system.recipes, currentLevel);
    } else {
      renderDisplayShopCatalog(system.recipes, selectedBuildingId);
    }
  } else if (buildingPanelMode === "building" && evidence.popupRoute === "production") {
    renderBuildingContractError(t("building.production_contract_missing"));
    return;
  } else if (buildingPanelMode === "building" && evidence.popupRoute === "service" && productServiceRoute(selectedBuildingId ?? "") !== null) {
    if (!selectedBuildingId) return;
    buildingLevelContract.hidden = true;
    buildingCatalog.hidden = false;
    const currentBuildingLevel = selectedInstance?.level ?? 1;
    const route = productServiceRoute(selectedBuildingId);
    if (!route) {
      renderBuildingContractError(t("building.service_contract_missing"));
      return;
    }
    const routeCandidates = system.recipes.filter((item) => item.shop_id === selectedBuildingId);
    // Project through the decoded route contract before rendering. This keeps
    // service popups isolated to their seven recovered product IDs.
    const routeProjection = projectProductService(selectedBuildingId, routeCandidates.map((recipe): ProductServiceInput => ({
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
    const activeServiceTab = serviceTabsByBuilding.get(route.buildingId) ?? "production";
    const authoritativeService = snapshot.hunter_roster.product_services.find((service) => service.building_id === route.buildingId) ?? null;
    for (const tab of ["production", "hunters"] as const) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = tab === activeServiceTab ? "selected" : "";
      button.textContent = tab === "production" ? t("service.production") : t("service.hunters");
      button.addEventListener("click", () => {
        serviceTabsByBuilding.set(route.buildingId, tab);
        renderBuildingSystem(latestSnapshot);
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
        selectedRecipe = recipe;
        selectedServiceMaterialId = null;
        selectedServiceQuantity = 1;
        consumCreatePop.hidden = false;
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
  } else if (buildingPanelMode === "building" && evidence.popupRoute === "service") {
    renderBuildingContractError(t("building.service_contract_missing"));
    return;
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
  buildingUpgrade.hidden = buildingPanelMode !== "building" || isEnhancementForgeRoute;
  const isBounty = evidence.id === BOUNTY_HUT_ROUTE.buildingId;
  buildingUse.hidden = buildingPanelMode !== "building"
    || evidence.id === TRADING_POST_ROUTE.buildingId
    || (evidence.popupRoute !== "request" && !isBounty && !isEnhancementForgeRoute);
  buildingConstruct.disabled = !spriteResolved || state?.constructed !== false || state.can_construct !== true;
  buildingConstruct.title = !spriteResolved ? t("building.sprite_unresolved") : state?.condition ?? "";
  buildingUpgrade.disabled = !spriteResolved || !selectedInstance || selectedInstance.can_upgrade !== true;
  buildingUpgrade.title = selectedInstance?.condition ?? "";
  buildingUpgrade.textContent = productServiceRoute(evidence.id) || isCatalogShopRoute
    ? originalUiLabel("buildpop_7")
    : `${originalUiLabel("buildpop_7")} · ${formatLevelCosts(evidence, targetLevel)}`;
  const selectedEnhancement = isEnhancementForgeRoute
    ? latestSnapshot?.hunter_roster.active_hunters
      .filter((hunter) => enhancementHunterId === null || hunter.hunter_id === enhancementHunterId)
      .flatMap((hunter) => hunter.gear_enhancements.map((gear) => ({ hunter, gear, key: gear.instance_id ?? `${hunter.hunter_id}:${gear.product_id}` })))
      .find((row) => row.key === selectedEnhancementGearKey) ?? null
    : null;
  const enhancementCanSubmit = selectedEnhancement !== null
    && canSubmitGearEnhancement(projectGearEnhancement(selectedEnhancement.gear, selectedEnhancementMode));
  buildingUse.disabled = isEnhancementForgeRoute
    ? !selectedInstance || selectedEnhancement === null || (enhancementView === "configure" && !enhancementCanSubmit)
    : !selectedInstance || (evidence.popupRoute !== "request" && evidence.popupRoute !== "production" && !isBounty);
  buildingUse.title = isEnhancementForgeRoute
    ? enhancementView === "select"
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
  buildingUse.textContent = isEnhancementForgeRoute ? (enhancementView === "select" ? t("enhancement.continue") : t("enhancement.action")) : isBounty ? t("service.bounties") : evidence.popupRoute === "request" ? t("service.requests")
    : evidence.popupRoute === "service" ? t(serviceLabels[evidence.id] ?? "service.use")
    : evidence.popupRoute === "production" ? t("common.create") : t("common.open");
}

function renderBuildingContractError(message: string): void {
  const error = document.createElement("p");
  error.className = "building-contract-error";
  error.textContent = message;
  buildingLevelContract.hidden = true;
  buildingCatalog.hidden = false;
  buildingCatalog.replaceChildren(error);
  buildingUse.hidden = true;
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
  const qualityLabels = [t("craft.quality.regular"), t("craft.quality.sturdy"), t("craft.quality.refined"), t("craft.quality.powerful"), t("craft.quality.supreme")] as const;
  const tabsForBuilding: readonly GearKind[] = producerBuildingId === JEWELER_BUILDING_IDS[0] ? JEWELER_GEAR_TABS : BLACKSMITH_GEAR_TABS;
  if (!tabsForBuilding.includes(gearTab)) gearTab = tabsForBuilding[0];
  const all = fullGearRecipes(recipes, producerBuildingId).filter((recipe) => gearKindFromRecipe(recipe) === gearTab);
  const catalogById = new Map(gearCatalog.map((entry) => [entry.id, entry]));
  const buildingLevel = findBuildingInstanceById(
    latestSnapshot?.village.building_system.instances ?? [],
    selectedBuildingInstanceId,
  )?.level ?? 1;
  const difficultyOptions = producerBuildingId === JEWELER_BUILDING_IDS[0]
    ? [t("difficulty.junk"), t("difficulty.easy"), t("difficulty.normal"), t("difficulty.hard"), t("difficulty.expert"), t("difficulty.nightmare"), t("difficulty.torment")]
    : [t("difficulty.easy"), t("difficulty.normal"), t("difficulty.hard"), t("difficulty.expert"), t("difficulty.nightmare"), t("difficulty.torment")];
  const maxDifficultyGroup = Math.min(6, producerBuildingId === JEWELER_BUILDING_IDS[0]
    ? Math.max(0, buildingLevel - 1)
    : Math.max(1, buildingLevel));
  if (blacksmithDifficultyGroup > maxDifficultyGroup) blacksmithDifficultyGroup = maxDifficultyGroup;
  const matching = all.filter((recipe) => {
    const staticRow = catalogById.get(recipe.id);
    return (staticRow?.difficultyGroup === undefined || staticRow.difficultyGroup < 0 || staticRow.difficultyGroup === blacksmithDifficultyGroup)
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
    button.setAttribute("aria-label", t(`craft.kind.${tab}` as MessageKey));
    button.dataset.gearTab = tab;
    button.className = tab === gearTab ? "selected" : "";
    button.addEventListener("click", () => { gearTab = tab; renderBuildingSystem(latestSnapshot); });
    tabs.append(button);
  }
  const filters = document.createElement("div");
  filters.className = "blacksmith-filters";
  const difficultyEntries = difficultyOptions.slice(0, maxDifficultyGroup + (producerBuildingId === JEWELER_BUILDING_IDS[0] ? 1 : 0)).map((label, index) => {
    const group = producerBuildingId === JEWELER_BUILDING_IDS[0] ? index : index + 1;
    return { value: String(group), label };
  });
  const difficulty = createGameDropdown(t("craft.gear_difficulty"), String(blacksmithDifficultyGroup), difficultyEntries, (value) => {
    blacksmithDifficultyGroup = Number(value);
    renderBuildingSystem(latestSnapshot);
  });
  filters.append(difficulty);
  const craftable = document.createElement("label");
  craftable.className = "blacksmith-craftable";
  const checkbox = document.createElement("input"); checkbox.type = "checkbox"; checkbox.checked = blacksmithCraftableOnly;
  checkbox.addEventListener("change", () => { blacksmithCraftableOnly = checkbox.checked; renderBuildingSystem(latestSnapshot); });
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
  const nextDifficulty = difficultyOptions[(producerBuildingId === JEWELER_BUILDING_IDS[0] ? buildingLevel : buildingLevel)];
  hint.textContent = nextDifficulty
    ? t("craft.gear_upgrade_hint", { level: buildingLevel + 1, difficulty: nextDifficulty, kind: producerBuildingId === JEWELER_BUILDING_IDS[0] ? t("craft.kind.accessories") : t("craft.kind.weapons_armor") })
    : t("craft.all_gear_difficulties");
  buildingCatalog.replaceChildren(controls, grid, footer, hint);
}

function renderEnhancementForge(_system: BuildingSystemSnapshot): void {
  const ownedRows = latestSnapshot?.hunter_roster.active_hunters
    .filter((hunter) => enhancementHunterId === null || hunter.hunter_id === enhancementHunterId)
    .flatMap((hunter) => (
    hunter.gear_enhancements.map((gear) => ({ hunter, gear, key: gear.instance_id ?? `${hunter.hunter_id}:${gear.product_id}` }))
  )) ?? [];
  if (!ownedRows.some((row) => row.key === selectedEnhancementGearKey)) selectedEnhancementGearKey = null;
  const selected = ownedRows.find((row) => row.key === selectedEnhancementGearKey) ?? null;
  const task = enhancementHunterId === null
    ? null
    : latestSnapshot?.hunter_roster.active_hunters.find((hunter) => hunter.hunter_id === enhancementHunterId)?.gear_enhancement_task ?? null;
  if (!selected) enhancementView = "select";
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
  stateBanner.textContent = enhancementView === "configure" && selected ? t("enhancement.configure") : t("enhancement.select_prompt");

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
    checkbox.checked = selectedEnhancementOptionalMaterialIds.includes(materialId);
    checkbox.disabled = !task || task.blockers.length > 0;
    checkbox.addEventListener("change", () => {
      selectedEnhancementOptionalMaterialIds = optionalMaterials
        .filter(([, id]) => id === materialId ? checkbox.checked : selectedEnhancementOptionalMaterialIds.includes(id))
        .map(([, id]) => id);
      renderBuildingSystem(latestSnapshot);
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
    radio.checked = selectedEnhancementMode === mode;
    radio.addEventListener("change", () => {
      selectedEnhancementMode = mode;
      renderBuildingSystem(latestSnapshot);
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
    row.classList.toggle("selected", owned.key === selectedEnhancementGearKey);
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
      selectedEnhancementGearKey = owned.key;
      renderBuildingSystem(latestSnapshot);
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
  processing.hidden = enhancementView !== "processing";
  processing.innerHTML = `<div class="enhancement-processing-art"></div><strong>${t("enhancement.processing")}</strong>`;
  const result = document.createElement("section");
  result.className = "enhancement-result";
  result.hidden = enhancementView !== "result";
  const finalLevel = task?.final_level === null || task?.final_level === undefined ? null : `+${task.final_level}`;
  const spent = task ? t("enhancement.spent_gold", { amount: formatNumber(task.spent_gold) }) : t("enhancement.waiting_snapshot");
  result.innerHTML = `<strong>${t("enhancement.result")}</strong><p>${finalLevel ? t("enhancement.final_level", { level: finalLevel }) : t("enhancement.final_level_unresolved")}<br />${spent}<br />${task?.stop_reason ?? ""}</p>`;

  shell.append(workspace, stage, stateBanner);
  if (enhancementView === "configure" && selected) shell.append(configureControls);
  shell.append(wallet, inventory, capNote, evidence, processing, result);
  buildingCatalog.replaceChildren(shell);
}

function enhancementGearPresentation(productId: string, system: BuildingSystemSnapshot): { name: string; icon: string | null } {
  const live = system.recipes.find((recipe) => recipe.id === productId);
  const catalog = gearCatalog.find((recipe) => recipe.id === productId);
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
  const system = latestSnapshot?.village.building_system;
  const level = findBuildingInstanceById(system?.instances ?? [], selectedBuildingInstanceId)?.level ?? 1;
  const allowed = recipes.filter((recipe) => recipe.shop_id === buildingId && recipe.required_level < level);
  const heading = document.createElement("h3");
  const isPotionShop = buildingId === POTION_SHOP_BUILDING_ID;
  heading.textContent = isPotionShop ? t("shop.potion_display") : t("shop.display_list");
  const grid = document.createElement("div");
  grid.className = isPotionShop ? "display-shop-grid potion-recipe-grid" : "display-shop-grid";
  grid.replaceChildren(...allowed.map((recipe) => {
    if (isPotionShop) {
      const card = document.createElement("article");
      card.className = "gear-catalog-card display-card potion-catalog-card potion-display-card";
      const badge = document.createElement("span");
      badge.className = "potion-stock-badge";
      badge.textContent = `${t("common.stock")}\n${recipe.stock}`;
      appendPotionArt(card, recipe);
      const name = document.createElement("strong");
      name.textContent = recipe.product_name;
      const price = document.createElement("small");
      const gold = document.createElement("img");
      gold.src = originalAsset("sprites/top_ic_01_gold_24__4677.png");
      gold.alt = "";
      price.append(gold, document.createTextNode(formatNumber(recipe.sale_price)));
      card.append(badge, name, price);
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
    empty.textContent = t("shop.no_potion_recipes");
    grid.append(empty);
  }
  const hint = document.createElement("div");
  hint.className = "blacksmith-upgrade-hint";
  hint.textContent = t("shop.potion_craft_hint");
  buildingCatalog.replaceChildren(heading, grid, hint);
}

function renderTradingPostCatalog(
  stocks: readonly MaterialStockSnapshot[],
  buildingLevel: number,
  nextTownHallRequirement: number | null,
): void {
  selectedTradingPostDifficulty = Math.min(selectedTradingPostDifficulty, Math.max(0, buildingLevel - 1));
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
      if (!selectedBuildingInstanceId) return;
      if (stock.requested > 0) client.cancelMaterialRequest(selectedBuildingInstanceId, stock.id);
      else {
        selectedTradingRequest = stock;
        selectedTradingRequestQuantity = 1;
        tradingRequestPending = false;
        renderTradingRequestEditor();
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
  buildingCatalog.replaceChildren(toolbar, grid, hint);
}

function renderBountyPop(): void {
  const system = latestSnapshot?.village.building_system;
  if (!system || selectedBuildingId !== BOUNTY_HUT_ROUTE.buildingId) return;
  const instance = findBuildingInstanceById(system.instances, selectedBuildingInstanceId);
  bountyTitle.textContent = `${t("common.level_short", { level: instance?.level ?? 1 })} ${BOUNTY_HUT_ROUTE.title}`;
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
  bountyUpgrade.textContent = `${t("common.upgrade")} · ${evidence ? formatLevelCosts(evidence, (instance?.level ?? 1) + 1) : t("bounty.cost_unresolved")}`;
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
  const system = latestSnapshot?.village.building_system;
  if (!system || !selectedRecipe) return;
  const quantity = clampQuantity(gearCreateQuantity.value, 1, 1000);
  gearCreateQuantity.value = String(quantity);
  const gearKind = gearKindFromRecipe(selectedRecipe) ?? "weapon";
  const gearKindLabel = t(`craft.kind.${gearKind}` as MessageKey);
  const stockFamily = (recipe: ShopRecipeSnapshot): string => {
    const kind = gearKindFromRecipe(recipe);
    if (kind === "weapon") return "weapon";
    if (kind && JEWELER_GEAR_TABS.some((candidate) => candidate === kind)) return "accessory";
    return "armor";
  };
  const remainingCapacity = remainingSharedCapacity(
    system.recipes.filter((recipe) => recipe.shop_id === selectedRecipe?.shop_id),
    selectedRecipe,
    (candidate, current) => stockFamily(candidate) === stockFamily(current),
  );
  gearCreatePop.classList.toggle("gear-detail-mode", gearPopupMode === "detail");
  gearCreateTitle.textContent = gearPopupMode === "detail" ? selectedRecipe.product_name : t("craft.create_item", { item: gearKindLabel });
  if (selectedRecipe.icon) gearCreateIcon.src = selectedRecipe.icon;
  else gearCreateIcon.removeAttribute("src");
  gearCreateIcon.hidden = !selectedRecipe.icon;
  gearCreateName.textContent = selectedRecipe.product_name;
  gearCreatePrice.textContent = gearPopupMode === "detail"
    ? t("craft.gear_display_price", { kind: gearKindLabel, price: formatNumber(selectedRecipe.sale_price) })
    : `${gearKindLabel} · ${t("craft.shop_stock", { stock: selectedRecipe.stock, capacity: selectedRecipe.capacity })}`;
  gearCreateDescription.replaceChildren();
  if (gearPopupMode === "detail") {
    const unresolved = document.createElement("p");
    unresolved.textContent = t("craft.gear_options_unavailable");
    gearCreateDescription.append(unresolved, gearLock);
  }
  gearLock.hidden = gearPopupMode !== "detail";
  gearLock.disabled = true;
  gearMaterialTitle.hidden = gearPopupMode === "detail";
  gearMaterialTitle.textContent = t("craft.required_materials");
  gearMaterialCosts.hidden = gearPopupMode === "detail";
  gearQuantityRow.hidden = gearPopupMode === "detail";
  gearStorageLabel.hidden = gearPopupMode === "detail";
  gearCreateSubmit.hidden = gearPopupMode === "detail";
  gearCreateSubmit.textContent = t("common.produce");
  gearCreateSell.hidden = gearPopupMode !== "detail";
  gearCreateSell.disabled = true;
  let craftable = true;
  gearMaterialCosts.replaceChildren(...selectedRecipe.material_costs.map((cost) => {
    const stock = system.material_stocks.find((item) => item.id === cost.material_id);
    const row = document.createElement("div");
    const selected = true;
    const iconPath = gearMaterialIcons.get(cost.material_id) ?? stock?.icon ?? resourceIconPath(cost.material_id);
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
  gearFrameQuantity.value = String(quantity);
  gearStorageLabel.textContent = selectedRecipe.capacity > 0
    ? t("craft.remaining_storage", { amount: remainingCapacity })
    : t("craft.remaining_storage_unlimited");
  gearCreateSubmit.disabled = gearPopupMode !== "craft" || !craftable || quantity > remainingCapacity;
  gearCreateSubmit.title = !craftable
    ? t("craft.materials_missing_tooltip")
    : quantity > remainingCapacity
      ? t("craft.capacity_tooltip")
      : "";
}

function renderConsumCreatePop(): void {
  const system = latestSnapshot?.village.building_system;
  if (!system || !selectedRecipe) return;
  const isServiceProduct = selectedRecipe.kind === "service"
    || productServiceRoute(selectedRecipe.shop_id) !== null;
  const isPotionRecipe = selectedRecipe.shop_id === ALCHEMIST_BUILDING_ID;
  if (!isServiceProduct && !isPotionRecipe) return;
  // ConsumCreatePop and ProductCreatePop have different recovered layout contracts.
  consumCreatePop.classList.toggle("service-product-ui", isServiceProduct);
  consumCreatePop.classList.toggle("potion-product-ui", isPotionRecipe);
  if (isServiceProduct) {
    selectedServiceMaterialId = resolveServiceMaterialId(
      selectedRecipe.material_costs,
      system.material_stocks,
      selectedServiceQuantity,
      selectedServiceMaterialId,
    );
  }
  const selectedCost = isServiceProduct
    ? selectedRecipe.material_costs.find((cost) => cost.material_id === selectedServiceMaterialId)
    : null;
  const outputPerBatch = Math.max(1, selectedCost?.output_quantity ?? 1);
  const inputPerBatch = Math.max(1, selectedCost?.quantity ?? 1);
  const availableInput = selectedServiceMaterialId
    ? townMaterialQuantity(system.material_stocks, selectedServiceMaterialId)
    : 0;
  const possibleOutput = isServiceProduct
    ? Math.floor(availableInput / inputPerBatch) * outputPerBatch
    : selectedRecipe.material_costs.reduce((maximum, cost) => {
      const available = system.material_stocks.find((stock) => stock.id === cost.material_id)?.town_quantity ?? 0;
      return Math.min(maximum, Math.floor(available / Math.max(1, cost.quantity)));
    }, Number.MAX_SAFE_INTEGER);
  const remainingCapacity = remainingSharedCapacity(
    system.recipes,
    selectedRecipe,
    (candidate, current) => candidate.shop_id === current.shop_id,
  );
  const serviceCapacity = isServiceProduct ? Number.MAX_SAFE_INTEGER : remainingCapacity;

  consumCreateTitle.textContent = t("craft.produce_item", { item: selectedRecipe.product_name });
  if (selectedRecipe.icon) consumCreateIcon.src = selectedRecipe.icon;
  else consumCreateIcon.removeAttribute("src");
  consumCreateIcon.hidden = !selectedRecipe.icon;
  consumCreateIconPlaceholder.hidden = Boolean(selectedRecipe.icon);
  consumCreateQuantity.value = String(selectedServiceQuantity);
  consumCreateQuantityInput.value = String(selectedServiceQuantity);
  consumMaterialTitle.textContent = isPotionRecipe ? t("craft.required_materials") : t("craft.select_material");
  consumConversion.textContent = isPotionRecipe
    ? `${t("craft.stock", { stock: selectedRecipe.stock, capacity: selectedRecipe.capacity > 0 ? selectedRecipe.capacity : "∞" })}\n${t("craft.produce_progress", { current: selectedServiceQuantity, maximum: Math.min(possibleOutput, serviceCapacity) })}`
    : selectedCost
    ? `${t("craft.conversion", { output: outputPerBatch, product: selectedRecipe.product_name, input: inputPerBatch, material: selectedCost.display_name })}\n${t("craft.produce_progress", { current: selectedServiceQuantity, maximum: possibleOutput })}`
    : t("craft.conversion_unresolved");
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
  const neededInput = selectedCost ? serviceMaterialRequired(selectedCost, selectedServiceQuantity) : 0;
  const missingMaterialId = isServiceProduct
    ? selectedCost && availableInput < neededInput ? selectedCost.material_id : null
    : missingCraftMaterial(selectedRecipe.material_costs, system.material_stocks, selectedServiceQuantity);
  const capacityExceeded = !isServiceProduct && selectedServiceQuantity > remainingCapacity;
  consumCreateSubmit.disabled = (!selectedCost && isServiceProduct) || missingMaterialId !== null || capacityExceeded;
  consumCreateSubmit.title = missingMaterialId
    ? t("craft.selected_material_missing_tooltip")
    : capacityExceeded
      ? t("craft.capacity_tooltip")
      : "";
  if (consumCreateSubmit.disabled) {
    consumConversion.textContent += missingMaterialId
      ? `\n${t("craft.missing_material")}`
      : `\n${t("craft.destination_full")}`;
  }
}

function renderCombatHud(state: CombatHudState): void {
  latestCombatHud = state;
  combatHud.hidden = !debugUi || !state.visible;
  if (!state.visible) return;
  element<HTMLElement>("#combat-evidence").textContent = `${state.evidenceLabel} · ${t("combat.tick", { tick: state.tick })} · ${state.fighting ? t("combat.fighting") : t("combat.idle")}`;
  element<HTMLElement>("#hunter-state").textContent = state.hunter.state;
  element<HTMLElement>("#hunter-position").textContent = t("combat.position", { position: state.hunter.position });
  element<HTMLElement>("#hunter-hp").textContent = t("combat.hp", { current: state.hunter.hp, maximum: state.hunter.maxHp });
  element<HTMLElement>("#hunter-hp-fill").style.width = `${state.hunter.percent}%`;
  element<HTMLElement>("#monster-state").textContent = state.monster.state;
  element<HTMLElement>("#monster-position").textContent = t("combat.position", { position: state.monster.position });
  element<HTMLElement>("#monster-hp").textContent = t("combat.hp", { current: state.monster.hp, maximum: state.monster.maxHp });
  element<HTMLElement>("#monster-hp-fill").style.width = `${state.monster.percent}%`;
  element<HTMLElement>("#combat-gold").textContent = t("combat.gold", { amount: state.gold });
  element<HTMLElement>("#combat-inventory").textContent = state.equipped
    ? [state.inventory, t("combat.item_equipped", { id: 2001 })].filter(Boolean).join(" · ")
    : state.inventory;
  element<HTMLElement>("#combat-drops").textContent = state.drops;
  equipFixtureItem.disabled = !state.equipEligible;
  equipFixtureItem.textContent = state.equipped ? t("combat.item_equipped", { id: 2001 }) : t("combat.equip_item", { id: 2001 });
}

function showIntentResult(result: IntentFeedback): void {
  if (result.intent === "set_material_request") {
    tradingRequestPending = false;
    if (result.accepted) {
      selectedTradingRequest = null;
      renderBuildingSystem(latestSnapshot);
    } else if (selectedTradingRequest) {
      renderTradingRequestEditor();
    }
  }
  if (!result.accepted) {
    const reasons: Record<string, MessageKey> = {
      insufficient_materials: "error.insufficient_materials",
      material_stock_missing: "error.material_stock_missing",
      recipe_unknown: "error.recipe_unknown",
      recipe_building_mismatch: "error.recipe_building_mismatch",
      product_level_locked: "error.product_level_locked",
      sale_building_instance_unknown: "error.sale_building_missing",
      product_capacity_exceeded: "error.product_capacity",
      product_stock_empty: "error.product_empty",
      sale_price_unresolved: "error.sale_price_unresolved",
      building_instance_unknown: "error.building_missing",
      building_capability_mismatch: "error.capability_mismatch",
      material_difficulty_unresolved: "error.material_difficulty_unresolved",
      material_difficulty_locked: "error.material_difficulty_locked",
      material_quantity_invalid: "error.material_quantity_invalid",
      material_price_unresolved: "error.material_price_unresolved",
    };
    const titles: Record<string, MessageKey> = {
      select_bottom_menu: "error.cannot_open_menu",
      navigate_back: "error.cannot_navigate_back",
      enter_field: "error.cannot_enter_field",
      select_entity: "error.cannot_select_entity",
      set_material_request: "error.cannot_request",
    };
    const reasonKey = reasons[result.reason ?? ""];
    const detail = reasonKey ? t(reasonKey) : debugUi && result.reason ? `${t("error.try_again")} (${result.reason})` : t("error.try_again");
    showPanelMessage(t(titles[result.intent] ?? "error.cannot_craft"), detail);
  }
}
function showBindingBlocked(result: BindingBlockedFeedback): void {
  showPanelMessage(t("error.coming_soon"), debugUi ? `${result.intent.replaceAll("_", " ")} · ${result.blockers.join(", ")}` : t("error.feature_rebuilding"));
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
  const labels: Record<ConnectionStatus, MessageKey> = { connecting: "connection.connecting", online: "connection.online", reconnecting: "connection.reconnecting", offline: "connection.offline" };
  connectionStatus.className = `connection-status ${status}`;
  connectionStatus.querySelector("span")!.textContent = t(labels[status]);
  updateBootState();
}
function updateBootState(): void {
  enterVillage.disabled = entryPhase !== "login" || mapLoadFailed;
  const entry = projectEntryPresentation(entryPhase);
  gameShell!.classList.toggle("entry-login", entryPhase === "login");
  gameShell!.classList.toggle("entry-loading", entryPhase === "loading");
  gameShell!.classList.toggle("entry-game", entryPhase === "game");
  loginScreen.classList.toggle("leaving", !entry.showLogin);
  gameLoadingScreen.hidden = !entry.showLoading;
  if (!bootRequested) {
    transition.hidden = true;
    bootStatus.textContent = mapLoadFailed ? t("boot.map_unavailable") : !mapReady ? t("boot.preparing_assets") : connectionState === "online" ? t("boot.ready_sign_in") : t("boot.connecting_server");
    return;
  }
  const dispatching = connectionState === "online";
  transition.hidden = !dispatching;
  bootStatus.textContent = dispatching ? t("boot.entering_village") : t("boot.waiting_server");
}
window.addEventListener("beforeunload", () => {
  hunterWorldCommandMenu.destroy();
  client.disconnect();
}, { once: true });
