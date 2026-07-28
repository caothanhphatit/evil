import type { WorldEntityProjection } from "../generated/protocol";

// The recovered H3 basic attack clip is 0.3333 seconds. The exact native
// projectile hit frame remains unresolved, so the rebuild uses that clip as
// the presentation envelope while the server remains authoritative for damage.
export const RANGER_PROJECTILE_DURATION_MS = 333.3;
export const RANGER_PROJECTILE_SCALE = 0.8;

export interface ProjectilePose {
  x: number;
  y: number;
  rotation: number;
  done: boolean;
}

export function shouldStartRangerProjectile(
  previousSequence: number,
  entity: WorldEntityProjection,
): boolean {
  return entity.attack_effect_key === "ranger_basic_arrow"
    && entity.action_state === "attacking"
    && entity.target_entity_id !== null
    && entity.action_sequence > previousSequence;
}

export function rangerProjectileOrigin(entity: WorldEntityProjection): { x: number; y: number } {
  return {
    x: entity.x + (entity.facing === "left" ? -10 : 10),
    y: entity.y - 12,
  };
}

export function rangerProjectilePose(
  start: { x: number; y: number },
  end: { x: number; y: number },
  elapsedMs: number,
): ProjectilePose {
  const progress = Math.max(0, Math.min(1, elapsedMs / RANGER_PROJECTILE_DURATION_MS));
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  return {
    x: start.x + dx * progress,
    y: start.y + dy * progress,
    // The recovered 24px sprite points down-left in setup pose.
    rotation: Math.atan2(dy, dx) + (Math.PI * 3) / 4,
    done: progress >= 1,
  };
}
