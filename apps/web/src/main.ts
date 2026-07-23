import { Application } from "pixi.js";
import { WorldClient, type BindingBlockedFeedback, type ConnectionStatus, type IntentFeedback } from "./net/world-client";
import type { BottomMenuIntent, OriginalFlowSnapshot } from "./generated/protocol";
import { VisibleEntityWorld } from "./game/visible-world";
import "./styles.css";

const mount = document.querySelector<HTMLDivElement>("#app");
if (!mount) throw new Error("Missing #app mount point");

const releaseRoot = "/content/releases/original-flow-v1";
const originalAsset = (sourcePath: string): string => `${releaseRoot}/${sourcePath}`;
const menuItems: Array<{ id: BottomMenuIntent; label: string; icon: string }> = [
  { id: "character", label: "Character", icon: "sprites/menu_ic_01__6756.png" },
  { id: "build", label: "Build", icon: "sprites/menu_ic_02__2060.png" },
  { id: "archive", label: "Archive", icon: "sprites/menu_ic_03__6410.png" },
  { id: "store", label: "Store", icon: "sprites/menu_ic_04__5070.png" },
  { id: "raid", label: "Raid", icon: "sprites/menu_ic_05__6398.png" },
];

mount.innerHTML = `
  <main class="game-shell">
    <section id="boot-screen" class="boot-screen" aria-label="Game intro">
      <img class="boot-background" src="${originalAsset("sprites/intro_bg_new__1695.png")}" alt="" />
      <div class="boot-vignette"></div>
      <img class="boot-logo" src="${originalAsset("sprites/intro_img_glo_new__2141.png")}" alt="Evil Hunter Tycoon" />
      <button id="enter-village" class="enter-village" type="button"><img src="${originalAsset("sprites/intro_glo_touchtostart__7172.png")}" alt="Touch to start" /></button>
    </section>
    <section id="village-screen" class="village-screen" aria-label="Village" aria-hidden="true">
      <div id="world-viewport" class="village-world" aria-label="Authoritative entity world"></div>
      <header class="resource-bar" aria-label="Village resources"><div class="town-mark"><span></span><b id="world-mode-label">Village</b></div><div class="resource-cluster"><div class="resource-pill unresolved"><img src="${originalAsset("sprites/top_ic_01_gold_24__4677.png")}" alt="Gold" /><b>--</b></div><div class="resource-pill unresolved"><img src="${originalAsset("sprites/top_ic_02_gem_24__4214.png")}" alt="Gem" /><b>--</b></div></div></header>
      <button id="enter-field" class="field-gate" type="button"><i></i><b>Field</b><span>Enter hunting area</span></button>
      <button id="field-back" class="field-back" type="button">Return to village</button>
      <div id="panel-message" class="panel-message" aria-live="polite"><b>Village</b><span>Choose a menu branch.</span></div>
      <nav class="bottom-menu" aria-label="Village menu">${menuItems.map((item) => `<button class="menu-button" type="button" data-menu="${item.id}"><span class="menu-icon"><img src="${originalAsset(item.icon)}" alt="" /></span><b>${item.label}</b></button>`).join("")}</nav>
      <button id="connection-status" class="connection-status connecting" type="button" aria-label="Server connection status"><i></i><span>Connecting</span></button>
    </section>
    <section id="roster-screen" class="roster-screen" aria-label="Hunter roster" aria-hidden="true">
      <img class="roster-background" src="${originalAsset("sprites/character_info__7588.png")}" alt="" />
      <div class="roster-card"><b>Hunter roster</b><span>Authoritative roster binding is confirmed; starter composition and stats remain unresolved.</span><small>Hunter Spine source · confirmed</small></div>
      <button id="roster-back" class="roster-back" type="button">Back to village</button>
    </section>
    <div id="loading-transition" class="loading-transition" hidden><img src="${originalAsset("sprites/cloud_loading_btn__4266.png")}" alt="" /><span>Loading...</span></div>
  </main>`;

function element<T extends HTMLElement>(selector: string): T { const value = document.querySelector<T>(selector); if (!value) throw new Error(`Missing UI element ${selector}`); return value; }
const bootScreen = element<HTMLElement>("#boot-screen");
const villageScreen = element<HTMLElement>("#village-screen");
const rosterScreen = element<HTMLElement>("#roster-screen");
const transition = element<HTMLElement>("#loading-transition");
const panelMessage = element<HTMLElement>("#panel-message");
const connectionStatus = element<HTMLButtonElement>("#connection-status");
const worldViewport = element<HTMLElement>("#world-viewport");
const enterField = element<HTMLButtonElement>("#enter-field");
const fieldBack = element<HTMLButtonElement>("#field-back");
const worldModeLabel = element<HTMLElement>("#world-mode-label");
let latestSnapshot: OriginalFlowSnapshot | null = null;
let world: VisibleEntityWorld | null = null;

