import { Application } from "pixi.js";
import type { OriginalFlowSnapshot } from "../generated/protocol";
import { WorldClient } from "../net/world-client";
import { VisibleEntityWorld } from "../game/visible-world";
import { TOWN_CAMERA_CLEAR_COLOR } from "../game/scene-projection";
import { projectCombatHud, type CombatHudState } from "../ui/combat-hud";
import { syncWorldFocusMenu } from "../ui/bottom-menu-state";
import { projectResourceBar } from "../ui/resource-projection";
import { setPanelMessage } from "../ui/panel-message";
import { hunterWorldEntityId, projectHunterRoster } from "../ui/hunter-roster";
import { projectAuthoritativeMonsterField } from "../ui/monster-field";
import { projectHunterInfo } from "../ui/hunter-info/project";
import { projectHunterEnhancementInteraction } from "../ui/hunter-enhancement-entry";
import { preloadUiAssets } from "./entry-controller";
import { createOpenHunterEnhancementIntent } from "../ui/hunter-enhancement-entry";
import { findBuildingInstanceById } from "../game/building-placement";
import { projectBuildingEvidence } from "../content/building-evidence";
import { ENHANCEMENT_FORGE_BUILDING_IDS } from "../content/blacksmith-route";
import type { BuildingRenderingContext } from "./building-renderer";
import type { HunterControllerContext } from "./hunter-controller";
import { t } from "../i18n";
import { recordClientEvent } from "../observability/client-telemetry";

