import { node, sourceImage, unavailable } from "./dom";
import type { HunterInfoView } from "./model";
import { t } from "../../i18n";

export function renderGrowthTab(info: HunterInfoView): HTMLElement {
  const root = node("section", "hunter-info-growth-tab");
  if (!info.growth) {
    root.append(node("header", "", t("hunter.growth.total_points", { points: "—" })));
    const grid = node("div", "hunter-growth-grid unresolved");
    for (let index = 0; index < 15; index += 1) grid.append(node("div", "hunter-growth-node empty"));
    root.append(grid, unavailable(t("hunter.growth.unsynchronized")));
    return root;
  }
  root.append(node("header", "", t("hunter.growth.total_points", { points: info.growth.secretPoints })));
  if (!info.growth.nodes.length) {
    root.append(unavailable(t("hunter.growth.nodes_unavailable")));
    return root;
  }
  const grid = node("div", "hunter-growth-grid");
  for (const growth of info.growth.nodes) {
    const cell = node("div", "hunter-growth-node");
    if (growth.icon) cell.append(sourceImage(growth.icon));
    cell.append(node("b", "", `${growth.points}/${growth.maxPoints}`));
    grid.append(cell);
  }
  root.append(grid);
  return root;
}
