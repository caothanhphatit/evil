const ACTOR_DEPTH_BACK = 487;
const ACTOR_DEPTH_SPAN = 2.5;
const WALKABLE_SURFACE_DEPTH = -485.95;
const BUILDING_DEPTH_FLOOR = WALKABLE_SURFACE_DEPTH + 0.01;
const OCCLUDER_DEPTH_STEP = 0.001;

export interface VillageDepthOccluder {
  x: number;
  y: number;
  halfWidth: number;
  depth: number;
}

// Unity's recovered scene uses smaller Z values for objects closer to the camera,
// while PixiJS draws larger zIndex values last.
export function sceneDepthFromUnityZ(unityZ: number): number {
  return -unityZ;
}

export function scenePieceDepth(id: string | undefined, unityZ: number): number {
  // Bridges must cover the unbroken wall sprite at their connection point.
  // Actor Y-depth still places a Hunter crossing the southern half above them.
  return id?.startsWith("bridge") ? WALKABLE_SURFACE_DEPTH : sceneDepthFromUnityZ(unityZ);
}

export function villageActorDepth(worldY: number, worldSize: number): number {
  const normalizedY = Math.max(0, Math.min(worldY, worldSize)) / worldSize;
  return -ACTOR_DEPTH_BACK + normalizedY * ACTOR_DEPTH_SPAN;
}

export function villageBuildingDepth(worldY: number, worldSize: number): number {
  // Buildings occupy the town side of the boundary and must not be covered by
  // walkable connectors when the temporary town grid overlaps bridge artwork.
  return Math.max(villageActorDepth(worldY, worldSize), BUILDING_DEPTH_FLOOR);
}

export function villageActorDepthWithOccluders(
  worldX: number,
  worldY: number,
  worldSize: number,
  occluders: readonly VillageDepthOccluder[],
): number {
  let depth = villageActorDepth(worldY, worldSize);
  for (const occluder of occluders) {
    if (worldY < occluder.y || Math.abs(worldX - occluder.x) > occluder.halfWidth) continue;
    depth = Math.max(depth, occluder.depth + OCCLUDER_DEPTH_STEP);
  }
  return depth;
}
