import { node, sourceImage, unavailable } from "./dom";
import type { HunterInfoView } from "./model";
import { t } from "../../i18n";

const EMPTY_PET = "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-hud/rp_hunter_empty__6924.png";

export function renderRidingPetTab(info: HunterInfoView): HTMLElement {
  const root = node("section", "hunter-info-riding-tab");
  if (!info.riding) {
    root.append(sourceImage(EMPTY_PET), unavailable(t("hunter.riding.unsynchronized")));
    const button = node("button", "hunter-ranch-button", t("hunter.riding.move_ranch"));
    button.type = "button";
    button.disabled = true;
    root.append(button);
    return root;
  }
  if (!info.riding.mounted) {
    root.append(sourceImage(EMPTY_PET), node("p", "", t("hunter.riding.none")));
    const button = node("button", "hunter-ranch-button", t("hunter.riding.move_ranch"));
    button.type = "button";
    button.disabled = !info.riding.canMoveToRanch;
    root.append(button);
    return root;
  }
  if (info.riding.icon) root.append(sourceImage(info.riding.icon, info.riding.name ?? t("hunter.riding.mounted_alt")));
  if (info.riding.name) root.append(node("b", "", info.riding.name));
  return root;
}
