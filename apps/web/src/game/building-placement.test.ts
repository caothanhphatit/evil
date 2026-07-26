import { describe, expect, it } from "vitest";
import { findBuildingInstanceById, footprintsOverlap, gridPointToWorld, isPlacementFree, projectRenderableBuildingInstances, snapWorldPointToGrid } from "./building-placement";

describe("building placement", () => {
  it("snaps world points to the nearest grid cell", () => {
    expect(snapWorldPointToGrid(151, 208, 32, 24, 8, 4)).toEqual({ gridX: 4, gridY: 9 });
    expect(gridPointToWorld({ gridX: 4, gridY: 9 }, 32, 24, 8, 4)).toEqual({ x: 136, y: 220 });
  });

  it("rejects rectangle overlap while allowing edge contact", () => {
    const occupied = { gridX: 2, gridY: 2, width: 3, height: 2 };
    expect(footprintsOverlap({ gridX: 4, gridY: 3, width: 2, height: 2 }, occupied)).toBe(true);
    expect(footprintsOverlap({ gridX: 5, gridY: 2, width: 2, height: 2 }, occupied)).toBe(false);
  });

  it("ignores the moved instance but rejects collisions with every other instance", () => {
    const occupied = [
      { instanceId: "town-hall", gridX: 8, gridY: 8, width: 3, height: 3 },
      { instanceId: "trading-post", gridX: 12, gridY: 8, width: 2, height: 2 },
    ];
    expect(isPlacementFree({ gridX: 8, gridY: 8, width: 3, height: 3 }, occupied, "town-hall")).toBe(true);
    expect(isPlacementFree({ gridX: 10, gridY: 8, width: 3, height: 3 }, occupied, "town-hall")).toBe(false);
  });

  it("projects only authoritative instances, never every available visual", () => {
    const availableVisuals = new Set(Array.from({ length: 79 }, (_, index) => `build_${index + 1}`));
    const instances = [
      { instanceId: "seed-v1:build_1", buildingId: "build_1", spriteAssetId: "build_1", gridX: -1, gridY: -1, width: 3, height: 3 },
      { instanceId: "seed-v1:build_2", buildingId: "build_2", spriteAssetId: "build_2", gridX: -3, gridY: -1, width: 2, height: 2 },
      { instanceId: "seed-v1:build_3", buildingId: "build_3", spriteAssetId: "build_3", gridX: 2, gridY: -1, width: 2, height: 2 },
    ];
    const projected = projectRenderableBuildingInstances(instances, availableVisuals, 92, 54, 500, 500);
    expect(projected.map((instance) => instance.buildingId)).toEqual(["build_1", "build_2", "build_3"]);
    expect(projected).toHaveLength(3);
  });

  it("preserves and selects duplicate base buildings by instance identity", () => {
    const instances = [
      { instance_id: "weapon-shop-a", building_id: "build_10", level: 2 },
      { instance_id: "weapon-shop-b", building_id: "build_10", level: 5 },
    ];
    const projected = projectRenderableBuildingInstances([
      { instanceId: "weapon-shop-a", buildingId: "build_10", spriteAssetId: "build_10", gridX: -2, gridY: 0, width: 2, height: 2 },
      { instanceId: "weapon-shop-b", buildingId: "build_10", spriteAssetId: "build_10", gridX: 2, gridY: 0, width: 2, height: 2 },
    ], new Set(["build_10"]), 32, 54, 500, 350);

    expect(projected.map((instance) => instance.instanceId)).toEqual(["weapon-shop-a", "weapon-shop-b"]);
    expect(findBuildingInstanceById(instances, "weapon-shop-b")?.level).toBe(5);
  });
});
