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

  it("snaps instead of crossing a teleport or visual-tick gap", () => {
    const teleport = new ProjectionBuffer({ teleportDistance: 100 });
    teleport.push("village", 1, [entity(0)], 100);
    expect(teleport.push("village", 2, [entity(400)], 200)).toBe("snapped");
    expect(teleport.sample(200)?.entities[0]?.x).toBe(400);

    const gap = new ProjectionBuffer();
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
    selectable: true,
  };
}
