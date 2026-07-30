import { describe, expect, it } from "vitest";
import { nextHunterRosterOpen, syncWorldFocusMenu } from "./bottom-menu-state";

describe("Hunter bottom-tab toggle", () => {
  it("opens from the bottom bar whenever the panel is actually closed", () => {
    expect(nextHunterRosterOpen(true, false)).toBe(true);
  });

  it("closes only when the panel is actually visible", () => {
    expect(nextHunterRosterOpen(true, true)).toBe(false);
  });

  it("keeps the top shortcut as an open-only action", () => {
    expect(nextHunterRosterOpen(false, true)).toBe(true);
  });
});

describe("shared-world focus menu", () => {
  it("restores field focus after reconnect without overriding an open panel", () => {
    expect(syncWorldFocusMenu("field", null)).toBe("field");
    expect(syncWorldFocusMenu("field", "build")).toBe("build");
  });

  it("clears stale field selection after returning to town", () => {
    expect(syncWorldFocusMenu("village", "field")).toBeNull();
    expect(syncWorldFocusMenu("village", "build")).toBe("build");
  });
});
