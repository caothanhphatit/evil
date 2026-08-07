import type { Spine } from "@esotericsoftware/spine-pixi-v8";
import type { WorldEntityProjection } from "../generated/protocol";
import { hunterActorVisual } from "./hunter-actor-presentation";
import { applyHunterSpineSkin, removeHunterPaperDollEffects } from "./hunter-spine-presentation";

export function prepareHunterPaperDoll(
  spine: Spine,
  entity: WorldEntityProjection | Record<string, unknown>,
  classFamily: string | null,
  compositionName: string,
): void {
  const visual = hunterActorVisual(entity);
  applyHunterSpineSkin(spine, visual.skinNames, classFamily, compositionName);
  if (spine.skeleton.data.findAnimation("hunter_stay")) spine.state.setAnimation(0, "hunter_stay", true);
  removeHunterPaperDollEffects(spine);
  spine.tint = visual.tint;
}