const client = new WorldClient(renderSnapshot, updateConnectionStatus, showIntentResult, showBindingBlocked);
element<HTMLButtonElement>("#enter-village").addEventListener("click", () => { transition.hidden = false; client.completeBoot(); });
document.querySelectorAll<HTMLButtonElement>("[data-menu]").forEach((button) => button.addEventListener("click", () => {
  document.querySelectorAll("[data-menu]").forEach((item) => item.classList.remove("selected"));
  button.classList.add("selected");
  client.selectBottomMenu(button.dataset.menu as BottomMenuIntent);
}));
element<HTMLButtonElement>("#roster-back").addEventListener("click", () => client.navigateBack());
enterField.addEventListener("click", () => client.enterField());
fieldBack.addEventListener("click", () => client.navigateBack());
connectionStatus.addEventListener("click", () => client.requestResync());
void initializeWorld().then(() => client.connect()).catch((error: unknown) => {
  console.error("Failed to initialize the visible world.", error);
  transition.hidden = true;
  panelMessage.innerHTML = "<b>Content unavailable</b><span>The visible-world release could not be loaded.</span>";
  client.connect();
});

async function initializeWorld(): Promise<void> {
  const app = new Application();
  await app.init({ resizeTo: worldViewport, backgroundAlpha: 0, antialias: false, autoDensity: true, resolution: Math.min(devicePixelRatio, 2) });
  worldViewport.appendChild(app.canvas);
  const visibleWorld = new VisibleEntityWorld((entityId) => {
    if (!client.selectEntity(entityId)) return;
    panelMessage.innerHTML = `<b>Entity selected</b><span>${entityId}</span>`;
  });
  await visibleWorld.initialize();
  app.stage.addChild(visibleWorld.root);
  const resize = (): void => visibleWorld.resize(worldViewport.clientWidth, worldViewport.clientHeight);
  const resizeObserver = new ResizeObserver(resize);
  resizeObserver.observe(worldViewport);
  resize();
  app.ticker.add((ticker) => visibleWorld.tick(ticker.deltaMS / 1000));
  world = visibleWorld;
  if (latestSnapshot) {
    visibleWorld.setMode(latestSnapshot.screen === "field" ? "field" : "village");
    visibleWorld.update(latestSnapshot.world.entities);
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
  world?.update(snapshot.world.entities);
  transition.hidden = true;
  const village = snapshot.screen === "village" || snapshot.screen === "field";
  const roster = snapshot.screen === "hunter_roster";
  bootScreen.classList.toggle("leaving", !snapshot.screen || snapshot.screen !== "boot");
  villageScreen.classList.toggle("visible", village);
  villageScreen.classList.toggle("field-mode", snapshot.screen === "field");
  villageScreen.setAttribute("aria-hidden", String(!village));
  rosterScreen.classList.toggle("visible", roster);
  rosterScreen.setAttribute("aria-hidden", String(!roster));
  worldModeLabel.textContent = snapshot.screen === "field" ? "Field" : "Village";
  enterField.hidden = snapshot.screen !== "village";
  fieldBack.hidden = snapshot.screen !== "field";
}

function showIntentResult(result: IntentFeedback): void {
  if (result.accepted) {
    panelMessage.innerHTML = `<b>${result.intent.replaceAll("_", " ")}</b><span>Server accepted.</span>`;
  } else panelMessage.innerHTML = `<b>Server response</b><span>${result.reason ?? "Intent rejected"}</span>`;
}
function showBindingBlocked(result: BindingBlockedFeedback): void {
  panelMessage.innerHTML = `<b>Binding blocked</b><span>${result.intent.replaceAll("_", " ")} · ${result.blockers.join(", ")}</span>`;
}
function updateConnectionStatus(status: ConnectionStatus): void {
  const labels: Record<ConnectionStatus, string> = { connecting: "Connecting", online: "Server online", reconnecting: "Reconnecting", offline: "Offline" };
  connectionStatus.className = `connection-status ${status}`;
  connectionStatus.querySelector("span")!.textContent = labels[status];
}
window.addEventListener("beforeunload", () => client.disconnect(), { once: true });
