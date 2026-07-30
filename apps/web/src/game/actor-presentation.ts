import type { WorldEntityProjection } from "../generated/protocol";

export function actorScaleForFamily(family: string): number {
  const scales: Record<string, number> = {
    hunter: 1.02,
    Chief: 1.08,
    Npc: 0.80,
    npc_animal: 0.68,
    pet: 0.58,
    mon_goldblin: 1.15,
    mon_a_01_1: 1.15,
  };
  return scales[family] ?? 0.72;
}

export function facingScale(family: string, entity: WorldEntityProjection): number {
  const left = entity.facing === "left";
  // Recovered Spine setup poses face left; hunters and monsters mirror the same way.
  return family === "hunter" || entity.descriptor.kind === "monster"
    ? (left ? 1 : -1)
    : (left ? -1 : 1);
}
