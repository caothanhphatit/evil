import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import {
  createHunterWorldCommandMenu,
  HUNTER_COMMAND_CATEGORIES,
  HUNTER_HUNTING_REGIONS,
  reduceHunterWorldCommandState,
  type HunterWorldCommandSelection,
} from "./hunter-world-command";

const selection: HunterWorldCommandSelection = {
  entityId: "village-hunter-7",
  displayName: "Morris",
  screenPoint: { x: 210, y: 360 },
};

describe("Hunter world command menu", () => {
  it("opens the command tooltip immediately and keeps movement in the same layer", () => {
    const previousDocument = globalThis.document;
    const fakeDocument = { createElement: () => new FakeElement() };
    Object.defineProperty(globalThis, "document", { value: fakeDocument, configurable: true, writable: true });
    try {
      const host = new FakeElement();
      const intents: unknown[] = [];
      const menu = createHunterWorldCommandMenu(host as unknown as HTMLElement, {
        onInfo: () => undefined,
        onIntent: (intent) => intents.push(intent),
        onRelease: () => undefined,
      });
      menu.selectHunter(selection);
      const layer = host.children[0];
      expect(layer?.hidden).toBe(false);
      expect(layer?.children.map((child) => child.className)).toEqual([
        "hunter-world-action-bubble", "hunter-world-command-panel",
      ]);
      findByDataValue(layer, "movement")?.click();
      expect(findByDataValue(layer, "back")).toBeDefined();
      expect(findByDataValue(layer, "map_new01")).toBeDefined();

      findByDataValue(layer, "background_08")?.click();
      expect(intents).toEqual([{
        type: "assign_hunter_hunting_region",
        hunterEntityId: "village-hunter-7",
        regionId: "background_08",
      }]);
      expect(layer?.hidden).toBe(true);

      menu.selectHunter(selection);
      findByDataValue(layer, "items")?.click();
      expect(menu.state()).toEqual({ mode: "items", selection });
      expect(findByDataValue(layer, "sell_hunter_loot")).toBeDefined();
      expect(findByDataValue(layer, "request_hunter_gear_enhancement")).toBeDefined();
      findByDataValue(layer, "sell_hunter_loot")?.click();
      expect(intents[1]).toEqual({
        type: "sell_hunter_loot",
        hunterEntityId: "village-hunter-7",
      });
      expect(layer?.hidden).toBe(true);
    } finally {
      Object.defineProperty(globalThis, "document", { value: previousDocument, configurable: true, writable: true });
    }
  });

  it("moves through the screenshot interaction without replacing the selected Hunter", () => {
    const categories = reduceHunterWorldCommandState({ mode: "closed" }, { type: "select_hunter", selection });
    expect(categories).toEqual({ mode: "categories", selection });
    const movement = reduceHunterWorldCommandState(categories, { type: "open_category", category: "movement" });
    expect(movement).toEqual({ mode: "movement", selection });
    expect(reduceHunterWorldCommandState(movement, { type: "back" })).toEqual({ mode: "categories", selection });
    const items = reduceHunterWorldCommandState(categories, { type: "open_category", category: "items" });
    expect(items).toEqual({ mode: "items", selection });
    expect(reduceHunterWorldCommandState(items, { type: "back" })).toEqual({ mode: "categories", selection });
  });

  it("emits a travel request for build_15 instead of opening enhancement immediately", () => {
    const previousDocument = globalThis.document;
    const fakeDocument = { createElement: () => new FakeElement() };
    Object.defineProperty(globalThis, "document", { value: fakeDocument, configurable: true, writable: true });
    try {
      const host = new FakeElement();
      const intents: unknown[] = [];
      const menu = createHunterWorldCommandMenu(host as unknown as HTMLElement, {
        onInfo: () => undefined,
        onIntent: () => undefined,
        onEnhancementRequest: (intent) => intents.push(intent),
        onRelease: () => undefined,
      });

      menu.selectHunter(selection);
      const layer = host.children[0];
      findByDataValue(layer, "items")?.click();
      findByDataValue(layer, "request_hunter_gear_enhancement")?.click();

      expect(intents).toEqual([{
        type: "request_hunter_gear_enhancement",
        hunterEntityId: "village-hunter-7",
        buildingId: "build_15",
      }]);
      expect(menu.state()).toEqual({ mode: "closed" });
    } finally {
      Object.defineProperty(globalThis, "document", { value: previousDocument, configurable: true, writable: true });
    }
  });

  it("uses X only to close the tooltip and release the selected Hunter", () => {
    const previousDocument = globalThis.document;
    const fakeDocument = { createElement: () => new FakeElement() };
    Object.defineProperty(globalThis, "document", { value: fakeDocument, configurable: true, writable: true });
    try {
      const host = new FakeElement();
      const released: string[] = [];
      const menu = createHunterWorldCommandMenu(host as unknown as HTMLElement, {
        onInfo: () => undefined,
        onIntent: () => undefined,
        onRelease: (entityId) => released.push(entityId),
      });

      menu.selectHunter(selection);
      findByClass(host.children[0], "hunter-world-command")?.click();

      expect(released).toEqual([selection.entityId]);
      expect(menu.state()).toEqual({ mode: "closed" });
      expect(menu.selectedEntityId()).toBeNull();
      expect(host.children[0]?.hidden).toBe(true);
    } finally {
      Object.defineProperty(globalThis, "document", { value: previousDocument, configurable: true, writable: true });
    }
  });

  it("keeps the recovered five-category and three-region order explicit", () => {
    expect(HUNTER_COMMAND_CATEGORIES.map((category) => category.id)).toEqual([
      "items", "movement", "learn", "daily-life", "management",
    ]);
    expect(HUNTER_HUNTING_REGIONS.map((region) => region.id)).toEqual([
      "map_new01", "background_08", "background_11",
    ]);
  });

  it("renders menu state inside one persistent layer and emits intent instead of an outcome", async () => {
    const source = await readFile(new URL("./hunter-world-command.ts", import.meta.url), "utf8");
    const visibleWorld = await readFile(new URL("../game/visible-world.ts", import.meta.url), "utf8");
    const main = await readFile(new URL("../main.ts", import.meta.url), "utf8");
    expect(source).toContain('root.className = "hunter-world-command-layer"');
    expect(source).toContain("root.replaceChildren()");
    expect(source).toContain('type: "assign_hunter_hunting_region"');
    expect(visibleWorld).toContain("event.global.x");
    expect(visibleWorld).toContain("event.global.y");
    expect(source).not.toContain("damage");
    expect(source).not.toContain("experience");
    expect(source).not.toContain("drop");
    expect(main).not.toContain('showPanelMessage(t("feedback.hunt_sent")');
  });

  it("keeps the click tooltip compact and gives desktop a content-width command sheet", async () => {
    const styles = await readFile(new URL("../styles.css", import.meta.url), "utf8");
    expect(styles).toContain(".hunter-world-action-bubble button { position: relative; display: grid; place-items: center; width: 44px; height: 49px;");
    expect(styles).toContain(".hunter-world-command-panel { position: absolute; right: 7px;");
    expect(styles).toContain("min-height: 116px");
    expect(styles).toContain(".hunter-command-fixture-icon { position: relative; display: grid; place-items: center; width: 29px; height: 29px;");
    expect(styles).toContain(".hunter-world-command-panel { right: auto; left: 50%; width: min(480px, calc(100% - 32px)); transform: translateX(-50%); }");
  });
});

