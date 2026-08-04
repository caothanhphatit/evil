export function bindPopupInteractionGuards(
  popups: NodeListOf<HTMLElement>,
  hold: () => void,
  release: () => void,
): void {
  popups.forEach((popup) => {
    popup.addEventListener("pointerdown", hold, true);
    popup.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") hold();
    }, true);
    popup.addEventListener("keyup", (event) => {
      if (event.key === "Enter" || event.key === " ") release();
    }, true);
    popup.addEventListener("focusin", (event) => {
      if ((event.target as HTMLElement).matches("select, input, textarea")) hold();
    }, true);
    popup.addEventListener("focusout", (event) => {
      if ((event.target as HTMLElement).matches("select, input, textarea")) release();
    }, true);
  });
  window.addEventListener("pointerup", release, true);
  window.addEventListener("pointercancel", release, true);
}

export function bindMenuInteraction(
  bottomMenu: HTMLElement,
  handleMenuAction: (button: HTMLButtonElement) => void,
): void {
  bottomMenu.addEventListener("click", (event) => {
    const target = event.target as HTMLElement | null;
    const button = target?.closest<HTMLButtonElement>("button[data-action]");
    if (!button || !bottomMenu.contains(button) || button.disabled) return;
    handleMenuAction(button);
  });
  document.querySelectorAll<HTMLButtonElement>("[data-action]").forEach((button) => {
    if (!button.closest(".bottom-menu")) button.addEventListener("click", () => handleMenuAction(button));
  });
}

export function bindInteractionGuards(): void {
  document.addEventListener("contextmenu", (event) => {
    const target = event.target as HTMLElement | null;
    if (target?.closest("input, textarea, [contenteditable=\"true\"]")) return;
    event.preventDefault();
  });
  document.addEventListener("selectstart", (event) => {
    const target = event.target as HTMLElement | null;
    if (!target?.closest("input, textarea, [contenteditable=\"true\"]")) event.preventDefault();
  });
  document.addEventListener("copy", (event) => {
    const target = event.target as HTMLElement | null;
    if (!target?.closest("input, textarea, [contenteditable=\"true\"]")) event.preventDefault();
  });
  document.addEventListener("cut", (event) => {
    const target = event.target as HTMLElement | null;
    if (!target?.closest("input, textarea, [contenteditable=\"true\"]")) event.preventDefault();
  });
}

