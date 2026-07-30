import { node, sourceImage } from "./dom";
import { createHunterInfoActor } from "./actor";
import { renderGrowthTab } from "./growth-tab";
import { renderMaterialsTab } from "./materials-tab";
import type { HunterInfoEquipmentSlot, HunterInfoTabId, HunterInfoView } from "./model";
import { renderRidingPetTab } from "./riding-pet-tab";
import { renderSkillsTab } from "./skills-tab";
import { renderStatusTab } from "./status-tab";
import { formatNumber, t } from "../../i18n";

const TABS: Array<{ id: HunterInfoTabId; label: string }> = [
  { id: "status", label: t("hunter.tabs.status") },
  { id: "skills", label: t("hunter.tabs.skills") },
  { id: "materials", label: t("hunter.tabs.materials") },
  { id: "growth", label: t("hunter.tabs.growth") },
  { id: "riding", label: t("hunter.tabs.riding") },
];

const EQUIPMENT_PLACEHOLDERS: Readonly<Record<string, string>> = {
  gloves: "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-equipment/equip_dummy_07__6275.png",
  helmet: "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-equipment/equip_dummy_01__7584.png",
  necklace: "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-equipment/equip_dummy_03__5688.png",
  boots: "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-equipment/equip_dummy_06__1917.png",
  ring: "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-equipment/equip_dummy_02__4105.png",
  weapon: "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-equipment/equip_dummy_05__4925.png",
  armor: "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-equipment/equip_dummy_04__5943.png",
  belt: "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-equipment/equip_dummy_08__5673.png",
};
const UTILITY_SLOT_COUNT = 6;
const HUNTER_INFO_DESIGN_WIDTH = 440;
const HUNTER_INFO_DESIGN_HEIGHT = 690;
const LOADOUT_COLUMNS = {
  // The source paper-doll places helmet and belt on the center axis. The
  // remaining six equipment slots form three rows around the Hunter.
  left: ["ring", "weapon", "necklace"],
  right: ["armor", "gloves", "boots"],
} as const;

export interface HunterInfoModalController {
  preload(): Promise<void>;
  show(info: HunterInfoView): void;
  refresh(info: HunterInfoView): void;
  close(): void;
  visible(): boolean;
}

export interface HunterInfoModalActions {
  useSkill?(hunterId: number, skillId: string): void;
}

export function calculateHunterInfoScale(availableWidth: number, availableHeight: number): number {
  if (availableWidth <= 0 || availableHeight <= 0) return 1;
  return Math.min(1, availableWidth / HUNTER_INFO_DESIGN_WIDTH, availableHeight / HUNTER_INFO_DESIGN_HEIGHT);
}

export function calculateExperiencePercent(current: number, maximum: number): number {
  return percent(current, maximum);
}

export function createHunterInfoModal(host: HTMLElement, actions: HunterInfoModalActions = {}): HunterInfoModalController {
  let activeTab: HunterInfoTabId = "status";
  let current: HunterInfoView | null = null;
  const actor = createHunterInfoActor();
  const overlay = node("section", "hunter-info-overlay");
  overlay.hidden = true;
  overlay.setAttribute("role", "dialog");
  overlay.setAttribute("aria-modal", "true");
  overlay.setAttribute("aria-label", t("hunter.info.aria"));
  host.append(overlay);
  const scaleFrame = node("div", "hunter-info-scale-frame");
  let panel: HTMLElement | null = null;

  const syncScale = (): void => {
    if (overlay.hidden || !panel) return;
    const style = getComputedStyle(overlay);
    const availableWidth = overlay.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
    const availableHeight = overlay.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom);
    const scale = calculateHunterInfoScale(availableWidth, availableHeight);
    scaleFrame.style.width = `${HUNTER_INFO_DESIGN_WIDTH * scale}px`;
    scaleFrame.style.height = `${HUNTER_INFO_DESIGN_HEIGHT * scale}px`;
    panel.style.transform = `scale(${scale})`;
  };
  new ResizeObserver(syncScale).observe(overlay);

  const close = (): void => { overlay.hidden = true; current = null; panel = null; actor.clear(); };
  const render = (): void => {
    if (!current) return;
    const info = current;
    panel = buildModal(info, activeTab, (tab) => {
      activeTab = tab;
      // Keep the frame, equipment, and tab strip mounted while switching content.
      const body = panel?.querySelector<HTMLElement>(".hunter-info-tab-body");
      if (body) body.replaceChildren(renderTab(activeTab, info, actions));
      panel?.querySelectorAll<HTMLButtonElement>(".hunter-info-tabs button").forEach((button, index) => {
        const selected = TABS[index]?.id === activeTab;
        button.classList.toggle("active", selected);
        button.setAttribute("aria-pressed", String(selected));
      });
    }, close, actions);
    scaleFrame.replaceChildren(panel);
    overlay.replaceChildren(scaleFrame);
    syncScale();
    panel.focus({ preventScroll: true });
    const actorHost = panel.querySelector<HTMLElement>(".hunter-paper-doll.actor");
    if (actorHost) void actor.render(actorHost, info.hunter);
  };
  overlay.addEventListener("click", (event) => { if (event.target === overlay) close(); });
  return {
    preload: () => actor.preload(),
    show(info) { current = info; activeTab = "status"; overlay.hidden = false; render(); },
    refresh(info) {
      if (!current || !panel) return;
      current = info;
      panel.querySelector<HTMLElement>(".hunter-info-tab-body")?.replaceChildren(renderTab(activeTab, info, actions));
    },
    close,
    visible: () => !overlay.hidden,
  };
}

