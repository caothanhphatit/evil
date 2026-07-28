import { describe, expect, it } from "vitest";
import type { WorldEntityProjection } from "../generated/protocol";
import {
  RANGER_PROJECTILE_DURATION_MS,
  rangerProjectileOrigin,
  rangerProjectilePose,
  shouldStartRangerProjectile,
} from "./ranged-attack-presentation";

describe("ranged attack presentation", () => {
  it("starts only for a new authoritative Ranger attack sequence with a target", () => {
    const ranger = entity({ attack_effect_key: "ranger_basic_arrow", target_entity_id: "monster-1", action_sequence: 4 });
    expect(shouldStartRangerProjectile(3, ranger)).toBe(true);
    expect(shouldStartRangerProjectile(4, ranger)).toBe(false);
    expect(shouldStartRangerProjectile(3, entity({ ...ranger, attack_effect_key: null }))).toBe(false);
    expect(shouldStartRangerProjectile(3, entity({ ...ranger, target_entity_id: null }))).toBe(false);
  });

  it("starts the arrow at the bow side of the authoritative facing", () => {
    expect(rangerProjectileOrigin(entity({ x: 50, y: 80, facing: "left" }))).toEqual({ x: 40, y: 68 });
    expect(rangerProjectileOrigin(entity({ x: 50, y: 80, facing: "right" }))).toEqual({ x: 60, y: 68 });
  });

  it("moves the recovered arrow asset from the Hunter to its target within one attack clip", () => {
    expect(rangerProjectilePose({ x: 10, y: 20 }, { x: 110, y: 70 }, 0)).toMatchObject({ x: 10, y: 20, done: false });
    expect(rangerProjectilePose({ x: 10, y: 20 }, { x: 110, y: 70 }, RANGER_PROJECTILE_DURATION_MS / 2)).toMatchObject({ x: 60, y: 45, done: false });
    expect(rangerProjectilePose({ x: 10, y: 20 }, { x: 110, y: 70 }, RANGER_PROJECTILE_DURATION_MS)).toMatchObject({ x: 110, y: 70, done: true });
  });
});

function entity(overrides: Partial<WorldEntityProjection>): WorldEntityProjection {
  return {
    descriptor: {
      entity_id: "village-hunter-3",
      kind: "hunter",
      asset_bundle_id: "hunter",
      source_skeleton_name: "hunter",
      role: "migration_visual_candidate",
      source_binding: { id: "actor", confidence: "confirmed", resolved: true },
      placement_binding: { id: "placement", confidence: "unknown", resolved: false },
    },
    x: 0,
    y: 0,
    facing: "right",
    action_state: "attacking",
    animation: "h3_hit",
    class_family: "H3",
    target_entity_id: "monster-1",
    action_sequence: 1,
    attack_effect_key: "ranger_basic_arrow",
    skill_presentation_key: null,
    current_hp: 100,
    maximum_hp: 100,
    selectable: true,
    ...overrides,
  };
}
