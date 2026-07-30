import type { MaterialStockSnapshot, OriginalFlowSnapshot } from "../generated/protocol";
import type { WorldClient } from "../net/world-client";
import { clampQuantity } from "../ui/shop-crafting";
import { formatNumber, t } from "../i18n";

export interface TradePopupContext {
  client: WorldClient;
  selectedTradingRequest: MaterialStockSnapshot | null;
  selectedTradingRequestQuantity: number;
  selectedBuildingInstanceId: string | null;
  tradingRequestPending: boolean;
  tradingRequestContent: HTMLElement;
  tradingRequestPop: HTMLElement;
  showPanelMessage(title: string, detail: string): void;
  resourceIconPath(resourceId: string): string | null;
  renderBuildingSystem(snapshot: OriginalFlowSnapshot | null): void;
  latestSnapshot: OriginalFlowSnapshot | null;
}
export function createTradePopup(context: TradePopupContext) {
  function renderTradingRequestEditor(): void {
    const stock = context.selectedTradingRequest;
    if (!stock) return;
    const editor = document.createElement("section");
    editor.id = "trading-request-editor";
    editor.className = "trading-request-editor";
    editor.setAttribute("aria-label", t("trading.request_editor_aria"));
  
    const product = document.createElement("div");
    product.className = "trading-request-product";
    const frame = document.createElement("div");
    frame.className = "trading-request-frame";
    const icon = document.createElement("img");
    icon.alt = "";
    icon.src = stock.icon || context.resourceIconPath(stock.id) || "";
    icon.hidden = !icon.src;
    const count = document.createElement("output");
    count.textContent = String(context.selectedTradingRequestQuantity);
    frame.append(icon, count);
  
    const controls = document.createElement("div");
    controls.className = "trading-request-controls";
    const label = document.createElement("span");
    label.textContent = t("common.quantity");
    const stepper = document.createElement("div");
    stepper.className = "quantity-stepper";
    const minus = document.createElement("button");
    minus.id = "trading-request-minus";
    minus.className = "consum-round-button trading-quantity-button";
    minus.type = "button";
    minus.textContent = "−";
    minus.setAttribute("aria-label", t("craft.decrease_quantity"));
    const input = document.createElement("input");
    input.id = "trading-request-quantity-input";
    input.type = "number";
    input.min = "1";
    input.max = "10000";
    input.step = "1";
    input.inputMode = "numeric";
    input.value = String(context.selectedTradingRequestQuantity);
    input.setAttribute("aria-label", t("trading.quantity_aria"));
    const plus = document.createElement("button");
    plus.id = "trading-request-plus";
    plus.className = "consum-round-button trading-quantity-button";
    plus.type = "button";
    plus.textContent = "+";
    plus.setAttribute("aria-label", t("craft.increase_quantity"));
    stepper.append(minus, input, plus);
  
    const quickSteps = document.createElement("div");
    quickSteps.className = "quantity-step-buttons";
    [1, 10, 100, 1000].forEach((delta) => {
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = `+${delta}`;
      button.addEventListener("click", () => {
        context.selectedTradingRequestQuantity = clampQuantity(context.selectedTradingRequestQuantity + delta, 1, 10_000);
        renderTradingRequestEditor();
      });
      quickSteps.append(button);
    });
    const max = document.createElement("button");
    max.type = "button";
    max.textContent = "∞";
    max.addEventListener("click", () => {
      context.selectedTradingRequestQuantity = 10_000;
      renderTradingRequestEditor();
  
    });
    quickSteps.append(max);
    controls.append(label, stepper, quickSteps);
    product.append(frame, controls);
  
    const name = document.createElement("strong");
    name.id = "trading-request-name";
    name.textContent = stock.display_name;
    const total = document.createElement("p");
    total.innerHTML = `${t("trading.request_quantity", { quantity: context.selectedTradingRequestQuantity })}<br />${t("trading.estimated_total")}<br /><span>${formatNumber(stock.unit_price * context.selectedTradingRequestQuantity)} ${t("common.gold")}</span>`;
    const actions = document.createElement("div");
    actions.className = "source-popup-actions";
    const submit = document.createElement("button");
    submit.id = "trading-request-submit";
    submit.className = "source-green-button";
    submit.type = "button";
    submit.disabled = context.tradingRequestPending || !context.selectedBuildingInstanceId;
    submit.textContent = context.tradingRequestPending ? t("common.requesting") : t("common.request");
    const back = document.createElement("button");
    back.id = "trading-request-close";
    back.className = "source-red-button";
    back.type = "button";
    back.textContent = t("common.back");
  
    const updateQuantity = (value: string | number): void => {
      context.selectedTradingRequestQuantity = clampQuantity(value, 1, 10_000);
      renderTradingRequestEditor();
    };
    minus.addEventListener("click", () => updateQuantity(context.selectedTradingRequestQuantity - 1));
    plus.addEventListener("click", () => updateQuantity(context.selectedTradingRequestQuantity + 1));
    input.addEventListener("change", () => updateQuantity(input.value));
    submit.addEventListener("click", () => {
      if (!context.selectedBuildingInstanceId || !context.selectedTradingRequest) {
        context.showPanelMessage(t("error.cannot_request"), t("error.trading_unavailable"));
        return;
      }
      if (!context.client.setMaterialRequest(context.selectedBuildingInstanceId, context.selectedTradingRequest.id, context.selectedTradingRequestQuantity)) {
        context.showPanelMessage(t("error.request_send_failed"), t("error.connection_not_ready"));
        return;
      }
      context.tradingRequestPending = true;
      renderTradingRequestEditor();
    });
    back.addEventListener("click", () => {
      context.selectedTradingRequest = null;
      context.tradingRequestPending = false;
      context.tradingRequestPop.hidden = true;
      context.renderBuildingSystem(context.latestSnapshot);
    });
    actions.append(submit, back);
    editor.append(product, name, total, actions);
    context.tradingRequestContent.replaceChildren(editor);
    context.tradingRequestPop.hidden = false;
    }

  return { renderTradingRequestEditor };
}
