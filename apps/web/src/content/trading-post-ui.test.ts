import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const stylesPath = resolve(import.meta.dirname, "../styles.css");

describe("Trading Post compact BuildingPop", () => {
  it("uses the shared compact shell and keeps its bottom actions visible", async () => {
    const styles = await readFile(stylesPath, "utf8");
    expect(styles).toContain(".game-shell .building-panel.source-popup.trading-post-ui");
    expect(styles).toContain("width: clamp(300px, 76%, 420px) !important");
    expect(styles).toContain("max-height: calc(100% - 24px)");
    expect(styles).toContain("padding: 16px 16px 64px");
    expect(styles).toContain(".building-panel.source-popup.trading-post-ui > #building-panel-close");
    expect(styles).toContain("right: 11%; bottom: 14px; width: 36%");
  });

  it("keeps material actions touch-sized without oversized cards", async () => {
    const styles = await readFile(stylesPath, "utf8");
    expect(styles).toContain("grid-template-rows: 50px minmax(25px, auto) 28px");
    expect(styles).toContain("min-height: 27px");
    expect(styles).toContain("width: calc(100% - 10px)");
  });
});
