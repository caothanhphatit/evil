import { describe, expect, it } from "vitest";
import type { WorldEntityProjection } from "../generated/protocol";
import { ProjectionBuffer } from "./projection-interpolation";

describe("protocol-v5 projection interpolation", () => {
  it("orders bounded frames by authoritative visual tick", () => {
    const buffer = new ProjectionBuffer({ maxFrames: 3 });
    buffer.push("village", 2, [entity(20)], 200);
    buffer.push("village", 1, [entity(10)], 100);
    buffer.push("village", 3, [entity(30)], 300);
    buffer.push("village", 4, [entity(40)], 400);
    expect(buffer.bufferedTicks()).toEqual([2, 3, 4]);
  });

  it("interpolates positions on the visual-tick timeline", () => {
    const buffer = new ProjectionBuffer({ tickDurationMs: 200, renderDelayMs: 200 });
    buffer.push("village", 10, [entity(0)], 1000);
    buffer.push("village", 11, [entity(100)], 1200);
    expect(buffer.sample(1300)?.entities[0]?.x).toBe(50);
    expect(buffer.sample(1400)?.entities[0]?.x).toBe(100);
  });

  it("advances walking presentation independently between confirmations", () => {
    const buffer = new ProjectionBuffer({ renderDelayMs: 0, maxExtrapolationTicks: 5 });
    buffer.push("village", 10, [entity(0)], 1000);
    buffer.push("village", 11, [entity(100)], 1100);
    expect(buffer.sample(1100)?.entities[0]?.x).toBe(100);
    expect(buffer.sample(1400)?.entities[0]?.x).toBe(400);
  });

  it("continues walking briefly across a delayed authoritative frame", () => {
    const buffer = new ProjectionBuffer({ renderDelayMs: 100, maxExtrapolationTicks: 1 });
    buffer.push("village", 10, [entity(0)], 1000);
    buffer.push("village", 11, [entity(10)], 1100);
    expect(buffer.sample(1250)?.entities[0]?.x).toBe(15);
    expect(buffer.sample(1400)?.entities[0]?.x).toBe(20);
  });

  it("bounds Hunter dead reckoning when confirmations are delayed", () => {
    const buffer = new ProjectionBuffer({ renderDelayMs: 0, maxExtrapolationTicks: 5 });
    buffer.push("village", 10, [entity(0)], 1000);
    buffer.push("village", 11, [entity(10)], 1100);
    expect(buffer.sample(1600)?.entities[0]?.x).toBe(60);
    expect(buffer.sample(2300)?.entities[0]?.x).toBe(60);
  });

  it("bounds monster patrol dead reckoning when confirmations are delayed", () => {
    const buffer = new ProjectionBuffer({ renderDelayMs: 0, maxExtrapolationTicks: 5 });
    const monster = { ...entity(0), descriptor: { ...entity(0).descriptor, kind: "monster" as const } };
    buffer.push("village", 10, [monster], 1000);
    buffer.push("village", 11, [{ ...monster, x: 10 }], 1100);
    expect(buffer.sample(2300)?.entities[0]?.x).toBe(60);
  });

  it("never predicts idle or combat actors past the server position", () => {
    const buffer = new ProjectionBuffer({ renderDelayMs: 100 });
    buffer.push("village", 10, [entity(0)], 1000);
    buffer.push("village", 11, [{ ...entity(10), action_state: "attacking" }], 1100);
    expect(buffer.sample(1400)?.entities[0]?.x).toBe(10);
  });

  it("does not overshoot and snap backward during steady authoritative movement", () => {
    const buffer = new ProjectionBuffer({ renderDelayMs: 150 });
    buffer.push("village", 10, [entity(0)], 1000);
    buffer.push("village", 11, [entity(10)], 1100);
    const beforeConfirmation = buffer.sample(1190)?.entities[0]?.x ?? 0;
    buffer.push("village", 12, [entity(20)], 1200);
    const afterConfirmation = buffer.sample(1200)?.entities[0]?.x ?? 0;

    expect(beforeConfirmation).toBeLessThanOrEqual(10);
    expect(afterConfirmation).toBeGreaterThanOrEqual(beforeConfirmation);
  });

  it("keeps the client presentation clock monotonic when a confirmation arrives", () => {
    const buffer = new ProjectionBuffer();
    buffer.push("village", 10, [entity(0)], 1000);
    expect(buffer.sample(1000)?.entities[0]?.x).toBe(0);
    buffer.push("village", 11, [entity(10)], 1100);
    expect(buffer.sample(1190)?.entities[0]?.x).toBeCloseTo(19);

    buffer.push("village", 12, [entity(20)], 1200);
    expect(buffer.sample(1200)?.entities[0]?.x).toBe(20);
  });

  it("uses immediate prediction with enough headroom for brief frame delays", () => {
    const buffer = new ProjectionBuffer();
    buffer.push("village", 10, [entity(0)], 1000);
    buffer.push("village", 11, [entity(10)], 1100);

    expect(buffer.sample(1150)?.entities[0]?.x).toBe(15);
    expect(buffer.sample(1500)?.entities[0]?.x).toBe(40);
  });

  it("snaps instead of crossing a teleport or visual-tick gap", () => {
    const teleport = new ProjectionBuffer({ teleportDistance: 100 });
    teleport.push("village", 1, [entity(0)], 100);
    expect(teleport.push("village", 2, [entity(400)], 200)).toBe("snapped");
    expect(teleport.sample(200)?.entities[0]?.x).toBe(400);

    const gap = new ProjectionBuffer({ maxTickGap: 1 });
    gap.push("field", 4, [entity(20)], 400);
    expect(gap.push("field", 6, [entity(80)], 800)).toBe("snapped");
    expect(gap.bufferedTicks()).toEqual([6]);
  });

  it("snaps when the active world mode changes", () => {
    const buffer = new ProjectionBuffer();
    buffer.push("village", 5, [entity(10)], 500);
    expect(buffer.push("field", 5, [entity(700)], 500)).toBe("snapped");
    expect(buffer.sample(500)?.mode).toBe("field");
  });
});

function entity(x: number): WorldEntityProjection {
  return {
    descriptor: {
      entity_id: "hunter-1",
      kind: "hunter",
      asset_bundle_id: "hunter",
      source_skeleton_name: "hunter",
      role: "migration_visual_candidate",
      source_binding: { id: "source", confidence: "confirmed", resolved: true },
      placement_binding: { id: "placement", confidence: "unknown", resolved: false },
    },
    x,
    y: 300,
    facing: "right",
    action_state: "walking",
    animation: "hunter_walk",
    class_family: "H1",
    target_entity_id: null,
    action_sequence: 0,
    loot_sequence: 0,
    loot_label: null,
    attack_effect_key: null,
    skill_presentation_key: null,
    current_hp: 100,
    maximum_hp: 100,
    interaction_prompt_key: null,
    selectable: true,
  };
}
