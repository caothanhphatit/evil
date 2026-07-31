export const PRIMARY_OVERLAY_CLOSE_CONTRACTS = [
  { overlay: "hunter-roster", controls: ["roster-back"] },
  { overlay: "building-panel", controls: ["building-panel-close"] },
  { overlay: "bounty-quest", controls: ["bounty-close", "bounty-close-bottom"] },
  { overlay: "gear-create", controls: ["gear-create-close"] },
  { overlay: "consumable-create", controls: ["consum-create-close"] },
] as const;

type CloseControl = Pick<EventTarget, "addEventListener">;

export interface OverlayCloseBinding {
  overlay: typeof PRIMARY_OVERLAY_CLOSE_CONTRACTS[number]["overlay"];
  controls: readonly CloseControl[];
  close: () => void;
}

export function bindOverlayCloseControls(bindings: readonly OverlayCloseBinding[]): void {
  const expected = new Set(PRIMARY_OVERLAY_CLOSE_CONTRACTS.map((contract) => contract.overlay));
  for (const binding of bindings) {
    if (!expected.delete(binding.overlay)) throw new Error(`Unknown or duplicate overlay close binding: ${binding.overlay}`);
    if (binding.controls.length === 0) throw new Error(`Overlay has no close control: ${binding.overlay}`);
    for (const control of binding.controls) control.addEventListener("click", binding.close);
  }
  if (expected.size > 0) throw new Error(`Missing overlay close bindings: ${[...expected].join(", ")}`);
}