function buildModal(info: HunterInfoView, activeTab: HunterInfoTabId, selectTab: (tab: HunterInfoTabId) => void, close: () => void, actions: HunterInfoModalActions): HTMLElement {
  const panel = node("article", "hunter-info-modal");
  panel.tabIndex = -1;
  const header = node("header", "hunter-info-header");
  header.append(node("span", "hunter-info-silhouette", info.hunter.classFamily ?? "H"), node("b", "", info.title));
  if (info.locked !== null) {
    const lock = node("span", `hunter-info-lock${info.locked ? " locked" : ""}`);
    lock.title = info.locked ? t("hunter.info.locked") : t("hunter.info.unlocked");
    if (info.locked) lock.append(sourceImage("/content/releases/evil-hunter-1.411/hunter-assets/ui/equipment-hud/ic_hunter_lock__6607.png", t("hunter.info.locked_alt")));
    else lock.append(node("small", "", t("hunter.info.unlocked_label")));
    header.append(lock);
  }
  panel.append(header, buildHero(info), buildTabs(activeTab, selectTab));
  const body = node("div", "hunter-info-tab-body");
  body.append(renderTab(activeTab, info, actions));
  panel.append(body);
  const closeButton = node("button", "source-red-button hunter-info-close", t("common.close"));
  closeButton.type = "button";
  closeButton.addEventListener("click", close);
  panel.append(closeButton);
  return panel;
}

function buildHero(info: HunterInfoView): HTMLElement {
  const hero = node("section", "hunter-info-hero");
  const top = node("div", "hunter-info-reincarnation");
  const stars = node("span", "hunter-reincarnation-stars");
  const starMaximum = info.reincarnation?.maximum ?? 5;
  for (let index = 0; index < starMaximum; index += 1) stars.append(node("i", info.reincarnation && index < info.reincarnation.current ? "on" : ""));
  top.append(stars, node("small", "", t("hunter.info.reincarnation")));
  if (info.hunter.gold !== null) {
    const money = node("b", "hunter-info-money");
    money.append(sourceImage("/content/releases/original-flow-v1/sprites/top_ic_01_gold_24__4677.png", t("common.gold")), node("span", "", formatNumber(info.hunter.gold)));
    top.append(money);
  }
  hero.append(top);

  const stage = node("div", "hunter-loadout-stage");
  const leftSlots = node("div", "hunter-equipment-column left");
  const rightSlots = node("div", "hunter-equipment-column right");
  const leftUtility = node("div", "hunter-utility-column left");
  const rightUtility = node("div", "hunter-utility-column right");
  const centerSlots = node("div", "hunter-center-loadout");
  const equipmentDetail = node("section", "hunter-equipment-detail");
  equipmentDetail.hidden = true;
  equipmentDetail.setAttribute("aria-live", "polite");
  const selectEquipment = (equipment: HunterInfoEquipmentSlot): void => {
    const title = node("b", "", equipment.name ?? equipment.id);
    const description = node("small", "", equipmentDetailText(equipment));
    const close = node("button", "source-red-button", t("common.close"));
    close.type = "button";
    close.setAttribute("aria-label", t("hunter.info.close_equipment"));
    close.addEventListener("click", () => { equipmentDetail.hidden = true; });
    equipmentDetail.replaceChildren(title, description, close);
    equipmentDetail.hidden = false;
  };
  for (let index = 0; index < UTILITY_SLOT_COUNT / 2; index += 1) {
    leftUtility.append(utilitySlot());
    rightUtility.append(utilitySlot());
  }
  const equipment = new Map(info.equipment.map((slot) => [slot.id, slot]));
  for (const slotId of LOADOUT_COLUMNS.left) leftSlots.append(equipmentSlot(equipment.get(slotId) ?? null, EQUIPMENT_PLACEHOLDERS[slotId], selectEquipment, slotId));
  for (const slotId of LOADOUT_COLUMNS.right) rightSlots.append(equipmentSlot(equipment.get(slotId) ?? null, EQUIPMENT_PLACEHOLDERS[slotId], selectEquipment, slotId));
  centerSlots.append(equipmentSlot(equipment.get("helmet") ?? null, EQUIPMENT_PLACEHOLDERS.helmet, selectEquipment, "helmet"));
  const paperDoll = node("div", `hunter-paper-doll${info.hunter.portrait ? "" : " actor"}`);
  if (info.hunter.portrait) paperDoll.append(sourceImage(info.hunter.portrait, info.hunter.name));
  else paperDoll.append(node("span", "sr-only", t("hunter.info.runtime_appearance")));
  centerSlots.append(paperDoll);
  centerSlots.append(equipmentSlot(equipment.get("belt") ?? null, EQUIPMENT_PLACEHOLDERS.belt, selectEquipment, "belt"));
  stage.append(leftUtility, leftSlots, centerSlots, rightSlots, rightUtility);
  hero.append(stage, equipmentDetail);
  {
    const exp = node("div", `hunter-experience${info.experience ? "" : " unresolved"}`);
    const track = node("i");
    const fill = node("i");
    if (info.experience) {
      const progress = calculateExperiencePercent(info.experience.current, info.experience.maximum);
      const accessibleMaximum = Math.max(0, info.experience.maximum);
      track.setAttribute("role", "progressbar");
      track.setAttribute("aria-label", t("hunter.info.experience_aria"));
      track.setAttribute("aria-valuemin", "0");
      track.setAttribute("aria-valuemax", String(accessibleMaximum));
      track.setAttribute("aria-valuenow", String(Math.max(0, Math.min(info.experience.current, accessibleMaximum))));
      fill.style.width = `${progress}%`;
    } else {
      track.setAttribute("aria-hidden", "true");
    }
    track.append(fill);
    exp.append(track, node("b", "", info.experience
      ? t("hunter.info.exp", { current: info.experience.current, maximum: info.experience.maximum })
      : t("hunter.info.exp_unavailable")));
    hero.append(exp);
  }
  return hero;
}

