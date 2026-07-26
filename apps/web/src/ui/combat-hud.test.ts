import { describe, expect, it } from "vitest";
import type { MigrationFixtureCombatProjection } from "../generated/protocol";
import { projectCombatHud } from "./combat-hud";

describe("authoritative fixture combat HUD", () => {
  it("projects HP, state, positions, drops, inventory and gold without local outcomes", () => {
    const state = projectCombatHud("field", fixture());
    expect(state.hunter).toMatchObject({ hp: 75, maxHp: 100, percent: 75, state: "attacking", position: "138, 320" });
    expect(state.monster.percent).toBe(50);
    expect(state.gold).toBe(42);
    expect(state.inventory).toBe("Item 2001 x1");
    expect(state.drops).toContain("Item 2001 x1 @ 500,330");
  });

  it("allows equip only while field fixture is active, owned and not already equipped", () => {
    expect(projectCombatHud("field", fixture()).equipEligible).toBe(true);
    const equipped = fixture();
    equipped.world.equipped_item_id = 2001;
    expect(projectCombatHud("field", equipped).equipEligible).toBe(false);
    expect(projectCombatHud("village", fixture()).equipEligible).toBe(false);
    const missing = fixture();
    missing.world.inventory = [];
    expect(projectCombatHud("field", missing).equipEligible).toBe(false);
  });
});

function fixture(): MigrationFixtureCombatProjection {
  return {
    content_id: "migration-fixture.slice1-combat-v1",
    evidence_label: "deterministic_migration_fixture_not_legacy_balance",
    active: true,
    world: {
      tick: 12,
      fighting: true,
      gold: 42,
      hunter: { id: 1, hp: 75, max_hp: 100, alive: true, x: 138, y: 320, state: "attacking" },
      monster: { id: 1001, hp: 25, max_hp: 50, alive: true, x: 632, y: 320, state: "idle" },
      inventory: [{ item_id: 2001, quantity: 1 }],
      equipped_item_id: null,
      ground_drops: [{ drop_id: "drop-1", item_id: 2001, quantity: 1, x: 500, y: 330 }],
      events: [],
    },
  };
}
