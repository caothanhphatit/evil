const ACTOR_DEPTH_BACK = 488.9;
const ACTOR_DEPTH_SPAN = 2.5;

// Unity's recovered scene uses smaller Z values for objects closer to the camera,
// while PixiJS draws larger zIndex values last.
export function sceneDepthFromUnityZ(unityZ: number): number {
  return -unityZ;
}

export function villageActorDepth(worldY: number, worldSize: number): number {
  const normalizedY = Math.max(0, Math.min(worldY, worldSize)) / worldSize;
  return -ACTOR_DEPTH_BACK + normalizedY * ACTOR_DEPTH_SPAN;
}
