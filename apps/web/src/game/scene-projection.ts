export const SCENE_PIXELS_PER_UNIT = 100;
export const SCENE_LEFT = 1.74;
export const SCENE_TOP = 16.67;
export const SCENE_WORLD_WIDTH = 3072;
export const SCENE_WORLD_HEIGHT = 1536;
export const TOWN_CAMERA_CLEAR_COLOR = 0x314d79;

export const TOWN_CAMERA_CENTER = { x: 1627, y: 700 } as const;
export const FIELD_CAMERA_CENTER = { x: 1536, y: 1050 } as const;
export const TOWN_CAMERA_ZOOM = 1.45;
export const FIELD_CAMERA_ZOOM = 1.35;
export const TOWN_BUILDING_GRID = { cellWidth: 24, cellHeight: 24, originX: 1627, originY: 600 } as const;

const TOWN_BOUNDS = { left: 1095, top: 330, width: 1064, height: 567 } as const;
const FIELD_BOUNDS = { left: 256, top: 128, width: 2560, height: 1280 } as const;

export function projectScenePoint(x: number, y: number): { x: number; y: number } {
  return {
    x: (x - SCENE_LEFT) * SCENE_PIXELS_PER_UNIT,
    y: (SCENE_TOP - y) * SCENE_PIXELS_PER_UNIT,
  };
}

export function projectNormalizedEntityPoint(
  mode: "village" | "field",
  x: number,
  y: number,
): { x: number; y: number } {
  const bounds = mode === "village" ? TOWN_BOUNDS : FIELD_BOUNDS;
  const normalizedX = Math.max(0, Math.min(1000, x)) / 1000;
  const normalizedY = Math.max(0, Math.min(1000, y)) / 1000;
  return {
    x: bounds.left + normalizedX * bounds.width,
    y: bounds.top + normalizedY * bounds.height,
  };
}

export function runtimeScenePieces<T extends { id?: string }>(pieces: T[]): T[] {
  // The recovered skull gate has no confirmed attachment and otherwise floats over open water.
  return pieces.filter((piece) => piece.id !== "gate");
}
