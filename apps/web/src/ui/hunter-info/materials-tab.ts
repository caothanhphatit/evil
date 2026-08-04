import { node, sourceImage, unavailable } from "./dom";
import type { HunterInfoView } from "./model";
import { formatNumber, t, type MessageKey } from "../../i18n";

const UNRESOLVED_PLACEHOLDER_COUNT = 12;

export function renderMaterialsTab(info: HunterInfoView, equipWeapon?: (gearInstanceId: string) => void): HTMLElement {
  const root = node("section", "hunter-info-materials-tab");
  root.append(node("h3", "", t("hunter.inventory.weapons")));
  const weapons = node("div", "hunter-weapon-inventory");
  if (info.weapons.length === 0) {
    weapons.append(unavailable(t("hunter.inventory.weapons_empty")));
  } else {
    for (const weapon of info.weapons) {
      const row = node("article", `hunter-weapon-row quality-${weapon.quality}${weapon.compatible ? "" : " incompatible"}`);
      const icon = node("span", "hunter-weapon-icon");
      icon.append(sourceImage(weapon.icon, weapon.nameVi));
      const copy = node("span", "hunter-weapon-copy");
      const heading = node("span", "hunter-weapon-heading");
      heading.append(
        node("b", "", `${weapon.nameVi}${weapon.enhancementLevel > 0 ? ` +${weapon.enhancementLevel}` : ""}`),
        node("i", "", qualityLabel(weapon.quality)),
      );
      copy.append(
        heading,
        node("small", "", weapon.nameEn),
        node("strong", "", t("hunter.inventory.attack_damage", { value: formatNumber(weapon.attackDamage) })),
        node("small", "", t("hunter.inventory.base_range", { min: formatNumber(weapon.attackDamageMin), max: formatNumber(weapon.attackDamageMax) })),
      );
      if (!weapon.compatible) copy.append(node("em", "", t("hunter.inventory.incompatible")));
      const action = node("button", "source-green-button hunter-weapon-equip", weapon.equipped
        ? t("hunter.inventory.equipped")
        : t("hunter.inventory.equip"));
      action.type = "button";
      action.disabled = weapon.equipped || !weapon.compatible || !equipWeapon;
      if (!action.disabled) action.addEventListener("click", () => equipWeapon?.(weapon.instanceId));
      row.append(icon, copy, action);
      weapons.append(row);
    }
  }
  root.append(weapons);
  root.append(node("h3", "", t("hunter.materials.title")));
  const grid = node("div", `hunter-material-grid${info.materials === null ? " unresolved" : ""}`);
  if (info.materials === null) {
    appendEmptySlots(grid, UNRESOLVED_PLACEHOLDER_COUNT);
    root.append(grid, unavailable(t("hunter.materials.unsynchronized")));
    return root;
  }
  for (const item of info.materials) {
    const cell = node("div", "hunter-material-cell");
    cell.title = item.name ?? item.id;
    cell.append(sourceImage(item.icon, item.name ?? item.id), node("b", "", String(item.quantity)));
    grid.append(cell);
  }
  root.append(grid);
  return root;
}

function qualityLabel(quality: number): string {
  const key = (["regular", "sturdy", "refined", "powerful", "supreme"][quality] ?? "regular");
  return t(`craft.quality.${key}` as MessageKey);
}

function appendEmptySlots(grid: HTMLElement, count: number): void {
  for (let index = 0; index < count; index += 1) {
    grid.append(node("div", "hunter-material-cell empty"));
  }
}
