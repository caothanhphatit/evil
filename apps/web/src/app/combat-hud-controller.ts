import type { CombatHudState } from "../ui/combat-hud";
import { t } from "../i18n";

export function createCombatHudController(debugUi: boolean, combatHud: HTMLElement, equipButton: HTMLButtonElement) {
  let latest: CombatHudState | null = null;
  function render(state: CombatHudState): void {
    latest = state;
    combatHud.hidden = !debugUi || !state.visible;
    if (!state.visible) return;
    const text = (id: string, value: string): void => { document.querySelector<HTMLElement>(id)!.textContent = value; };
    text("#combat-evidence", `${state.evidenceLabel} · ${t("combat.tick", { tick: state.tick })} · ${state.fighting ? t("combat.fighting") : t("combat.idle")}`);
    text("#hunter-state", state.hunter.state); text("#hunter-position", t("combat.position", { position: state.hunter.position }));
    text("#hunter-hp", t("combat.hp", { current: state.hunter.hp, maximum: state.hunter.maxHp }));
    document.querySelector<HTMLElement>("#hunter-hp-fill")!.style.width = `${state.hunter.percent}%`;
    text("#monster-state", state.monster.state); text("#monster-position", t("combat.position", { position: state.monster.position }));
    text("#monster-hp", t("combat.hp", { current: state.monster.hp, maximum: state.monster.maxHp }));
    document.querySelector<HTMLElement>("#monster-hp-fill")!.style.width = `${state.monster.percent}%`;
    text("#combat-gold", t("combat.gold", { amount: state.gold }));
    text("#combat-inventory", state.equipped ? [state.inventory, t("combat.item_equipped", { id: 2001 })].filter(Boolean).join(" · ") : state.inventory);
    text("#combat-drops", state.drops);
    equipButton.disabled = !state.equipEligible;
    equipButton.textContent = state.equipped ? t("combat.item_equipped", { id: 2001 }) : t("combat.equip_item", { id: 2001 });
  }
  return { render, latest: () => latest };
}