class FakeElement {
  children: FakeElement[] = [];
  hidden = false;
  className = "";
  textContent = "";
  title = "";
  type = "";
  readonly dataset: Record<string, string> = {};
  readonly style = { setProperty: () => undefined };
  readonly classList = { add: (...names: string[]) => { this.className = [this.className, ...names].filter(Boolean).join(" "); } };
  private readonly listeners = new Map<string, Array<() => void>>();

  append(...children: FakeElement[]): void { this.children.push(...children); }
  replaceChildren(...children: FakeElement[]): void { this.children = children; }
  setAttribute(): void { /* The interaction test does not inspect accessibility attributes. */ }
  addEventListener(type: string, listener: EventListenerOrEventListenerObject): void {
    const callback = typeof listener === "function" ? () => listener(new Event(type)) : () => listener.handleEvent(new Event(type));
    this.listeners.set(type, [...(this.listeners.get(type) ?? []), callback]);
  }
  click(): void { for (const listener of this.listeners.get("click") ?? []) listener(); }
  remove(): void { /* Detached ownership is not needed by this isolated fake tree. */ }
}

function findByClass(root: FakeElement | undefined, className: string): FakeElement | undefined {
  if (!root) return undefined;
  if (root.className.split(" ").includes(className)) return root;
  return root.children.map((child) => findByClass(child, className)).find(Boolean);
}

function findByDataValue(root: FakeElement | undefined, value: string): FakeElement | undefined {
  if (!root) return undefined;
  if (root.dataset.value === value) return root;
  return root.children.map((child) => findByDataValue(child, value)).find(Boolean);
}
