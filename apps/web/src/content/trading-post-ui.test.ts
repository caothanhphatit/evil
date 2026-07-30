import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const stylesPath = resolve(import.meta.dirname, "../styles.css");
const mainPath = resolve(import.meta.dirname, "../main.ts");

describe("Trading Post compact BuildingPop", () => {
  it("uses the shared compact shell and keeps its bottom actions visible", async () => {
    const styles = await readFile(stylesPath, "utf8");
    expect(styles).toContain(".game-shell .building-panel.source-popup.trading-post-ui");
    expect(styles).toContain("width: clamp(300px, 76%, 420px) !important");
    expect(styles).toContain("top: calc((100% - var(--bottom-menu-bottom) - var(--bottom-menu-reserved)) / 2)");
    expect(styles).toContain("height: min(560px, calc(100% - var(--bottom-menu-bottom) - var(--bottom-menu-reserved) - 12px))");
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

  it("keeps the quantity sub-popup open until the authoritative command succeeds", async () => {
    const source = await readFile(mainPath, "utf8");
    const submitHandler = source.slice(
      source.indexOf('submit.addEventListener("click"'),
      source.indexOf('back.addEventListener("click"'),
    );
    expect(submitHandler).toContain("client.setMaterialRequest");
    expect(submitHandler).toContain("tradingRequestPending = true");
    expect(source).toContain('if (result.intent === "set_material_request")');
    expect(source).toContain("if (result.accepted)");
    expect(source).toContain('editor.id = "trading-request-editor"');
    expect(source).toContain('id="trading-request-pop"');
    expect(source).toContain("tradingRequestContent.replaceChildren(editor)");
  });

  it("uses a dedicated source-style sub-popup instead of replacing the Trading Post catalog", async () => {
    const [source, styles] = await Promise.all([readFile(mainPath, "utf8"), readFile(stylesPath, "utf8")]);
    expect(source).toContain("trading-request-pop source-popup");
    expect(styles).toContain(".trading-request-editor");
    expect(styles).toContain(".trading-request-pop");
    expect(source).not.toContain("buildingCatalog.replaceChildren(editor)");
  });

  it("does not retain generic service-tab fallback markup", async () => {
    const [source, styles] = await Promise.all([readFile(mainPath, "utf8"), readFile(stylesPath, "utf8")]);
    expect(source).not.toContain("tabs.innerHTML");
    expect(source).not.toContain('document.createElement(serviceRow ? "div" : "button")');
    expect(source).toContain("renderBuildingContractError");
    expect(styles).not.toContain(".service-tabs b, .service-tabs span");
  });
});
