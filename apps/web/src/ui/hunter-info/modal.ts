import { node, sourceImage } from "./dom";
import { renderGrowthTab } from "./growth-tab";
import { renderMaterialsTab } from "./materials-tab";
import type { HunterInfoTabId, HunterInfoView } from "./model";
import { renderRidingPetTab } from "./riding-pet-tab";
import { renderSkillsTab } from "./skills-tab";
import { renderStatusTab } from "./status-tab";

const TABS: Array<{ id: HunterInfoTabId; label: string }> = [
  { id: "status", label: "Status" },
  { id: "skills", label: "Skills" },
  { id: "growth", label: "Growth" },
  { id: "riding", label: "Riding Pet" },
  { id: "materials", label: "Materials" },
];

export interface HunterInfoModalController {
  show(info: HunterInfoView): void;
  close(): void;
  visible(): boolean;
}

export function createHunterInfoModal(host: HTMLElement): HunterInfoModalController {
  let activeTab: HunterInfoTabId = "status";
  let current: HunterInfoView | null = null;
  const overlay = node("section", "hunter-info-overlay");
  overlay.hidden = true;
  overlay.setAttribute("role", "dialog");
  overlay.setAttribute("aria-modal", "true");
  overlay.setAttribute("aria-label", "Hunter information");
  host.append(overlay);

  const close = (): void => { overlay.hidden = true; current = null; };
  const render = (): void => {
    if (!current) return;
    overlay.replaceChildren(buildModal(current, activeTab, (tab) => { activeTab = tab; render(); }, close));
  };
  overlay.addEventListener("click", (event) => { if (event.target === overlay) close(); });
  return {
    show(info) { current = info; activeTab = "status"; overlay.hidden = false; render(); },
    close,
    visible: () => !overlay.hidden,
  };
}

function buildModal(info: HunterInfoView, activeTab: HunterInfoTabId, selectTab: (tab: HunterInfoTabId) => void, close: () => void): HTMLElement {
  const panel = node("article", "hunter-info-modal");
  const header = node("header", "hunter-info-header");
  header.append(node("span", "hunter-info-silhouette", info.hunter.classFamily ?? "H"), node("b", "", info.title));
  if (info.locked !== null) header.append(node("span", `hunter-info-lock${info.locked ? " locked" : ""}`, info.locked ? "Locked" : "Unlocked"));
  panel.append(header, buildHero(info), buildTabs(activeTab, selectTab));
  const body = node("div", "hunter-info-tab-body");
  body.append(renderTab(activeTab, info));
  panel.append(body);
  const closeButton = node("button", "source-red-button hunter-info-close", "Close");
  closeButton.type = "button";
  closeButton.addEventListener("click", close);
  panel.append(closeButton);
  return panel;
}

function buildHero(info: HunterInfoView): HTMLElement {
  const hero = node("section", "hunter-info-hero");
  const top = node("div", "hunter-info-reincarnation");
  if (info.reincarnation) {
    const stars = node("span", "hunter-reincarnation-stars");
    for (let index = 0; index < info.reincarnation.maximum; index += 1) stars.append(node("i", index < info.reincarnation.current ? "on" : ""));
    top.append(stars, node("small", "", "Reincarnation"));
  }
  if (info.hunter.gold !== null) top.append(node("b", "hunter-info-money", info.hunter.gold.toLocaleString()));
  hero.append(top);

  const stage = node("div", "hunter-loadout-stage");
  const leftSlots = node("div", "hunter-equipment-column left");
  const rightSlots = node("div", "hunter-equipment-column right");
  info.equipment.forEach((slot, index) => (index % 2 === 0 ? leftSlots : rightSlots).append(equipmentSlot(slot.icon, slot.placeholderIcon, slot.locked)));
  const paperDoll = node("div", "hunter-paper-doll");
  if (info.hunter.portrait) paperDoll.append(sourceImage(info.hunter.portrait, info.hunter.name));
  else paperDoll.append(node("span", "", "Look unavailable"));
  stage.append(leftSlots, paperDoll, rightSlots);
  hero.append(stage);
  if (info.experience) {
    const exp = node("div", "hunter-experience");
    const track = node("i");
    const fill = node("i");
    fill.style.width = `${percent(info.experience.current, info.experience.maximum)}%`;
    track.append(fill);
    exp.append(track, node("b", "", `EXP ${info.experience.current}/${info.experience.maximum}`));
    hero.append(exp);
  }
  return hero;
}

function equipmentSlot(icon: string | null, placeholder: string | null, locked: boolean | null): HTMLElement {
  const slot = node("span", `hunter-equipment-slot${locked ? " locked" : ""}`);
  if (icon) slot.append(sourceImage(icon));
  else if (placeholder) slot.append(sourceImage(placeholder));
  return slot;
}

function buildTabs(active: HunterInfoTabId, selectTab: (tab: HunterInfoTabId) => void): HTMLElement {
  const tabs = node("nav", "hunter-info-tabs");
  tabs.setAttribute("aria-label", "Hunter information sections");
  for (const tab of TABS) {
    const button = node("button", tab.id === active ? "active" : "", tab.label);
    button.type = "button";
    button.setAttribute("aria-pressed", String(tab.id === active));
    button.addEventListener("click", () => selectTab(tab.id));
    tabs.append(button);
  }
  return tabs;
}

function renderTab(tab: HunterInfoTabId, info: HunterInfoView): HTMLElement {
  if (tab === "status") return renderStatusTab(info);
  if (tab === "skills") return renderSkillsTab(info);
  if (tab === "growth") return renderGrowthTab(info);
  if (tab === "riding") return renderRidingPetTab(info);
  return renderMaterialsTab(info);
}

function percent(current: number, maximum: number): number { return maximum > 0 ? Math.max(0, Math.min(100, (current / maximum) * 100)) : 0; }
