import { describe, expect, it } from "vitest";
import { fitWorldViewport, panWorldViewport } from "./camera";

describe("visible-world camera", () => {
  it("fills a portrait viewport and crops the wider world horizontally", () => {
    const transform = fitWorldViewport(506.25, 900, 1000);
    expect(transform.scale).toBe(0.9);
    expect(transform.x).toBe(-196.875);
    expect(transform.y).toBe(0);
  });

  it("guards zero-sized resize observer frames", () => {
    expect(fitWorldViewport(0, 0, 1000)).toEqual({ scale: 0.001, x: 0, y: 0 });
  });

  it("clamps panning inside a rectangular recovered scene", () => {
    expect(panWorldViewport(506.25, 900, 3072, 1536, -100, 2000)).toEqual({
      scale: 0.5859375,
      x: 0,
      y: 0,
    });
  });
});
