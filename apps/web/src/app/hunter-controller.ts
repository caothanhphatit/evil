import type { OriginalFlowSnapshot } from "../generated/protocol";
import type { WorldClient } from "../net/world-client";
import type { VisibleEntityWorld } from "../game/visible-world";
import { hunterClassTone, hunterPercent, hunterRarityPresentation, hunterWorldEntityId, projectHunterRoster, type HunterView } from "../ui/hunter-roster";
import { projectHunterInfo } from "../ui/hunter-info/project";
import type { createHunterRosterActors } from "../ui/hunter-roster-actors";
import type { createHunterInfoModal } from "../ui/hunter-info/modal";
import type { createHunterWorldCommandMenu, HunterGearEnhancementRequestIntent, HunterWorldCommandIntent } from "../ui/hunter-world-command";
import type { BuildingRenderingContext } from "./building-renderer";
import { t, type MessageKey } from "../i18n";
import { originalAsset } from "./shell";

export interface HunterControllerContext {
  client: WorldClient;
  latestSnapshot: OriginalFlowSnapshot | null;
  selectedHunterId: string | null;
  releasedWorldHunterEntityId: string | null;
  hunterRosterActors: ReturnType<typeof createHunterRosterActors>;
  hunterCapacity: HTMLElement;
  hunterActiveList: HTMLElement;
  hunterRosterStatus: HTMLElement;
  hunterInfoModal: ReturnType<typeof createHunterInfoModal>;
  worldHunterInfoModal: ReturnType<typeof createHunterInfoModal>;
  hunterWorldCommandMenu: ReturnType<typeof createHunterWorldCommandMenu>;
  world: VisibleEntityWorld | null;
  worldViewport: HTMLElement;
  originalAsset: typeof originalAsset;
  buildingContext: BuildingRenderingContext;
  setHunterRosterVisibility(open: boolean): void;
  showPanelMessage(title: string, detail: string): void;
  openHunterShop(hunterId: number, shopId: "build_7" | "build_8" | "build_20"): void;
}

