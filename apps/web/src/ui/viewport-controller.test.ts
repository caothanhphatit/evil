import { describe, expect, it } from "vitest";
import { classifyShellViewport } from "./viewport-controller";

describe("classifyShellViewport", () => {
  it.each([
    [405, 720, "compact"], [393, 852, "compact"], [390, 720, "narrow"], [520, 720, "compact"], [680, 720, "standard"], [900, 720, "wide"],
  ] as const)("maps %d x %d to %s", (width, height, mode) => expect(classifyShellViewport(width, height).mode).toBe(mode));
  it("tracks short shell heights independently of width", () => {
    expect(classifyShellViewport(405, 560)).toMatchObject({ short: true, veryShort: true, gearShort: true });
    expect(classifyShellViewport(405, 620)).toMatchObject({ short: true, veryShort: false, gearShort: true });
    expect(classifyShellViewport(405, 641)).toMatchObject({ short: false, veryShort: false, gearShort: false });
  });
});
