import type { CombatPresentationKind, CombatPresentationSnapshot } from "../generated/protocol";

export const ORIGINAL_DAMAGE_FONT_FAMILY = "EvilHunterDamage";
export const ORIGINAL_DAMAGE_FONT_SIZE_PX = 32;
export const ORIGINAL_CRITICAL_LABEL_SIZE_PX = 20;
export const ORIGINAL_DAMAGE_PREFAB_WIDTH_PX = 50;
export const ORIGINAL_DAMAGE_PREFAB_HEIGHT_PX = 20;
export const COMBAT_PRESENTATION_RENDER_SCALE = 0.5;
export const COMBAT_DAMAGE_FONT_SIZE_PX = ORIGINAL_DAMAGE_FONT_SIZE_PX * COMBAT_PRESENTATION_RENDER_SCALE;
export const COMBAT_CRITICAL_LABEL_SIZE_PX = ORIGINAL_CRITICAL_LABEL_SIZE_PX * COMBAT_PRESENTATION_RENDER_SCALE;
export const COMBAT_CRITICAL_LABEL_OFFSET_Y = -13 * COMBAT_PRESENTATION_RENDER_SCALE;
export const COMBAT_CRITICAL_AMOUNT_OFFSET_Y = 11 * COMBAT_PRESENTATION_RENDER_SCALE;
export const ORIGINAL_NORMAL_DAMAGE_COLOR = 0xaf70e0;
export const ORIGINAL_INCOMING_DAMAGE_COLOR = 0xde3232;
export const ORIGINAL_CRITICAL_LABEL_COLOR = 0xffd228;
export const ORIGINAL_EVADE_COLOR = 0x81f7f3;
export const ORIGINAL_MISS_COLOR = 0xd43d3d;
// EXP has no recovered color binding yet; this is rebuild-only presentation tuning.
export const REBUILD_EXPERIENCE_COLOR = 0xf4df67;

// The packaged Dodge clip runs from 0 through 1.0166666507720947 seconds and
// swaps these four sprites in a symmetric seven-key sequence.
export const ORIGINAL_DODGE_DURATION_MS = 1016.6666507720947;
export const ORIGINAL_DODGE_FRAME_SEQUENCE = [0, 1, 2, 3, 2, 1, 0] as const;

const ORIGINAL_DAMAGE_MOTION_PHASES = [
  { distance: 5, speed: 20 },
  { distance: 10, speed: 120 },
  { distance: 5, speed: 80 },
  { distance: 15, speed: 20 },
] as const;

export const ORIGINAL_DAMAGE_TEXT_TRAVEL_PX = 35;
export const ORIGINAL_DAMAGE_TEXT_IDEAL_DURATION_MS = ORIGINAL_DAMAGE_MOTION_PHASES.reduce(
  (durationMs, phase) => durationMs + (phase.distance / phase.speed) * 1000,
  0,
);

export interface OriginalDamageMotion {
  yOffset: number;
  scale: number;
  done: boolean;
}

// The original coroutine is FixedUpdate-quantized. The browser interpolates the
// same recovered phase envelope continuously because its render cadence differs.
export function originalDamageMotionAt(elapsedMs: number): OriginalDamageMotion {
  const clampedMs = Math.max(0, Math.min(elapsedMs, ORIGINAL_DAMAGE_TEXT_IDEAL_DURATION_MS));
  let remainingSeconds = clampedMs / 1000;
  let yOffset = 0;
  for (const phase of ORIGINAL_DAMAGE_MOTION_PHASES) {
    const phaseDuration = phase.distance / phase.speed;
    const elapsedInPhase = Math.min(remainingSeconds, phaseDuration);
    yOffset += elapsedInPhase * phase.speed;
    remainingSeconds -= elapsedInPhase;
    if (remainingSeconds <= 0) break;
  }
  return {
    yOffset,
    scale: 1 - clampedMs / 3000,
    done: elapsedMs >= ORIGINAL_DAMAGE_TEXT_IDEAL_DURATION_MS,
  };
}

export function combatPresentationText(event: CombatPresentationSnapshot): string[] {
  if (event.kind === "evade") return ["Evade"];
  if (event.kind === "miss") return ["Miss"];
  const amount = event.amount === null ? "" : event.amount.toLocaleString("en-US");
  if (event.kind === "experience") return [`+${amount} EXP`];
  return event.kind === "critical_damage" ? ["CRIT", amount] : [amount];
}

export function dodgeFrameAt(elapsedMs: number): number {
  if (elapsedMs <= 0) return ORIGINAL_DODGE_FRAME_SEQUENCE[0];
  if (elapsedMs >= ORIGINAL_DODGE_DURATION_MS) return ORIGINAL_DODGE_FRAME_SEQUENCE.at(-1)!;
  const key = Math.min(
    ORIGINAL_DODGE_FRAME_SEQUENCE.length - 1,
    Math.floor(elapsedMs / (ORIGINAL_DODGE_DURATION_MS / ORIGINAL_DODGE_FRAME_SEQUENCE.length)),
  );
  return ORIGINAL_DODGE_FRAME_SEQUENCE[key]!;
}

export function combatPresentationHasValidPayload(kind: CombatPresentationKind, amount: number | null): boolean {
  return kind === "evade" || kind === "miss"
    ? amount === null
    : amount !== null && Number.isSafeInteger(amount) && amount >= 0;
}
