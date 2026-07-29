import { describe, expect, it } from "vitest";
import { sceneDepthFromUnityZ, scenePieceDepth, villageActorDepth, villageBuildingDepth } from "./depth";

describe("visible-world depth projection", () => {
  it("inverts recovered Unity Z so foreground pieces render above background tiles", () => {
    expect(sceneDepthFromUnityZ(489)).toBeGreaterThan(sceneDepthFromUnityZ(499));
    expect(sceneDepthFromUnityZ(486)).toBeGreaterThan(sceneDepthFromUnityZ(493));
  });

  it("moves village actors from behind to in front of the wall by world Y", () => {
    expect(villageActorDepth(0, 1000)).toBeGreaterThan(sceneDepthFromUnityZ(499));
    expect(villageActorDepth(0, 1000)).toBeGreaterThan(sceneDepthFromUnityZ(489));
    expect(villageActorDepth(0, 1000)).toBeLessThan(sceneDepthFromUnityZ(486));
    expect(villageActorDepth(1000, 1000)).toBeGreaterThan(sceneDepthFromUnityZ(486));
    expect(villageActorDepth(900, 1000)).toBeGreaterThan(villageActorDepth(100, 1000));
  });

  it("renders bridge connectors above the unbroken wall and below crossing actors", () => {
    const bridge = scenePieceDepth("bridgeC", 487);
    expect(bridge).toBeGreaterThan(sceneDepthFromUnityZ(489));
    expect(bridge).toBeGreaterThan(sceneDepthFromUnityZ(486));
    expect(bridge).toBeLessThan(villageActorDepth(800, 1536));
    expect(scenePieceDepth("wallA", 486)).toBe(sceneDepthFromUnityZ(486));
  });

  it("keeps the north town building row above bridge C when their artwork overlaps", () => {
    const bridge = scenePieceDepth("bridgeC", 487);
    const northRowBuilding = villageBuildingDepth(528, 1536);

    expect(northRowBuilding).toBeGreaterThan(bridge);
    expect(villageBuildingDepth(1200, 1536)).toBeGreaterThan(northRowBuilding);
  });
});
