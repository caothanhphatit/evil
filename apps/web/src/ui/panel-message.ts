import { t } from "../i18n";

export interface PanelMessageTarget {
  querySelector(selector: "b" | "span"): { textContent: string | null } | null;
}

export function setPanelMessage(target: PanelMessageTarget, title: string, detail: string): void {
  const titleElement = target.querySelector("b");
  const detailElement = target.querySelector("span");
  if (!titleElement || !detailElement) throw new Error(t("error.panel_structure"));
  titleElement.textContent = title;
  detailElement.textContent = detail;
}
