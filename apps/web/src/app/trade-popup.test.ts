import { describe, expect, it } from "vitest";
import type { MaterialStockSnapshot } from "../generated/protocol";
import { createTradePopup, type TradePopupContext } from "./trade-popup";

describe("Trading Post request popup", () => {
  it("closes through its rendered Back button and clears transient request state", () => {
    const previousDocument = globalThis.document;
    Object.defineProperty(globalThis, "document", {
      value: { createElement: () => new FakeElement() },
      configurable: true,
      writable: true,
    });
    try {
      const content = new FakeElement();
      const popup = new FakeElement();
      popup.hidden = true;
      let renderedBuilding = false;
      const context = {
        client: { setMaterialRequest: () => true },
        selectedTradingRequest: materialStock(),
        selectedTradingRequestQuantity: 7,
        selectedBuildingInstanceId: "trading-post-1",
        tradingRequestPending: true,
        tradingRequestContent: content,
        tradingRequestPop: popup,
        showPanelMessage: () => undefined,
        resourceIconPath: () => null,
        renderBuildingSystem: () => { renderedBuilding = true; },
        latestSnapshot: null,
      } as unknown as TradePopupContext;

      createTradePopup(context).renderTradingRequestEditor();
      expect(popup.hidden).toBe(false);
      findById(content, "trading-request-close")?.click();

      expect(context.selectedTradingRequest).toBeNull();
      expect(context.tradingRequestPending).toBe(false);
      expect(popup.hidden).toBe(true);
      expect(renderedBuilding).toBe(true);
    } finally {
      Object.defineProperty(globalThis, "document", { value: previousDocument, configurable: true, writable: true });
    }
  });
});

function materialStock(): MaterialStockSnapshot {
  return {
    id: "material-1",
    display_name: "Material",
    town_quantity: 0,
    hunter_quantity: 0,
    requested: 0,
    unit_price: 5,
    icon: "",
    difficulty: 0,
  };
}

class FakeElement extends EventTarget {
  children: FakeElement[] = [];
  hidden = false;
  id = "";
  className = "";
  textContent = "";
  innerHTML = "";
  type = "";
  value = "";
  min = "";
  max = "";
  step = "";
  inputMode = "";
  disabled = false;
  src = "";
  alt = "";

  append(...children: FakeElement[]): void { this.children.push(...children); }
  replaceChildren(...children: FakeElement[]): void { this.children = children; }
  setAttribute(): void { /* Attribute values are outside this interaction contract. */ }
  click(): void { this.dispatchEvent(new Event("click")); }
}

function findById(root: FakeElement, id: string): FakeElement | undefined {
  if (root.id === id) return root;
  return root.children.map((child) => findById(child, id)).find(Boolean);
}