export function createHunterController(context: HunterControllerContext) {
  function renderHunterRoster(snapshot: OriginalFlowSnapshot): void {
    const roster = projectHunterRoster(snapshot, context.selectedHunterId);
    context.selectedHunterId = roster.selectedId;
    context.hunterCapacity.textContent = `${roster.active.length} / ${roster.capacity}`;
    context.hunterCapacity.classList.toggle("full", roster.active.length >= roster.capacity);
    context.hunterActiveList.replaceChildren(...Array.from({ length: roster.capacity }, (_, index) => {
      const hunter = roster.active[index];
      return hunter ? hunterRosterCard(hunter, snapshot) : emptyHunterSlot(index + 1);
    }));
    void context.hunterRosterActors.render(roster.active.slice(0, roster.capacity)).catch((error: unknown) => {
      console.error("Could not render current Hunter roster actors.", error);
      context.hunterRosterStatus.textContent = t("roster.actor_load_failed");
      context.hunterRosterStatus.classList.add("error");
    });
    context.hunterRosterStatus.textContent = roster.constraintViolation
      ?? (!roster.resolved ? t("roster.waiting_server") : "");
    context.hunterRosterStatus.classList.toggle("error", roster.constraintViolation !== null);
    if (context.hunterInfoModal.visible()) {
      const selected = [...roster.active, ...roster.waiting].find((hunter) => hunter.id === context.selectedHunterId);
      if (selected) context.hunterInfoModal.refresh(projectHunterInfo(rawHunterFor(snapshot, selected), selected));
    }
  }
  
  function hunterRosterCard(hunter: HunterView, snapshot: OriginalFlowSnapshot): HTMLElement {
    const rarity = hunterRarityPresentation(hunter.rarityId, hunter.rarityName);
    const card = document.createElement("article");
    card.className = `hunter-roster-card${rarity ? ` rarity-${rarity.key}` : " rarity-unresolved"}${hunter.id === context.selectedHunterId ? " selected" : ""}`;
    card.dataset.hunterId = hunter.id;
    card.setAttribute("aria-selected", String(hunter.id === context.selectedHunterId));
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
    const openHunterInfo = (trigger: HTMLButtonElement): void => {
      trigger.blur();
      selectHunterCard(hunter.id);
      const raw = rawHunterFor(snapshot, hunter);
      context.hunterInfoModal.show(projectHunterInfo(raw, hunter));
    };
    const avatar = document.createElement("button");
    avatar.type = "button";
    avatar.className = "hunter-avatar";
    avatar.setAttribute("aria-label", t("roster.info_aria", { name: hunter.name }));
    avatar.addEventListener("click", () => openHunterInfo(avatar));
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
    info.addEventListener("click", () => openHunterInfo(info));
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
      context.setHunterRosterVisibility(false);
      context.hunterInfoModal.close();
      if (!context.client.selectEntity(worldEntityId)) return;
      if (!context.world?.focusEntity(worldEntityId)) return;
      // Locate uses the same post-click command bubble as selecting the actor in
      // the context.world; after camera focus the actor is centered in the viewport.
      context.hunterWorldCommandMenu.selectHunter({
        entityId: worldEntityId,
        displayName: hunter.name,
        screenPoint: { x: context.worldViewport.clientWidth / 2, y: context.worldViewport.clientHeight / 2 },
      });
    });
    const actions = document.createElement("footer");
    actions.append(info, target);
    card.append(heading, avatar, meta, actions);
    return card;
  }
  
  function selectHunterCard(id: string): void {
    context.selectedHunterId = id;
    document.querySelectorAll<HTMLElement>(".hunter-roster-card[data-hunter-id]").forEach((card) => {
      const selected = card.dataset.hunterId === id;
      card.classList.toggle("selected", selected);
      card.setAttribute("aria-selected", String(selected));
    });
  }
  
  function hunterForWorldEntity(snapshot: OriginalFlowSnapshot | null, entityId: string): HunterView | null {
    if (!snapshot) return null;
    const roster = projectHunterRoster(snapshot, context.selectedHunterId);
    return roster.active.find((hunter) => hunterWorldEntityId(snapshot, hunter) === entityId) ?? null;
  }
  
  function showWorldHunterInfo(entityId: string): void {
    const hunter = hunterForWorldEntity(context.latestSnapshot, entityId);
    if (!hunter || !context.latestSnapshot) {
      context.showPanelMessage(t("error.hunter_binding_unresolved"), entityId);
      return;
    }
    context.selectedHunterId = hunter.id;
    context.worldHunterInfoModal.show(projectHunterInfo(rawHunterFor(context.latestSnapshot, hunter), hunter));
  }
  
  function useHunterSkillFromInfo(hunterId: number, skillId: string): void {
    if (context.client.useHunterSkill(hunterId, skillId, null)) {
      context.showPanelMessage(t("feedback.skill_sent"), t("feedback.skill_checking"));
    } else {
      context.showPanelMessage(t("feedback.skill_failed"), t("feedback.server_not_ready"));
    }
  }
  
  function handleHunterWorldCommandIntent(intent: HunterWorldCommandIntent): void {
    context.worldViewport.dispatchEvent(new CustomEvent<HunterWorldCommandIntent>("hunter-context.world-command-intent", {
      detail: intent,
      bubbles: true,
    }));
    const hunter = hunterForWorldEntity(context.latestSnapshot, intent.hunterEntityId);
    if (hunter?.numericId === null || hunter?.numericId === undefined) {
      context.showPanelMessage(t("feedback.command_failed"), t("feedback.hunter_id_unbound"));
      return;
    }
    if (intent.type === "sell_hunter_loot") {
      if (context.client.sellHunterLoot(hunter.numericId)) {
        context.showPanelMessage(t("feedback.sale_sent"), t("feedback.sale_checking"));
      } else {
        context.showPanelMessage(t("feedback.sale_failed"), t("feedback.server_not_ready"));
      }
      return;
    }
    if (intent.type === "request_hunter_shop") {
      context.openHunterShop(hunter.numericId, intent.shopId);
      return;
    }
    if (!context.client.assignHunterHunt(hunter.numericId, intent.regionId)) {
      context.showPanelMessage(t("feedback.command_failed"), t("feedback.server_not_ready"));
    }
  }
  
  function handleHunterEnhancementRequest(intent: HunterGearEnhancementRequestIntent): void {
    const hunter = hunterForWorldEntity(context.latestSnapshot, intent.hunterEntityId);
    if (hunter?.numericId === null || hunter?.numericId === undefined) {
      context.showPanelMessage(t("feedback.command_failed"), t("feedback.hunter_id_unbound"));
      return;
    }
    if (!context.client.startHunterEnhancement(hunter.numericId)) {
      context.showPanelMessage(t("feedback.command_failed"), t("feedback.server_not_ready"));
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
  
  
  return {
    renderHunterRoster,
    hunterForWorldEntity,
    showWorldHunterInfo,
    useHunterSkillFromInfo,
    handleHunterWorldCommandIntent,
    handleHunterEnhancementRequest,
    rawHunterFor,
  };
}
