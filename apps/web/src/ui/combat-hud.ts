import type { MigrationFixtureCombatProjection } from "../generated/protocol";
import { t } from "../i18n";

const FIXTURE_ITEM_ID = 2001;

export interface CombatHudState {
  visible: boolean;
  evidenceLabel: string;
  tick: number;
  fighting: boolean;
  gold: number;
  hunter: { hp: number; maxHp: number; percent: number; state: string; position: string };
  monster: { hp: number; maxHp: number; percent: number; state: string; position: string };
  ownedQuantity: number;
  inventory: string;
  equipped: boolean;
  equipEligible: boolean;
  drops: string;
}

export function projectCombatHud(screen: string, fixture: MigrationFixtureCombatProjection): CombatHudState {
  const world = fixture.world;
  const ownedQuantity = world.inventory.find((stack) => stack.item_id === FIXTURE_ITEM_ID)?.quantity ?? 0;
  return {
    visible: screen === "field" && fixture.active,
    evidenceLabel: fixture.evidence_label.replaceAll("_", " "),
    tick: world.tick,
    fighting: world.fighting,
    gold: world.gold,
    hunter: entityState(world.hunter),
    monster: entityState(world.monster),
    ownedQuantity,
    inventory: world.inventory.length === 0
      ? t("combat.inventory_empty")
      : world.inventory.map((stack) => t("combat.inventory_item", { id: stack.item_id, quantity: stack.quantity })).join(" | "),
    equipped: world.equipped_item_id === FIXTURE_ITEM_ID,
    equipEligible: screen === "field" && fixture.active && ownedQuantity > 0 && world.equipped_item_id !== FIXTURE_ITEM_ID,
    drops: world.ground_drops.length === 0
      ? t("combat.no_ground_drops")
      : world.ground_drops.map((drop) => t("combat.ground_drop", { id: drop.item_id, quantity: drop.quantity, x: drop.x, y: drop.y })).join(" | "),
  };
}

function entityState(entity: MigrationFixtureCombatProjection["world"]["hunter"]): CombatHudState["hunter"] {
  return {
    hp: entity.hp,
    maxHp: entity.max_hp,
    percent: entity.max_hp > 0 ? Math.max(0, Math.min(100, entity.hp / entity.max_hp * 100)) : 0,
    state: entity.state,
    position: `${entity.x}, ${entity.y}`,
  };
}
