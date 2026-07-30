import { Assets } from "pixi.js";

export const HUNTER_SKELETON_ALIAS = "hunter:presentation:skeleton";
export const HUNTER_ATLAS_ALIAS = "hunter:presentation:atlas";

const HUNTER_SKELETON_PATH = "/content/releases/visible-world-v1/actors/hunter/hunter.json";
const HUNTER_ATLAS_PATH = "/content/releases/visible-world-v1/actors/hunter/hunter.atlas";

let preloadPromise: Promise<void> | null = null;

export function preloadHunterPresentationAssets(): Promise<void> {
  return preloadPromise ??= (async () => {
    if (!Assets.cache.has(HUNTER_SKELETON_ALIAS)) {
      Assets.add({ alias: HUNTER_SKELETON_ALIAS, src: HUNTER_SKELETON_PATH });
    }
    if (!Assets.cache.has(HUNTER_ATLAS_ALIAS)) {
      Assets.add({ alias: HUNTER_ATLAS_ALIAS, src: HUNTER_ATLAS_PATH });
    }
    await Assets.load([HUNTER_SKELETON_ALIAS, HUNTER_ATLAS_ALIAS]);
  })();
}