export interface WorldControllerContext {
  client: WorldClient;
  app?: Application;
  latestSnapshot: OriginalFlowSnapshot | null;
  releasedWorldHunterEntityId: string | null;
  hunterController: ReturnType<typeof import("./hunter-controller").createHunterController>;
  hunterWorldCommandMenu: ReturnType<typeof import("../ui/hunter-world-command").createHunterWorldCommandMenu>;
  worldHunterInfoModal: ReturnType<typeof import("../ui/hunter-info/modal").createHunterInfoModal>;
  world: VisibleEntityWorld | null;
  buildingContext: BuildingRenderingContext;
  buildingRenderer: ReturnType<typeof import("./building-renderer").createBuildingRenderer>;
  entryController: import("./entry-controller").EntryController;
  hunterRosterActors: HunterControllerContext["hunterRosterActors"];
  hunterInfoModal: HunterControllerContext["hunterInfoModal"];
  worldViewport: HTMLElement;
  showPanelMessage(title: string, detail: string): void;
  debugUi: boolean;
  evidenceDiagnostics: HTMLElement;
  fpsCounter: HTMLElement;
  hunterEnhancementInteractions: HTMLElement;
  hunterRosterOpen: boolean;
  hunterRosterPrimed: boolean;
  nextHunterRosterRefreshAt: number;
  selectedMenuAction: string | null;
  bottomMenu: HTMLElement;
  worldModeLabel: HTMLElement;
  goldAmount: HTMLElement;
  hunterPopulation: HTMLElement;
  fieldBack: HTMLButtonElement;
  popupSnapshotSignature: string;
  popupInteractionActive: boolean;
  gearCreatePop: HTMLElement;
  consumCreatePop: HTMLElement;
  renderCombatHud(state: CombatHudState): void;
  syncEnhancementTaskView(snapshot: OriginalFlowSnapshot): void;
}
export function createWorldController(context: WorldControllerContext) {
  async function initializeWorld(): Promise<void> {
  const app = new Application();
  await app.init({ resizeTo: context.worldViewport, backgroundColor: TOWN_CAMERA_CLEAR_COLOR, backgroundAlpha: 1, antialias: false, autoDensity: true, resolution: Math.min(devicePixelRatio, 2) });
  context.worldViewport.appendChild(app.canvas);
  const visibleWorld = new VisibleEntityWorld((entityId, screenPoint) => {
    context.releasedWorldHunterEntityId = null;
    const entity = context.latestSnapshot?.world.entities.find((candidate) => candidate.descriptor.entity_id === entityId);
    if (!entity || entity.descriptor.kind === "monster") return;
    if (!context.client.selectEntity(entityId)) return;
    if (entity?.descriptor.kind === "hunter") {
      const hunter = context.hunterController.hunterForWorldEntity(context.latestSnapshot, entityId);
      context.hunterWorldCommandMenu.selectHunter({
        entityId,
        displayName: hunter?.name ?? entityId,
        screenPoint: screenPoint ?? { x: context.worldViewport.clientWidth / 2, y: context.worldViewport.clientHeight / 2 },
      });
      return;
    }
    context.hunterWorldCommandMenu.close();
    context.worldHunterInfoModal.close();
    context.showPanelMessage(t("error.entity_selected"), entityId);
  }, (instance, visual) => {
    context.hunterWorldCommandMenu.close();
    context.worldHunterInfoModal.close();
    if (ENHANCEMENT_FORGE_BUILDING_IDS.includes(instance.building_id as typeof ENHANCEMENT_FORGE_BUILDING_IDS[number])) {
      context.showPanelMessage(t("enhancement.forge_title"), t("enhancement.forge_instruction"));
      return;
    }
    context.buildingContext.selectedBuildingId = instance.building_id;
    context.buildingContext.selectedBuildingInstanceId = instance.instance_id;
    context.buildingContext.selectedBuildingVisual = visual;
    context.buildingContext.buildingPanelMode = "building";
    const evidence = projectBuildingEvidence(context.buildingContext.buildingEvidenceRegistry, instance.building_id);
    if (!evidence) {
      context.buildingContext.buildingPanel.hidden = true;
      context.showPanelMessage(t("error.building_binding_unresolved"), instance.building_id);
      return;
    }
    context.buildingContext.buildingPanel.hidden = false;
    context.buildingRenderer.renderBuildingSystem(context.latestSnapshot);
  }, (regionId, nextLevel) => {
    if (!context.client.setMonsterRegionDensity(regionId, nextLevel)) return;
    context.showPanelMessage(t("error.monster_density"), `${regionId}: ${["I", "II", "III"][nextLevel - 1] ?? nextLevel}`);
  }, (diagnostic) => {
    recordClientEvent("warn", "world_visual_tick_drift", {
      authoritative_tick: diagnostic.authoritativeTick,
      visual_tick: diagnostic.visualTick,
      drift_ticks: diagnostic.driftTicks,
      threshold_ticks: diagnostic.thresholdTicks,
    });
  });
  const diagnostics = await visibleWorld.initialize((loaded, total) => {
    context.entryController.updateMapProgress(loaded, total);
  });
  context.entryController.prepareHunters();
  await Promise.all([
    context.hunterRosterActors.preload(),
    context.hunterInfoModal.preload(),
    context.worldHunterInfoModal.preload(),
    preloadUiAssets(),
  ]);
  if (context.debugUi) {
    context.evidenceDiagnostics.hidden = false;
    setPanelMessage(context.evidenceDiagnostics, diagnostics.fixture ? t("diagnostics.fixture") : t("diagnostics.runtime"), t("diagnostics.unresolved", { items: diagnostics.unresolved.join(", ") }));
  }
  app.stage.addChild(visibleWorld.root);
  const resize = (): void => visibleWorld.resize(context.worldViewport.clientWidth, context.worldViewport.clientHeight);
  const resizeObserver = new ResizeObserver(resize);
  resizeObserver.observe(context.worldViewport);
  resize();
  let dragging = false;
  let dragCaptured = false;
  let lastX = 0;
  let lastY = 0;
  let pointerDownX = 0;
  let pointerDownY = 0;
  context.worldViewport.addEventListener("pointerdown", (event) => {
    dragging = true;
    dragCaptured = false;
    lastX = event.clientX;
    lastY = event.clientY;
    pointerDownX = event.clientX;
    pointerDownY = event.clientY;
  });
  context.worldViewport.addEventListener("pointermove", (event) => {
    if (!dragging) return;
    if (!dragCaptured && Math.hypot(event.clientX - pointerDownX, event.clientY - pointerDownY) < 5) return;
    if (!dragCaptured) {
      dragCaptured = true;
      context.worldViewport.setPointerCapture(event.pointerId);
    }
    visibleWorld.panBy(event.clientX - lastX, event.clientY - lastY);
    if (context.latestSnapshot) renderHunterEnhancementInteractions(context.latestSnapshot);
    lastX = event.clientX;
    lastY = event.clientY;
  });
  context.worldViewport.addEventListener("pointerup", (event) => {
    dragging = false;
    if (dragCaptured && context.worldViewport.hasPointerCapture(event.pointerId)) context.worldViewport.releasePointerCapture(event.pointerId);
    dragCaptured = false;
  });
  context.worldViewport.addEventListener("pointercancel", () => { dragging = false; dragCaptured = false; });
  context.worldViewport.addEventListener("wheel", (event) => {
    event.preventDefault();
    visibleWorld.zoomBy(event.deltaY > 0 ? -0.1 : 0.1);
    if (context.latestSnapshot) renderHunterEnhancementInteractions(context.latestSnapshot);
  }, { passive: false });
  let fpsUpdatedAt = performance.now();
  app.ticker.add(() => {
    visibleWorld.tick();
    const now = performance.now();
    if (now - fpsUpdatedAt < 500) return;
    fpsUpdatedAt = now;
    context.fpsCounter.textContent = t("world.fps", { value: Math.round(app.ticker.FPS) });
  });
  context.world = visibleWorld;
  if (context.latestSnapshot) {
    context.buildingRenderer.syncBuildingPresentation(visibleWorld, context.latestSnapshot);
    visibleWorld.setMode(context.latestSnapshot.screen === "field" ? "field" : "village");
    visibleWorld.setMonsterDensityLevels(projectAuthoritativeMonsterField(context.latestSnapshot.monster_world).farms);
    visibleWorld.update(
      context.latestSnapshot.world.entities,
      context.latestSnapshot.world.visual_tick,
      context.latestSnapshot.world.combat_presentations,
      context.latestSnapshot.world.drops,
    );
  }
  // Generate the static render cache and paint the initial authoritative frame
  // while the boot screen still covers the world canvas.
  app.render();
  await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
  app.render();
  context.entryController.markMapReady(context.latestSnapshot, () => context.latestSnapshot);
  window.addEventListener("beforeunload", () => {
    resizeObserver.disconnect();
    visibleWorld.destroy();
    app.destroy(true);
  }, { once: true });
}

function renderSnapshot(snapshot: OriginalFlowSnapshot): void {
  context.latestSnapshot = snapshot;
  context.entryController.scheduleReveal(snapshot, () => context.latestSnapshot);
  context.world?.setMode(snapshot.screen === "field" ? "field" : "village");
  context.world?.setMonsterDensityLevels(projectAuthoritativeMonsterField(snapshot.monster_world).farms);
  if (context.world) context.buildingRenderer.syncBuildingPresentation(context.world, snapshot);
  context.world?.update(snapshot.world.entities, snapshot.world.visual_tick, snapshot.world.combat_presentations, snapshot.world.drops);
  renderHunterEnhancementInteractions(snapshot);
  if (snapshot.world.selected_entity_id !== context.releasedWorldHunterEntityId) context.releasedWorldHunterEntityId = null;
  context.world?.setSelectedEntity(snapshot.world.selected_entity_id === context.releasedWorldHunterEntityId
    ? null
    : snapshot.world.selected_entity_id);
  const village = context.entryController.syncScreens(snapshot, context.hunterRosterOpen);
  const roster = context.hunterRosterOpen;
  const commandHunterEntityId = context.hunterWorldCommandMenu.selectedEntityId();
  const commandHunter = commandHunterEntityId ? context.hunterController.hunterForWorldEntity(snapshot, commandHunterEntityId) : null;
  const commandHunterVisible = commandHunterEntityId === null || snapshot.world.entities.some((entity) => (
    entity.descriptor.entity_id === commandHunterEntityId && entity.descriptor.kind === "hunter"
  ));
  if (!village || roster || !commandHunterVisible) {
    context.hunterWorldCommandMenu.close();
    context.worldHunterInfoModal.close();
  } else if (context.worldHunterInfoModal.visible() && commandHunter) {
    context.worldHunterInfoModal.refresh(projectHunterInfo(context.hunterController.rawHunterFor(snapshot, commandHunter), commandHunter));
  }
  context.selectedMenuAction = syncWorldFocusMenu(snapshot.screen, context.selectedMenuAction);
  const activeMenuAction = roster ? "character" : context.selectedMenuAction;
  context.bottomMenu.querySelectorAll<HTMLElement>("[data-action]").forEach((item) => {
    item.classList.toggle("selected", item.dataset.action === activeMenuAction);
  });
  const now = performance.now();
  if (!context.hunterRosterPrimed && snapshot.screen === "village") {
    context.hunterController.renderHunterRoster(snapshot);
    context.hunterRosterPrimed = true;
    context.nextHunterRosterRefreshAt = now + 500;
  } else if (roster && now >= context.nextHunterRosterRefreshAt) {
    context.hunterController.renderHunterRoster(snapshot);
    context.hunterRosterPrimed = true;
    context.nextHunterRosterRefreshAt = now + 500;
  } else if (!roster && context.hunterInfoModal.visible()) context.hunterInfoModal.close();
  context.syncEnhancementTaskView(snapshot);
  context.worldModeLabel.textContent = snapshot.screen === "field" ? t("world.hunt") : t("world.easy");
  const nextPopupSignature = context.buildingRenderer.popupDataSignature(snapshot);
  if (!context.popupInteractionActive && nextPopupSignature !== context.popupSnapshotSignature) {
    context.popupSnapshotSignature = nextPopupSignature;
    if (nextPopupSignature !== "closed") {
      context.buildingRenderer.renderBuildingSystem(snapshot);
      if (!context.gearCreatePop.hidden) context.buildingRenderer.renderGearCreatePop();
      if (!context.consumCreatePop.hidden) context.buildingRenderer.renderConsumCreatePop();
    }
  }
  const resources = projectResourceBar(snapshot);
  const displayedGold = snapshot.screen === "village" ? snapshot.village.building_system.town_gold : resources.gold;
  context.goldAmount.textContent = displayedGold === null ? "--" : String(displayedGold);
  context.goldAmount.parentElement?.classList.toggle("unresolved", !resources.evidenceBacked);
  const population = projectHunterRoster(snapshot, null);
  context.hunterPopulation.textContent = `${population.active.length}/${population.capacity}`;
  context.fieldBack.hidden = snapshot.screen !== "field";
  context.renderCombatHud(projectCombatHud(snapshot.screen, snapshot.migration_fixture_combat));
}

function syncEnhancementTaskView(snapshot: OriginalFlowSnapshot): void {
  if (context.buildingContext.enhancementHunterId === null) return;
  const hunter = snapshot.hunter_roster.active_hunters.find((row) => row.hunter_id === context.buildingContext.enhancementHunterId);
  const task = hunter?.gear_enhancement_task;
  if (!task) {
    context.buildingContext.enhancementHunterId = null;
    context.buildingContext.enhancementView = "select";
    context.buildingContext.selectedEnhancementGearKey = null;
    context.buildingContext.selectedEnhancementOptionalMaterialIds = [];
    if (context.buildingContext.selectedBuildingId === "build_15") context.buildingContext.buildingPanel.hidden = true;
    return;
  }
  context.buildingContext.enhancementView = task.status === "configuring" ? "configure"
    : task.status === "processing" ? "processing"
      : task.status === "result" ? "result" : "select";
  if (task.selected_gear_instance_id) context.buildingContext.selectedEnhancementGearKey = task.selected_gear_instance_id;
  if (task.mode) context.buildingContext.selectedEnhancementMode = task.mode;
  context.buildingContext.selectedEnhancementOptionalMaterialIds = task.optional_material_ids;
}

function renderWorldFrame(snapshot: OriginalFlowSnapshot): void {
  context.latestSnapshot = snapshot;
  context.world?.update(
    snapshot.world.entities,
    snapshot.world.visual_tick,
    snapshot.world.combat_presentations,
    snapshot.world.drops,
  );
  renderHunterEnhancementInteractions(snapshot);
}

function renderHunterEnhancementInteractions(snapshot: OriginalFlowSnapshot): void {
  context.hunterEnhancementInteractions.replaceChildren();
  if (snapshot.screen !== "village" || !context.world) return;
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
    const point = context.world.screenPointForEntity(entityId);
    if (state.mode === "hidden" || !point) continue;
    if (state.mode === "traveling") {
      const indicator = document.createElement("span");
      indicator.className = "hunter-enhancement-travel-indicator";
      indicator.style.setProperty("--interaction-x", `${point.x}px`);
      indicator.style.setProperty("--interaction-y", `${point.y}px`);
      indicator.setAttribute("aria-label", t("enhancement.traveling_aria", { name: hunter.display_name }));
      indicator.textContent = t("world.hunter_initials");
      context.hunterEnhancementInteractions.append(indicator);
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
        context.showPanelMessage(t("enhancement.open_failed"), t("enhancement.forge_unavailable"));
        return;
      }
      context.buildingContext.enhancementHunterId = hunter.hunter_id;
      context.buildingContext.enhancementView = task.status === "result" ? "result" : "select";
      context.buildingContext.selectedEnhancementGearKey = null;
      context.buildingContext.selectedEnhancementMode = "single";
      context.buildingContext.selectedEnhancementOptionalMaterialIds = task.optional_material_ids;
      context.buildingContext.selectedBuildingId = "build_15";
      context.buildingContext.selectedBuildingInstanceId = instance.instance_id;
      context.buildingContext.selectedBuildingVisual = null;
      context.buildingContext.buildingPanelMode = "building";
      context.buildingContext.buildingPanel.hidden = false;
      context.buildingRenderer.renderBuildingSystem(context.latestSnapshot);
    });
    context.hunterEnhancementInteractions.append(button);
  }
}


  return { initializeWorld, renderSnapshot, renderWorldFrame, renderHunterEnhancementInteractions, syncEnhancementTaskView };
}
