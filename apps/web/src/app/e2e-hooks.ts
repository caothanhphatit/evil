import type { OriginalFlowSnapshot } from "../generated/protocol";

export interface GameE2eHooks {
  openBuilding(buildingId: string): boolean;
  openHunterInfo(hunterId: number): boolean;
  snapshot(): OriginalFlowSnapshot | null;
}

export function installGameE2eHooks(
  locationLike: Pick<Location, "hostname" | "search">,
  hooks: GameE2eHooks,
): void {
  if (!isLocalE2eEnvironment(locationLike)) return;
  window.__EVIL_HUNTER_E2E__ = hooks;
}

export function isLocalE2eEnvironment(locationLike: Pick<Location, "hostname" | "search">): boolean {
  const localHost = locationLike.hostname === "localhost" || locationLike.hostname === "127.0.0.1";
  return localHost && new URLSearchParams(locationLike.search).get("e2e") === "1";
}

declare global {
  interface Window {
    __EVIL_HUNTER_E2E__?: GameE2eHooks;
  }
}
