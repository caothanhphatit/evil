export interface GridPosition {
  gridX: number;
  gridY: number;
}

export interface GridFootprint extends GridPosition {
  width: number;
  height: number;
}

export interface PositionedFootprint extends GridFootprint {
  instanceId: string;
}

export interface BuildingInstancePlacement extends PositionedFootprint {
  buildingId: string;
  spriteAssetId: string | null;
}

export interface RenderedBuildingPlacement extends BuildingInstancePlacement {
  x: number;
  y: number;
}

export function findBuildingInstanceById<T extends { instance_id: string }>(
  instances: T[],
  instanceId: string | null,
): T | null {
  if (!instanceId) return null;
  return instances.find((instance) => instance.instance_id === instanceId) ?? null;
}

export function snapWorldPointToGrid(
  worldX: number,
  worldY: number,
  cellWidth: number,
  cellHeight: number,
  originX = 0,
  originY = 0,
): GridPosition {
  return {
    gridX: Math.round((worldX - originX) / cellWidth),
    gridY: Math.round((worldY - originY) / cellHeight),
  };
}

export function gridPointToWorld(
  position: GridPosition,
  cellWidth: number,
  cellHeight: number,
  originX = 0,
  originY = 0,
): { x: number; y: number } {
  return {
    x: originX + position.gridX * cellWidth,
    y: originY + position.gridY * cellHeight,
  };
}

export function footprintsOverlap(left: GridFootprint, right: GridFootprint): boolean {
  return left.gridX < right.gridX + right.width
    && left.gridX + left.width > right.gridX
    && left.gridY < right.gridY + right.height
    && left.gridY + left.height > right.gridY;
}

export function isPlacementFree(
  candidate: GridFootprint,
  occupied: PositionedFootprint[],
  ignoredInstanceId: string | null = null,
): boolean {
  return occupied.every((footprint) => footprint.instanceId === ignoredInstanceId || !footprintsOverlap(candidate, footprint));
}

export function isInsideGrid(footprint: GridFootprint, minimum: number, maximum: number): boolean {
  return footprint.gridX >= minimum && footprint.gridY >= minimum
    && footprint.gridX + footprint.width <= maximum
    && footprint.gridY + footprint.height <= maximum;
}

export function projectRenderableBuildingInstances(
  instances: BuildingInstancePlacement[],
  resolvedVisualIds: ReadonlySet<string>,
  cellWidth: number,
  cellHeight: number,
  originX: number,
  originY: number,
): RenderedBuildingPlacement[] {
  return instances.flatMap((instance) => {
    if (!instance.spriteAssetId || !resolvedVisualIds.has(instance.spriteAssetId)) return [];
    return [{
      ...instance,
      x: originX + (instance.gridX + instance.width / 2) * cellWidth,
      y: originY + (instance.gridY + instance.height) * cellHeight,
    }];
  });
}
