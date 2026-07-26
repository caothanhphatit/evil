import { describe, expect, it } from "vitest";
import { hunterActorVisual, hunterPercent, projectHunterRoster } from "./hunter-roster";

describe("projectHunterRoster", () => {
  it("projects active slots, waiting order, class, traits, skills and action data", () => {
    const view = projectHunterRoster({
      hunter_roster: {
        active_capacity: 8,
        active_hunters: [{ id: 7, name: "Rin", level: 12, class: { id: "berserker", name: "Berserker", family: "h1" }, trait: { name: "Swift" }, stats: { hp: 75, max_hp: 100, attack: 22 }, action_state: { kind: "farming", animation: "hunter_walk" }, skills: [{ id: "slash", name: "Slash", level: 2 }] }],
        waiting_queue: [{ hunter_id: 9, display_name: "Mina", queue_position: 1 }],
      },
      world: { entities: [] },
    }, null);
    expect(view.capacity).toBe(8);
    expect(view.active[0]).toMatchObject({ id: "hunter-7", name: "Rin", classFamily: "H1", traitName: "Swift", action: "farming", hp: 75, attack: 22 });
    expect(view.active[0].skills[0]).toMatchObject({ id: "slash", name: "Slash", level: 2 });
    expect(view.waiting[0]).toMatchObject({ id: "hunter-9", queuePosition: 1 });
    expect(view.selectedId).toBe("hunter-7");
  });

  it("keeps protocol v14 usable by deriving visible hunters without inventing stats", () => {
    const view = projectHunterRoster({
      hunter_roster: { infirmary: { hunters: [] }, product_services: [] },
      world: { entities: [{ descriptor: { entity_id: "hunter-3", kind: "hunter" }, animation: "hunter_stay" }] },
    });
    expect(view.capacity).toBe(8);
    expect(view.active[0]).toMatchObject({ id: "hunter-3", numericId: 3, animation: "hunter_stay", hp: null });
    expect(view.resolved).toBe(false);
  });

  it("reports an invalid server projection that exceeds town capacity", () => {
    const active_hunters = Array.from({ length: 9 }, (_, index) => ({ id: index + 1 }));
    expect(projectHunterRoster({ hunter_roster: { capacity: 8, active_hunters }, world: {} }).constraintViolation).toBe("Town capacity exceeded: 9/8");
  });

  it("reads the durable Hunter profile shape without flattening it first", () => {
    const view = projectHunterRoster({
      hunter_roster: {
        active_capacity: 8,
        active_hunters: [{
          hunter_id: 4,
          current_hp: 88,
          max_hp: 120,
          profile: {
            display_name: "Kara",
            class_id: "h3",
            class_name: "Sorcerer",
            visual_family: "H3",
            level: 17,
            attack: 44,
            defense: 19,
            action_state: "walking",
            animation_name: "hunter_walk",
            traits: [{ trait_id: "swift", display_name: "Swift", unlocked_rank: 2, equipped: true }],
            skills: [{ skill_id: "arcane", display_name: "Arcane", skill_level: 3, animation_name: "h3_hit_arcane", ready: true }],
          },
        }],
      },
      world: { entities: [] },
    });
    expect(view.active[0]).toMatchObject({ name: "Kara", classId: "h3", className: "Sorcerer", classFamily: "H3", level: 17, attack: 44, defense: 19, action: "walking", animation: "hunter_walk", traitName: "Swift" });
    expect(view.active[0].traits[0]).toMatchObject({ id: "swift", name: "Swift", rank: 2, equipped: true });
    expect(view.active[0].skills[0]).toMatchObject({ id: "arcane", name: "Arcane", level: 3, ready: true });
  });
});

describe("hunter helpers", () => {
  it("clamps gauges and resolves class-family Spine composition", () => {
    expect(hunterPercent(150, 100)).toBe(100);
    expect(hunterPercent(null, 100)).toBeNull();
    expect(hunterActorVisual({ hunter_id: 1, class_family: "h4", hunter_visual: { weapon_skin: "weapon_h4_a_01" } })).toEqual({
      skinNames: ["All_h4", "weapon_h4_a_01"],
      animation: null,
      tint: 0xffffff,
      signature: "All_h4|weapon_h4_a_01:ffffff",
    });
  });

  it("gives the eight demo Hunters distinct confirmed aggregate compositions", () => {
    const visuals = Array.from({ length: 8 }, (_, index) => hunterActorVisual({ descriptor: { entity_id: `hunter-${index + 1}` }, animation: "hunter_stay" }));
    expect(new Set(visuals.map((visual) => visual.skinNames[0])).size).toBe(8);
    expect(new Set(visuals.map((visual) => visual.signature)).size).toBe(8);
  });
});
