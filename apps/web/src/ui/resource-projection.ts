import type { OriginalFlowSnapshot } from "../generated/protocol";

export interface ResourceBarState {
  gold: number | null;
  evidenceBacked: boolean;
}

export function projectResourceBar(snapshot: OriginalFlowSnapshot): ResourceBarState {
  const projection = snapshot.migration_fixture_combat;
  if (snapshot.screen === "boot") return { gold: null, evidenceBacked: false };
  return { gold: projection.world.gold, evidenceBacked: true };
}
