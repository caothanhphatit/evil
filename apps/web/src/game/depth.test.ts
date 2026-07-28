import { describe, expect, it } from "vitest";
import { sceneDepthFromUnityZ, scenePieceDepth, villageActorDepth } from "./depth";

describe("visible-world depth projection", () => {
  it("inverts recovered Unity Z so foreground pieces render above background tiles", () => {
    expect(sceneDepthFromUnityZ(489)).toBeGreaterThan(sceneDepthFromUnityZ(499));
    expect(sceneDepthFromUnityZ(486)).toBeGreaterThan(sceneDepthFromUnityZ(493));
  });

  it("keeps village actors above the town ground but behind its front wall", () => {
    expect(villageActorDepth(0, 1000)).toBeGreaterThan(sceneDepthFromUnityZ(499));
    expect(villageActorDepth(0, 1000)).toBeGreaterThan(sceneDepthFromUnityZ(489));
    expect(villageActorDepth(1000, 1000)).toBeLessThan(sceneDepthFromUnityZ(486));
    expect(villageActorDepth(900, 1000)).toBeGreaterThan(villageActorDepth(100, 1000));
  });

  it("renders walkable bridges above ground but below every actor", () => {
    const bridge = scenePieceDepth("bridgeC", 487);
    expect(bridge).toBeGreaterThan(sceneDepthFromUnityZ(489));
    expect(bridge).toBeLessThan(villageActorDepth(0, 1536));
    expect(scenePieceDepth("wallA", 486)).toBe(sceneDepthFromUnityZ(486));
  });
});
