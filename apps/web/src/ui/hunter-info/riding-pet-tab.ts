import { node, sourceImage, unavailable } from "./dom";
import type { HunterInfoView } from "./model";

const EMPTY_PET = "/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-hud/rp_hunter_empty__6924.png";

export function renderRidingPetTab(info: HunterInfoView): HTMLElement {
  const root = node("section", "hunter-info-riding-tab");
  if (!info.riding) {
    root.append(sourceImage(EMPTY_PET), unavailable("Riding pet data has not been synchronized for this Hunter."));
    const button = node("button", "hunter-ranch-button", "Move to Ranch");
    button.type = "button";
    button.disabled = true;
    root.append(button);
    return root;
  }
  if (!info.riding.mounted) {
    root.append(sourceImage(EMPTY_PET), node("p", "", "No riding pets are being mounted."));
    const button = node("button", "hunter-ranch-button", "Move to Ranch");
    button.type = "button";
    button.disabled = !info.riding.canMoveToRanch;
    root.append(button);
    return root;
  }
  if (info.riding.icon) root.append(sourceImage(info.riding.icon, info.riding.name ?? "Mounted riding pet"));
  if (info.riding.name) root.append(node("b", "", info.riding.name));
  return root;
}