function equipmentSlot(
  equipment: HunterInfoEquipmentSlot | null,
  fallbackPlaceholder: string | null,
  select: (equipment: HunterInfoEquipmentSlot) => void,
  slotId: string,
): HTMLElement {
  const slot = node("button", `hunter-equipment-slot${equipment?.locked ? " locked" : ""}`);
  slot.type = "button";
  slot.dataset.slotId = slotId;
  slot.disabled = equipment === null;
  slot.setAttribute("aria-label", equipment?.name ? t("hunter.info.view_equipment", { name: equipment.name }) : t("hunter.info.equipment_unavailable"));
  if (equipment) slot.addEventListener("click", () => select(equipment));
  const icon = equipment?.icon ?? equipment?.placeholderIcon ?? fallbackPlaceholder;
  if (icon) slot.append(sourceImage(icon));
  return slot;
}

export function equipmentDetailText(equipment: HunterInfoEquipmentSlot): string {
  const identity = [equipment.catalogKind, equipment.catalogIndex === null ? null : `#${equipment.catalogIndex}`]
    .filter((value): value is string => value !== null)
    .join(" ");
  const evidence = equipment.evidenceState
    ? t("hunter.info.evidence", { state: equipment.evidenceState })
    : t("hunter.info.evidence_unavailable");
  return identity ? `${identity} · ${evidence}` : evidence;
}

function utilitySlot(): HTMLElement {
  const slot = node("span", "hunter-utility-slot");
  slot.setAttribute("aria-label", t("hunter.info.utility_unavailable"));
  // The captured utility icon-to-slot bindings are unresolved; keep the
  // original empty-state marker instead of assigning a gear selection asset.
  slot.append(node("i"));
  return slot;
}

function buildTabs(active: HunterInfoTabId, selectTab: (tab: HunterInfoTabId) => void): HTMLElement {
  const tabs = node("nav", "hunter-info-tabs");
  tabs.setAttribute("aria-label", t("hunter.tabs.aria"));
  for (const tab of TABS) {
    const button = node("button", tab.id === active ? "active" : "", tab.label);
    button.type = "button";
    button.setAttribute("aria-pressed", String(tab.id === active));
    button.addEventListener("click", (event) => {
      selectTab(tab.id);
      if (event.detail > 0) button.blur();
    });
    tabs.append(button);
  }
  return tabs;
}

function renderTab(tab: HunterInfoTabId, info: HunterInfoView, actions: HunterInfoModalActions): HTMLElement {
  if (tab === "status") return renderStatusTab(info);
  if (tab === "skills") {
    const hunterId = info.hunter.numericId;
    return renderSkillsTab(info, hunterId === null || !actions.useSkill
      ? undefined
      : (skillId) => actions.useSkill?.(hunterId, skillId));
  }
  if (tab === "growth") return renderGrowthTab(info);
  if (tab === "riding") return renderRidingPetTab(info);
  return renderMaterialsTab(info);
}

function percent(current: number, maximum: number): number { return maximum > 0 ? Math.max(0, Math.min(100, (current / maximum) * 100)) : 0; }
