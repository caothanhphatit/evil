import { describe, expect, it } from "vitest";
import type { WorldEntityProjection } from "../generated/protocol";
import { projectAuthoritativeMonsterField, projectMonsterField, validateMonsterIntent } from "./monster-field";

describe("monster field projection", () => {
  it("uses visible-world monster families and leaves spawn bounds unresolved", () => {
    const projection = projectMonsterField([entity("mon_goldblin", "gold")], null);
    expect(projection.monsters[0]).toMatchObject({ family: "mon_goldblin", state: "alive", targetable: false });
    expect(projection.spawn).toMatchObject({ current: 1, minimum: null, maximum: null, evidenceState: "fixture_current_only" });
  });

  it("projects combat death, respawn and drops without local outcomes", () => {
    const projection = projectMonsterField([], {
      tick: 3, fighting: false, gold: 0,
      hunter: { id: 1, hp: 1, max_hp: 1, alive: true, x: 0, y: 0, state: "idle" },
      monster: { id: 1001, hp: 0, max_hp: 50, alive: false, x: 0, y: 0, state: "dead" },
      inventory: [], equipped_item_id: null,
      ground_drops: [{ drop_id: "drop", item_id: 1, quantity: 2, x: 1, y: 1 }],
      events: [{ type: "monster_respawned" }],
    });
    expect(projection.monsters[0]).toMatchObject({ state: "dead", targetable: false });
    expect(projection.respawnEvent).toBe(true);
    expect(projection.dropCount).toBe(2);
  });

  it("does not create player targeting intents for monsters", () => {
    const projection = projectMonsterField([entity("mon_a_01_1", "a")], null);
    expect(validateMonsterIntent(projection, "missing", "map_new01")).toBeNull();
    expect(validateMonsterIntent(projection, "monster:a", "bad-map")).toBeNull();
    expect(validateMonsterIntent(projection, "monster:a", "map_new01")).toBeNull();
  });

  it("projects all three in-instance farms and their density counts", () => {
    const projection = projectAuthoritativeMonsterField({
      ruleset: "evil-hunter-1.411-catalog-with-temporary-runtime-tuning",
      tick: 42,
      map_id: "background_11",
      monster_tier: 3,
      map_asset_id: "/content/releases/visible-world-v1/village/background/background_11__1508.png",
      world_difficulty: 0,
      maps: [
        { map_id: "map_new01", monster_tier: 1, map_asset_id: "/field-1.png", density_level: 1 },
        { map_id: "background_08", monster_tier: 2, map_asset_id: "/field-2.png", density_level: 3 },
        { map_id: "background_11", monster_tier: 3, map_asset_id: "/field-3.png", density_level: 2 },
      ],
      density_level: 2,
      spawn_count: 7,
      spawn_min: 3,
      spawn_max: 12,
      cluster_active: false,
      banner_message: null,
      monsters: Array.from({ length: 7 }, (_, index) => ({
        entity_id: `monster-background_11-${index}`,
        monster_id: "mon_a_01_1",
        source_index: 0,
        asset_bundle_id: "mon_a_01_1",
        hp: 20,
        max_hp: 20,
        damage: 4,
        armor: 1,
        experience: 7,
        gold: 3,
        x: 560,
        y: 735,
        action_state: "idle",
        animation: "stay",
        target_hunter_id: null,
        respawn_ticks: null,
      })),
      drops: [],
    });
    expect(projection.selectedMap).toBe("background_11");
    expect(projection.maps.map((map) => map.id)).toEqual(["map_new01", "background_08", "background_11"]);
    expect(projection.farms.map((farm) => ({ id: farm.id, density: farm.densityLevel, count: farm.spawnCount }))).toEqual([
      { id: "map_new01", density: 1, count: 1 },
      { id: "background_08", density: 3, count: 10 },
      { id: "background_11", density: 2, count: 7 },
    ]);
    expect(projection.densityLevel).toBe(2);
    expect(projection.spawn.current).toBe(7);
    expect(projection.monsters[0]).toMatchObject({
      sourceIndex: 0, hp: 20, maxHp: 20, damage: 4, armor: 1, experience: 7, gold: 3,
    });
  });
});

function entity(family: "mon_a_01_1" | "mon_goldblin", id: string): WorldEntityProjection {
  return {
    descriptor: {
      entity_id: `monster:${id}`, kind: "monster", asset_bundle_id: family,
      source_skeleton_name: family, role: "migration_visual_candidate",
      source_binding: { id: family, confidence: "confirmed", resolved: true },
      placement_binding: { id: `${family}:placement`, confidence: "unknown", resolved: false },
    }, x: 200, y: 300, facing: "right", action_state: "idle", animation: "idle", class_family: null,
    target_entity_id: null, action_sequence: 0, loot_sequence: 0, loot_label: null,
    attack_effect_key: null, skill_presentation_key: null,
    current_hp: 100, maximum_hp: 100, selectable: true,
    interaction_prompt_key: null,
  };
}
