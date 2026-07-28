import { describe, expect, it } from "vitest";
import type { WorldEntityProjection } from "../generated/protocol";
import {
  ACTOR_HP_EMPTY_COLOR,
  ACTOR_HP_HEALTHY_COLOR,
  ACTOR_HP_MID_COLOR,
  actorHealthBarLayout,
  actorHealthColor,
  actorHealthPresentation,
  actorHealthRatio,
} from "./actor-health-bar";

describe("original actor health-bar presentation", () => {
  it("uses the exact Hunter and monster prefab offsets", () => {
    expect(actorHealthBarLayout("hunter")).toEqual({ y: 6, innerX: -6.5, frameX: 2.5 });
    expect(actorHealthBarLayout("monster")).toEqual({ y: 6, innerX: -9, frameX: 0 });
    expect(actorHealthBarLayout("npc")).toBeNull();
  });

  it("uses the recovered native red, orange, and green thresholds", () => {
    expect(actorHealthColor(0)).toBe(ACTOR_HP_EMPTY_COLOR);
    expect(actorHealthColor(0.199)).toBe(ACTOR_HP_EMPTY_COLOR);
    expect(actorHealthColor(0.2)).toBe(ACTOR_HP_MID_COLOR);
    expect(actorHealthColor(0.499)).toBe(ACTOR_HP_MID_COLOR);
    expect(actorHealthColor(0.5)).toBe(ACTOR_HP_HEALTHY_COLOR);
    expect(actorHealthColor(1)).toBe(ACTOR_HP_HEALTHY_COLOR);
  });

  it("clamps authoritative HP safely and keeps an empty bar at death", () => {
    expect(actorHealthRatio(120, 100)).toBe(1);
    expect(actorHealthRatio(-1, 100)).toBe(0);
    expect(actorHealthPresentation(entity({ current_hp: 0, maximum_hp: 100 }))).toEqual({ ratio: 0, color: ACTOR_HP_EMPTY_COLOR });
  });

  it("does not synthesize a bar without an authoritative maximum", () => {
    expect(actorHealthRatio(10, 0)).toBeNull();
    expect(actorHealthPresentation(entity({ current_hp: null, maximum_hp: null }))).toBeNull();
  });
});

function entity(overrides: Partial<WorldEntityProjection>): WorldEntityProjection {
  return {
    descriptor: {
      entity_id: "monster-1",
      kind: "monster",
      asset_bundle_id: "mon_a_01_1",
      source_skeleton_name: "mon_a_01_1",
      role: "migration_visual_candidate",
      source_binding: { id: "actor", confidence: "confirmed", resolved: true },
      placement_binding: { id: "placement", confidence: "unknown", resolved: false },
    },
    x: 0,
    y: 0,
    facing: "left",
    action_state: "idle",
    animation: "stay",
    class_family: null,
    target_entity_id: null,
    action_sequence: 0,
    attack_effect_key: null,
    skill_presentation_key: null,
    current_hp: 100,
    maximum_hp: 100,
    selectable: true,
    ...overrides,
  };
}