export function bindCraftInteractions(options: {
  gearCreateQuantity: HTMLInputElement;
  gearCreateSubmit: HTMLButtonElement;
  gearCreateSell: HTMLButtonElement;
  gearLock: HTMLButtonElement;
  consumCreateQuantityInput: HTMLInputElement;
  consumCreateSubmit: HTMLButtonElement;
  consumMinus: HTMLButtonElement;
  consumPlus: HTMLButtonElement;
  gearDeltaButtons: NodeListOf<HTMLButtonElement>;
  consumDeltaButtons: NodeListOf<HTMLButtonElement>;
  clampQuantity: (value: number | string, min: number, max: number) => number;
  renderGear: () => void;
  renderConsumable: () => void;
  getServiceQuantity: () => number;
  setServiceQuantity: (quantity: number) => void;
  canCraft: () => boolean;
  craftGear: (quantity: number) => boolean;
  craftConsumable: (quantity: number) => boolean;
  sellGear: () => void;
  setGearLocked: () => void;
}): void {
  const changeGearQuantity = (delta: number): void => {
    options.gearCreateQuantity.value = String(options.clampQuantity(Number(options.gearCreateQuantity.value) + delta, 1, 1000));
    options.renderGear();
  };
  options.gearCreateQuantity.addEventListener("input", () => {
    if (options.gearCreateQuantity.value === "") return;
    options.gearCreateQuantity.value = String(options.clampQuantity(options.gearCreateQuantity.value, 1, 1000));
    options.renderGear();
  });
  options.gearCreateQuantity.addEventListener("change", () => {
    options.gearCreateQuantity.value = String(options.clampQuantity(options.gearCreateQuantity.value, 1, 1000));
    options.renderGear();
  });
  options.gearCreateSubmit.addEventListener("click", () => {
    if (options.canCraft() && options.craftGear(Number(options.gearCreateQuantity.value))) options.gearCreateSubmit.disabled = true;
  });
  options.gearCreateSell.addEventListener("click", options.sellGear);
  options.gearLock.addEventListener("click", options.setGearLocked);
  options.gearDeltaButtons.forEach((button) => button.addEventListener("click", () => changeGearQuantity(Number(button.dataset.gearDelta))));

  const changeServiceQuantity = (delta: number): void => {
    options.setServiceQuantity(options.clampQuantity(options.getServiceQuantity() + delta, 1, 1000));
    options.renderConsumable();
  };
  options.consumMinus.addEventListener("click", () => changeServiceQuantity(-1));
  options.consumPlus.addEventListener("click", () => changeServiceQuantity(1));
  options.consumDeltaButtons.forEach((button) => button.addEventListener("click", () => changeServiceQuantity(Number(button.dataset.consumDelta))));
  options.consumCreateQuantityInput.addEventListener("input", () => {
    if (options.consumCreateQuantityInput.value === "") return;
    options.setServiceQuantity(options.clampQuantity(options.consumCreateQuantityInput.value, 1, 1000));
    options.renderConsumable();
  });
  options.consumCreateQuantityInput.addEventListener("change", () => {
    options.setServiceQuantity(options.clampQuantity(options.consumCreateQuantityInput.value, 1, 1000));
    options.renderConsumable();
  });
  options.consumCreateSubmit.addEventListener("click", () => {
    if (options.canCraft() && options.craftConsumable(Number(options.consumCreateQuantityInput.value))) options.consumCreateSubmit.disabled = true;
  });
}

export function bindBuildingControls(options: {
  buildingConstruct: HTMLButtonElement;
  buildingUpgrade: HTMLButtonElement;
  bountyUpgrade: HTMLButtonElement;
  buildingId: () => string | null;
  instanceId: () => string | null;
  construct: (buildingId: string) => void;
  upgrade: (instanceId: string) => void;
  upgradeBounty: (instanceId: string) => void;
}): void {
  options.buildingConstruct.addEventListener("click", () => {
    const id = options.buildingId();
    if (id) options.construct(id);
  });
  options.buildingUpgrade.addEventListener("click", () => {
    const id = options.instanceId();
    if (id) options.upgrade(id);
  });
  options.bountyUpgrade.addEventListener("click", () => {
    const id = options.instanceId();
    if (id) options.upgradeBounty(id);
  });
}

export function bindOverlayInteractions(options: {
  rosterBack: HTMLButtonElement;
  buildingPanelClose: HTMLButtonElement;
  bountyClose: HTMLButtonElement;
  bountyCloseBottom: HTMLButtonElement;
  gearCreateClose: HTMLButtonElement;
  consumCreateClose: HTMLButtonElement;
  closeRoster: () => void;
  closeBuilding: () => void;
  closeBounty: () => void;
  closeGear: () => void;
  closeConsumable: () => void;
}): void {
  bindOverlayCloseControls([
    { overlay: "hunter-roster", controls: [options.rosterBack], close: options.closeRoster },
    { overlay: "building-panel", controls: [options.buildingPanelClose], close: options.closeBuilding },
    { overlay: "bounty-quest", controls: [options.bountyClose, options.bountyCloseBottom], close: options.closeBounty },
    { overlay: "gear-create", controls: [options.gearCreateClose], close: options.closeGear },
    { overlay: "consumable-create", controls: [options.consumCreateClose], close: options.closeConsumable },
  ]);
}
import { bindOverlayCloseControls } from "../ui/overlay-close-controls";
