import { describe, expect, it } from "vitest";
import {
  COMBAT_CRITICAL_AMOUNT_OFFSET_Y,
  COMBAT_CRITICAL_LABEL_OFFSET_Y,
  COMBAT_CRITICAL_LABEL_SIZE_PX,
  COMBAT_DAMAGE_FONT_SIZE_PX,
  COMBAT_PRESENTATION_RENDER_SCALE,
  ORIGINAL_DAMAGE_TEXT_IDEAL_DURATION_MS,
  ORIGINAL_DAMAGE_TEXT_TRAVEL_PX,
  ORIGINAL_DODGE_DURATION_MS,
  combatPresentationHasValidPayload,
  combatPresentationText,
  dodgeFrameAt,
  originalDamageMotionAt,
} from "./combat-presentation";

describe("original combat presentation contract", () => {
  it("renders the recovered text layout at the requested half scale", () => {
    expect(COMBAT_PRESENTATION_RENDER_SCALE).toBe(0.5);
    expect(COMBAT_DAMAGE_FONT_SIZE_PX).toBe(16);
    expect(COMBAT_CRITICAL_LABEL_SIZE_PX).toBe(10);
    expect(COMBAT_CRITICAL_LABEL_OFFSET_Y).toBe(-6.5);
    expect(COMBAT_CRITICAL_AMOUNT_OFFSET_Y).toBe(5.5);
  });

  it("formats normal damage and the recovered two-line critical layout", () => {
    expect(combatPresentationText(event("normal_damage", 1234))).toEqual(["1,234"]);
    expect(combatPresentationText(event("incoming_damage", 987))).toEqual(["987"]);
    expect(combatPresentationText(event("critical_damage", 4321))).toEqual(["CRIT", "4,321"]);
    expect(combatPresentationText(event("evade", null))).toEqual(["Evade"]);
    expect(combatPresentationText(event("miss", null))).toEqual(["Miss"]);
  });

  it("replays the packaged symmetric Dodge sprite sequence", () => {
    expect(dodgeFrameAt(0)).toBe(0);
    expect(dodgeFrameAt(ORIGINAL_DODGE_DURATION_MS * 0.5)).toBe(3);
    expect(dodgeFrameAt(ORIGINAL_DODGE_DURATION_MS)).toBe(0);
  });

  it("replays the recovered four-phase DamageManager movement envelope", () => {
    expect(originalDamageMotionAt(0)).toEqual({ yOffset: 0, scale: 1, done: false });
    expect(originalDamageMotionAt(250).yOffset).toBeCloseTo(5);
    expect(originalDamageMotionAt(333.3333333333333).yOffset).toBeCloseTo(15);
    expect(originalDamageMotionAt(395.8333333333333).yOffset).toBeCloseTo(20);

    const completed = originalDamageMotionAt(ORIGINAL_DAMAGE_TEXT_IDEAL_DURATION_MS);
    expect(completed.yOffset).toBeCloseTo(ORIGINAL_DAMAGE_TEXT_TRAVEL_PX);
    expect(completed.scale).toBeCloseTo(0.6180555556);
    expect(completed.done).toBe(true);
  });

  it("fails closed on incompatible amount payloads", () => {
    expect(combatPresentationHasValidPayload("normal_damage", 1)).toBe(true);
    expect(combatPresentationHasValidPayload("incoming_damage", 1)).toBe(true);
    expect(combatPresentationHasValidPayload("critical_damage", null)).toBe(false);
    expect(combatPresentationHasValidPayload("evade", null)).toBe(true);
    expect(combatPresentationHasValidPayload("evade", 0)).toBe(false);
    expect(combatPresentationHasValidPayload("miss", null)).toBe(true);
  });
});

function event(kind: "incoming_damage" | "normal_damage" | "critical_damage" | "evade" | "miss", amount: number | null) {
  return {
    sequence: 1,
    source_entity_id: "source",
    target_entity_id: "target",
    kind,
    amount,
  } as const;
}
