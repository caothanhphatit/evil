import type { WorldEntityKind, WorldEntityProjection } from "../generated/protocol";

export const ACTOR_HP_INNER_ASSET = "/content/releases/original-flow-v1/sprites/hp_in__5625.png";
export const ACTOR_HP_FRAME_ASSET = "/content/releases/original-flow-v1/sprites/hp_bg__7393.png";

export const ACTOR_HP_EMPTY_COLOR = 0xe73020;
export const ACTOR_HP_MID_COLOR = 0xe76620;
export const ACTOR_HP_HEALTHY_COLOR = 0x66e720;
export const ACTOR_HP_BACKGROUND_COLOR = 0x484848;

export interface ActorHealthBarLayout {
  y: number;
  innerX: number;
  frameX: number;
}

const LAYOUT_BY_KIND: Partial<Record<WorldEntityKind, ActorHealthBarLayout>> = {
  hunter: { y: 6, innerX: -6.5, frameX: 2.5 },
  monster: { y: 6, innerX: -9, frameX: 0 },
};

export function actorHealthBarLayout(kind: WorldEntityKind): ActorHealthBarLayout | null {
  return LAYOUT_BY_KIND[kind] ?? null;
}

export function actorHealthRatio(currentHp: number | null, maximumHp: number | null): number | null {
  if (currentHp === null || maximumHp === null || !Number.isFinite(currentHp) || !Number.isFinite(maximumHp) || maximumHp <= 0) return null;
  return Math.max(0, Math.min(1, currentHp / maximumHp));
}

export function actorHealthColor(ratio: number): number {
  if (ratio >= 0.5) return ACTOR_HP_HEALTHY_COLOR;
  if (ratio >= 0.2) return ACTOR_HP_MID_COLOR;
  return ACTOR_HP_EMPTY_COLOR;
}

export function actorHealthPresentation(entity: WorldEntityProjection): { ratio: number; color: number } | null {
  if (!actorHealthBarLayout(entity.descriptor.kind)) return null;
  const ratio = actorHealthRatio(entity.current_hp, entity.maximum_hp);
  return ratio === null ? null : { ratio, color: actorHealthColor(ratio) };
}
