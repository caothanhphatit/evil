export const ENHANCEMENT_FORGE_BUILDING_ID = "build_15" as const;

/**
 * This projection deliberately mirrors only server-authored workflow state.
 * World coordinates remain a Pixi concern and are attached when the icon is rendered.
 */
export interface HunterEnhancementAuthoritySnapshot {
  hunterEntityId: string;
  workflow: "gear_enhancement" | null;
  phase: "traveling" | "waiting_for_interaction" | null;
  buildingId: string | null;
  buildingInstanceId: string | null;
}

export type HunterEnhancementInteractionState =
  | { mode: "hidden" }
  | {
    mode: "traveling";
    hunterEntityId: string;
    buildingId: typeof ENHANCEMENT_FORGE_BUILDING_ID;
    buildingInstanceId: string;
  }
  | {
    mode: "ready";
    hunterEntityId: string;
    buildingId: typeof ENHANCEMENT_FORGE_BUILDING_ID;
    buildingInstanceId: string;
  };

export interface OpenHunterEnhancementIntent {
  type: "open_hunter_gear_enhancement";
  hunterEntityId: string;
  buildingInstanceId: string;
}

export function projectHunterEnhancementInteraction(
  snapshot: HunterEnhancementAuthoritySnapshot | null,
): HunterEnhancementInteractionState {
  if (
    snapshot?.workflow !== "gear_enhancement"
    || snapshot.buildingId !== ENHANCEMENT_FORGE_BUILDING_ID
    || !snapshot.buildingInstanceId
  ) {
    return { mode: "hidden" };
  }
  if (snapshot.phase === "traveling") {
    return {
      mode: "traveling",
      hunterEntityId: snapshot.hunterEntityId,
      buildingId: ENHANCEMENT_FORGE_BUILDING_ID,
      buildingInstanceId: snapshot.buildingInstanceId,
    };
  }
  if (snapshot.phase !== "waiting_for_interaction") return { mode: "hidden" };
  return {
    mode: "ready",
    hunterEntityId: snapshot.hunterEntityId,
    buildingId: ENHANCEMENT_FORGE_BUILDING_ID,
    buildingInstanceId: snapshot.buildingInstanceId,
  };
}

export function createOpenHunterEnhancementIntent(
  state: HunterEnhancementInteractionState,
): OpenHunterEnhancementIntent | null {
  if (state.mode !== "ready") return null;
  return {
    type: "open_hunter_gear_enhancement",
    hunterEntityId: state.hunterEntityId,
    buildingInstanceId: state.buildingInstanceId,
  };
}
