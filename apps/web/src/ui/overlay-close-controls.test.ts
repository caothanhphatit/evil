import { describe, expect, it, vi } from "vitest";
import { bindOverlayCloseControls, PRIMARY_OVERLAY_CLOSE_CONTRACTS } from "./overlay-close-controls";

describe("primary overlay close contract", () => {
  it("closes every registered overlay through a real click event", () => {
    const state = new Map(PRIMARY_OVERLAY_CLOSE_CONTRACTS.map((contract) => [contract.overlay, true]));
    const controls = new Map<string, FakeControl>();
    for (const contract of PRIMARY_OVERLAY_CLOSE_CONTRACTS) {
      for (const id of contract.controls) controls.set(id, new FakeControl());
    }

    bindOverlayCloseControls(PRIMARY_OVERLAY_CLOSE_CONTRACTS.map((contract) => ({
      overlay: contract.overlay,
      controls: contract.controls.map((id) => controls.get(id)!),
      close: () => state.set(contract.overlay, false),
    })));

    for (const contract of PRIMARY_OVERLAY_CLOSE_CONTRACTS) {
      expect(state.get(contract.overlay)).toBe(true);
      controls.get(contract.controls[0])?.click();
      expect(state.get(contract.overlay)).toBe(false);
    }
  });

  it("fails closed when a primary overlay is missing or registered twice", () => {
    const close = vi.fn();
    const bindings = PRIMARY_OVERLAY_CLOSE_CONTRACTS.slice(1).map((contract) => ({
      overlay: contract.overlay,
      controls: [new FakeControl()],
      close,
    }));
    expect(() => bindOverlayCloseControls(bindings)).toThrow("Missing overlay close bindings: hunter-roster");
    expect(() => bindOverlayCloseControls([
      ...PRIMARY_OVERLAY_CLOSE_CONTRACTS.map((contract) => ({ overlay: contract.overlay, controls: [new FakeControl()], close })),
      { overlay: "hunter-roster", controls: [new FakeControl()], close },
    ])).toThrow("Unknown or duplicate overlay close binding: hunter-roster");
  });
});

class FakeControl extends EventTarget {
  click(): void { this.dispatchEvent(new Event("click")); }
}
