import type { Facing } from "../types";
import type { LegacyEntityState } from "../types";

export type ActorType = "hunter" | "monster";

const HUNTER_ANIMATIONS = {
  idle: ["hunter_stay", "hunter_stay_back"],
  moving: ["hunter_walk", "hunter_walk_back"],
  attacking: ["h1_hit", "h1_hit_back"],
  dead: ["hunter_die", "hunter_die"],
  reviving: ["hunter_dying", "hunter_dying"],
} as const;

const MONSTER_ANIMATIONS = {
  idle: ["stay", "stay_b"],
  moving: ["walk", "walk_b"],
  attacking: ["atk", "atk_b"],
  dead: ["die", "die"],
  reviving: ["dying", "dying"],
} as const;

export function animationFor(actor: ActorType, state: LegacyEntityState, facing: Facing): string {
  const pair = actor === "hunter" ? HUNTER_ANIMATIONS[state] : MONSTER_ANIMATIONS[state];
  return pair[facing === "back" ? 1 : 0];
}

export function animationLoops(state: LegacyEntityState): boolean {
  return state === "idle" || state === "moving" || state === "reviving";
}
