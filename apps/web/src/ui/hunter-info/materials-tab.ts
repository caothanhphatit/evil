import { node, sourceImage, unavailable } from "./dom";
import type { HunterInfoView } from "./model";

export function renderMaterialsTab(info: HunterInfoView): HTMLElement {
  const root = node("section", "hunter-info-materials-tab");
  root.append(node("h3", "", "Material"));
  if (info.materials === null) {
    const grid = node("div", "hunter-material-grid unresolved");
    for (let index = 0; index < 12; index += 1) grid.append(node("div", "hunter-material-cell empty"));
    root.append(grid, unavailable("This Hunter's material inventory has not been synchronized."));
    return root;
  }
  if (!info.materials.length) {
    root.append(node("p", "hunter-info-empty", "No carried materials."));
    return root;
  }
  const grid = node("div", "hunter-material-grid");
  for (const material of info.materials) {
    const cell = node("div", "hunter-material-cell");
    cell.title = material.name ?? material.id;
    cell.append(sourceImage(material.icon, material.name ?? ""), node("b", "", String(material.quantity)));
    grid.append(cell);
  }
  root.append(grid);
  return root;
}
