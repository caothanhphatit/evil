export interface WorldViewportTransform {
  scale: number;
  x: number;
  y: number;
}

export function panWorldViewport(
  width: number,
  height: number,
  worldWidth: number,
  worldHeight: number,
  centerX: number,
  centerY: number,
  zoom = 1,
): WorldViewportTransform {
  const safeWidth = Math.max(1, width);
  const safeHeight = Math.max(1, height);
  const scale = Math.max(safeWidth / worldWidth, safeHeight / worldHeight) * Math.max(1, zoom);
  const visibleWidth = safeWidth / scale;
  const visibleHeight = safeHeight / scale;
  const clampedX = Math.max(visibleWidth / 2, Math.min(worldWidth - visibleWidth / 2, centerX));
  const clampedY = Math.max(visibleHeight / 2, Math.min(worldHeight - visibleHeight / 2, centerY));
  return { scale, x: safeWidth / 2 - clampedX * scale, y: safeHeight / 2 - clampedY * scale };
}

export function fitWorldViewport(width: number, height: number, worldSize: number): WorldViewportTransform {
  const safeWidth = Math.max(1, width);
  const safeHeight = Math.max(1, height);
  // The original game uses a portrait camera that fills the device and crops
  // the wider world horizontally instead of exposing empty space above/below.
  const scale = Math.max(safeWidth / worldSize, safeHeight / worldSize);
  return {
    scale,
    x: (safeWidth - worldSize * scale) / 2,
    y: (safeHeight - worldSize * scale) / 2,
  };
}
