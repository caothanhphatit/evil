import { describe, expect, it } from "vitest";
import {
  projectNormalizedEntityPoint,
  projectScenePoint,
  projectWorldEntityPoint,
  runtimeScenePieces,
  SCENE_WORLD_HEIGHT,
  SCENE_WORLD_WIDTH,
  TOWN_BUILDING_GRID,
  TOWN_CAMERA_CLEAR_COLOR,
  TOWN_CAMERA_CENTER,
  TOWN_CAMERA_ZOOM,
} from "./scene-projection";

describe("recovered level1 scene projection", () => {
  it("projects the 6 by 3 background tile centers into a 3072 by 1536 world", () => {
    const topLeft = projectScenePoint(4.3, 14.11);
    const bottomRight = projectScenePoint(29.9, 3.87);
    expect(topLeft.x).toBeCloseTo(256);
    expect(topLeft.y).toBeCloseTo(256);
    expect(bottomRight.x).toBeCloseTo(2816);
    expect(bottomRight.y).toBeCloseTo(1280);
    expect({ width: SCENE_WORLD_WIDTH, height: SCENE_WORLD_HEIGHT }).toEqual({ width: 3072, height: 1536 });
  });

  it("keeps village projections on the recovered town ground", () => {
    expect(projectNormalizedEntityPoint("village", 0, 0)).toEqual({ x: 1095, y: 330 });
    expect(projectNormalizedEntityPoint("village", 1000, 1000)).toEqual({ x: 2159, y: 897 });
  });

  it("uses the surrounding recovered field for field projections", () => {
    expect(projectNormalizedEntityPoint("field", 0, 0)).toEqual({ x: 256, y: 128 });
    expect(projectNormalizedEntityPoint("field", 1000, 1000)).toEqual({ x: 2816, y: 1408 });
  });

  it("keeps authoritative world entities in the shared scene coordinate space", () => {
    expect(projectWorldEntityPoint(320, 500)).toEqual({ x: 320, y: 500 });
    expect(projectWorldEntityPoint(2860, 1030)).toEqual({ x: 2860, y: 1030 });
  });

  it("keeps the default building grid compact and excludes the unattached skull gate", () => {
    expect(TOWN_BUILDING_GRID).toEqual({ cellWidth: 24, cellHeight: 24, originX: 1627, originY: 600 });
    expect(runtimeScenePieces([{ id: "ground" }, { id: "gate" }, { id: "bridgeA" }])).toEqual([
      { id: "ground" },
      { id: "bridgeA" },
    ]);
  });

  it("uses the close original-game town framing instead of fitting the whole scene", () => {
    expect(TOWN_CAMERA_CENTER).toEqual({ x: 1627, y: 700 });
    expect(TOWN_CAMERA_ZOOM).toBe(1.45);
    expect(TOWN_CAMERA_CLEAR_COLOR).toBe(0x314d79);
  });
});
