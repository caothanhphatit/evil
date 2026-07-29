import { node, sourceImage, unavailable } from "./dom";
import type { HunterInfoView } from "./model";

const UNRESOLVED_PLACEHOLDER_COUNT = 12;

export function renderMaterialsTab(info: HunterInfoView): HTMLElement {
  const root = node("section", "hunter-info-materials-tab");
  root.append(node("h3", "", "Material"));
  const grid = node("div", `hunter-material-grid${info.materials === null ? " unresolved" : ""}`);
  if (info.materials === null) {
    appendEmptySlots(grid, UNRESOLVED_PLACEHOLDER_COUNT);
    root.append(grid, unavailable("This Hunter's material inventory has not been synchronized."));
    return root;
  }
  for (const item of info.materials) {
    const cell = node("div", "hunter-material-cell");
    cell.title = item.name ?? item.id;
    cell.append(sourceImage(item.icon, item.name ?? item.id), node("b", "", String(item.quantity)));
    grid.append(cell);
  }
  root.append(grid);
  return root;
}

function appendEmptySlots(grid: HTMLElement, count: number): void {
  for (let index = 0; index < count; index += 1) {
    grid.append(node("div", "hunter-material-cell empty"));
  }
}
