import type { IntentFeedback } from "../net/world-client";
import { recordClientEvent } from "../observability/client-telemetry";
import { t, type MessageKey } from "../i18n";
import type { BuildingRenderingContext } from "./building-renderer";

export function showIntentResult(result: IntentFeedback, options: {
  context: BuildingRenderingContext;
  gearPopup: HTMLElement;
  consumablePopup: HTMLElement;
  gearSubmit: HTMLButtonElement;
  consumableSubmit: HTMLButtonElement;
  renderGear: () => void;
  renderConsumable: () => void;
  renderBuilding: () => void;
  renderTradeRequest: () => void;
  clearTradingRequest: () => void;
  showMessage: (title: string, detail: string) => void;
  debugUi: boolean;
  setAnimationTimer: (timer: number | undefined) => void;
  getAnimationTimer: () => number | undefined;
}): void {
  const { context } = options;
  if (result.intent === "craft_shop_item") {
    const pending = context.pendingCraft;
    context.pendingCraft = null;
    if (!result.accepted) {
      options.renderGear();
      options.renderConsumable();
    } else if (!pending) {
      options.showMessage(t("craft.completed"), t("craft.completed_detail"));
      return;
    } else {
      const popup = pending.popup === "gear" ? options.gearPopup : options.consumablePopup;
      const submit = pending.popup === "gear" ? options.gearSubmit : options.consumableSubmit;
      const stillShowingRequest = !popup.hidden && context.selectedRecipe?.id === pending.recipeId;
      if (!stillShowingRequest) {
        options.showMessage(t("craft.completed"), t("craft.completed_detail"));
        return;
      }
      popup.classList.remove("crafting");
      void popup.offsetWidth;
      popup.classList.add("crafting");
      popup.setAttribute("aria-busy", "true");
      submit.disabled = true;
      submit.textContent = t("craft.processing");
      const previous = options.getAnimationTimer();
      if (previous !== undefined) window.clearTimeout(previous);
      options.setAnimationTimer(window.setTimeout(() => {
        popup.classList.remove("crafting");
        popup.removeAttribute("aria-busy");
        popup.hidden = true;
        options.setAnimationTimer(undefined);
      }, 850));
      return;
    }
  }
  if (result.intent === "set_material_request") {
    context.tradingRequestPending = false;
    if (result.accepted) {
      options.clearTradingRequest();
      options.renderBuilding();
    } else if (context.selectedTradingRequest) {
      options.renderTradeRequest();
    }
  }
  if (result.intent === "purchase_shop_item") {
    const pending = context.pendingPurchase;
    context.pendingPurchase = null;
    if (result.accepted) {
      options.gearPopup.hidden = true;
      context.buildingPanel.hidden = false;
      const hunter = context.latestSnapshot?.hunter_roster.active_hunters.find((row) => row.hunter_id === context.purchaseHunterId);
      const equipped = pending?.productId.startsWith("recipe:weapon:")
        ? hunter?.hunter_info.weapons.find((weapon) => weapon.product_id === pending.productId && weapon.equipped)
        : null;
      options.showMessage(
        equipped ? t("shop.purchase_equipped") : t("shop.purchase_completed"),
        equipped ? t("shop.purchase_equipped_detail", { item: equipped.display_name_vi || equipped.display_name_en }) : t("shop.purchase_completed_detail"),
      );
    }
    options.renderBuilding();
    if (!result.accepted && !options.gearPopup.hidden) options.renderGear();
  }
  if (!result.accepted) {
    recordClientEvent("warn", "intent_rejected", { intent: result.intent, reason: result.reason });
    const reasons: Record<string, MessageKey> = {
      insufficient_materials: "error.insufficient_materials", material_stock_missing: "error.material_stock_missing", recipe_unknown: "error.recipe_unknown", recipe_building_mismatch: "error.recipe_building_mismatch", product_level_locked: "error.product_level_locked", sale_building_instance_unknown: "error.sale_building_missing", product_capacity_exceeded: "error.product_capacity", product_stock_empty: "error.product_empty", crafted_gear_stock_empty: "error.product_empty", sale_price_unresolved: "error.sale_price_unresolved", insufficient_hunter_gold: "error.insufficient_hunter_gold", hunter_not_in_town: "error.hunter_not_in_town", building_instance_unknown: "error.building_missing", building_capability_mismatch: "error.capability_mismatch", material_difficulty_unresolved: "error.material_difficulty_unresolved", material_difficulty_locked: "error.material_difficulty_locked", material_quantity_invalid: "error.material_quantity_invalid", material_price_unresolved: "error.material_price_unresolved",
    };
    const titles: Record<string, MessageKey> = { select_bottom_menu: "error.cannot_open_menu", navigate_back: "error.cannot_navigate_back", enter_field: "error.cannot_select_entity", select_entity: "error.cannot_select_entity", set_material_request: "error.cannot_request", purchase_shop_item: "error.cannot_purchase" };
    const reasonKey = reasons[result.reason ?? ""];
    const detail = reasonKey ? t(reasonKey) : options.debugUi && result.reason ? `${t("error.try_again")} (${result.reason})` : t("error.try_again");
    options.showMessage(t(titles[result.intent] ?? "error.cannot_craft"), detail);
  }
}
