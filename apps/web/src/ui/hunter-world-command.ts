import { t } from "../i18n";

export const HUNTER_COMMAND_CATEGORIES = [
  { id: "items", label: t("hunter.command.items"), fixtureGlyph: "VP" },
  { id: "movement", label: t("hunter.command.movement"), fixtureGlyph: "DC" },
  { id: "learn", label: t("hunter.command.learn"), fixtureGlyph: "H" },
  { id: "daily-life", label: t("hunter.command.daily_life"), fixtureGlyph: "SH" },
  { id: "management", label: t("hunter.command.management"), fixtureGlyph: "QL" },
] as const;

export const HUNTER_HUNTING_REGIONS = [
  { id: "map_new01", label: t("hunter.command.colony"), fixtureGlyph: "I", tone: "colony" },
  { id: "background_08", label: t("hunter.command.dead_land"), fixtureGlyph: "II", tone: "dead-land" },
  { id: "background_11", label: t("hunter.command.demon_world"), fixtureGlyph: "III", tone: "demon-world" },
] as const;

export type HunterCommandCategory = typeof HUNTER_COMMAND_CATEGORIES[number]["id"];
export type HunterHuntingRegionId = typeof HUNTER_HUNTING_REGIONS[number]["id"];

export type HunterWorldCommandIntent =
  | { type: "assign_hunter_hunting_region"; hunterEntityId: string; regionId: HunterHuntingRegionId }
  | { type: "sell_hunter_loot"; hunterEntityId: string }
  | { type: "request_hunter_shop"; hunterEntityId: string; shopId: "build_7" | "build_8" | "build_20" };

export interface HunterGearEnhancementRequestIntent {
  type: "request_hunter_gear_enhancement";
  hunterEntityId: string;
  buildingId: "build_15";
}

export interface HunterWorldCommandSelection {
  entityId: string;
  displayName: string;
  screenPoint: { x: number; y: number };
}

export type HunterWorldCommandState =
  | { mode: "closed" }
  | { mode: "categories"; selection: HunterWorldCommandSelection }
  | { mode: "items"; selection: HunterWorldCommandSelection }
  | { mode: "movement"; selection: HunterWorldCommandSelection };

export type HunterWorldCommandEvent =
  | { type: "select_hunter"; selection: HunterWorldCommandSelection }
  | { type: "open_category"; category: HunterCommandCategory }
  | { type: "back" }
  | { type: "close" };

export interface HunterWorldCommandCallbacks {
  onInfo: (entityId: string) => void;
  onIntent: (intent: HunterWorldCommandIntent) => void;
  onEnhancementRequest?: (intent: HunterGearEnhancementRequestIntent) => void;
  onRelease: (entityId: string) => void;
  onUnavailable?: (category: Exclude<HunterCommandCategory, "items" | "movement">) => void;
}

export interface HunterWorldCommandMenu {
  selectHunter(selection: HunterWorldCommandSelection): void;
  close(): void;
  state(): HunterWorldCommandState;
  selectedEntityId(): string | null;
  destroy(): void;
}

export function reduceHunterWorldCommandState(
  state: HunterWorldCommandState,
  event: HunterWorldCommandEvent,
): HunterWorldCommandState {
  if (event.type === "select_hunter") return { mode: "categories", selection: event.selection };
  if (event.type === "close") return { mode: "closed" };
  if (state.mode === "closed") return state;
  if (event.type === "back") return { mode: "categories", selection: state.selection };
  if (event.type === "open_category" && event.category === "movement") {
    return { mode: "movement", selection: state.selection };
  }
  if (event.type === "open_category" && event.category === "items") {
    return { mode: "items", selection: state.selection };
  }
  return state;
}

export function createHunterWorldCommandMenu(
  host: HTMLElement,
  callbacks: HunterWorldCommandCallbacks,
): HunterWorldCommandMenu {
  const root = document.createElement("section");
  root.className = "hunter-world-command-layer";
  root.hidden = true;
  root.setAttribute("aria-label", t("hunter.command.aria"));
  host.append(root);
  let current: HunterWorldCommandState = { mode: "closed" };

  const transition = (event: HunterWorldCommandEvent): void => {
    current = reduceHunterWorldCommandState(current, event);
    render();
  };

  const render = (): void => {
    root.replaceChildren();
    root.hidden = current.mode === "closed";
    if (current.mode === "closed") return;
    const state = current;

    const actionBubble = document.createElement("div");
    actionBubble.className = "hunter-world-action-bubble";
    actionBubble.style.setProperty("--hunter-command-x", `${state.selection.screenPoint.x}px`);
    actionBubble.style.setProperty("--hunter-command-y", `${state.selection.screenPoint.y}px`);
    actionBubble.setAttribute("aria-label", t("hunter.command.actions_aria", { name: state.selection.displayName }));

    const info = iconButton("hunter-world-info", "i", t("hunter.command.info_aria", { name: state.selection.displayName }));
    info.dataset.evidence = "screenshot-reconstruction";
    info.addEventListener("click", () => callbacks.onInfo(state.selection.entityId));
    const command = iconButton("hunter-world-command", "X", t("hunter.command.order_aria", { name: state.selection.displayName }));
    command.dataset.evidence = "screenshot-reconstruction";
    command.setAttribute("aria-label", t("hunter.command.release_aria", { name: state.selection.displayName }));
    command.addEventListener("click", () => {
      callbacks.onRelease(state.selection.entityId);
      transition({ type: "close" });
    });
    actionBubble.append(info, command);
    root.append(actionBubble);

    const panel = document.createElement("section");
    panel.className = "hunter-world-command-panel";
    panel.setAttribute("aria-label", t("hunter.command.menu_aria", { name: state.selection.displayName }));
    const speech = document.createElement("p");
    speech.className = "hunter-world-command-line";
    speech.textContent = state.mode === "movement"
      ? t("hunter.command.speech.movement")
      : state.mode === "items"
        ? t("hunter.command.speech.items")
        : t("hunter.command.speech.default");
    panel.append(speech);

    const options = document.createElement("nav");
    options.className = state.mode === "movement" ? "hunter-region-options" : "hunter-command-categories";
    options.setAttribute("aria-label", state.mode === "movement"
      ? t("hunter.command.regions_aria")
      : state.mode === "items"
        ? t("hunter.command.items_aria")
        : t("hunter.command.categories_aria"));
    if (state.mode === "movement") {
      const back = menuButton(t("common.back"), "back", "←");
      back.addEventListener("click", () => transition({ type: "back" }));
      options.append(back);
      for (const region of HUNTER_HUNTING_REGIONS) {
        const button = menuButton(region.label, region.id, region.fixtureGlyph);
        button.classList.add(`region-${region.tone}`);
        button.dataset.evidence = "user-screenshot-order";
        button.addEventListener("click", () => {
          callbacks.onIntent({
            type: "assign_hunter_hunting_region",
            hunterEntityId: state.selection.entityId,
            regionId: region.id,
          });
          transition({ type: "close" });
        });
        options.append(button);
      }
    } else if (state.mode === "items") {
      const back = menuButton(t("common.back"), "back", "←");
      back.addEventListener("click", () => transition({ type: "back" }));
      options.append(back);

      const sell = menuButton(t("hunter.command.sell_loot"), "sell_hunter_loot", "BNL");
      sell.dataset.evidence = "web-rebuild-confirmed-auto-trade-flow";
      sell.addEventListener("click", () => {
        callbacks.onIntent({ type: "sell_hunter_loot", hunterEntityId: state.selection.entityId });
        transition({ type: "close" });
      });
      options.append(sell);

      for (const shop of [
        { shopId: "build_7", label: t("hunter.command.buy_weapon"), glyph: "VK" },
        { shopId: "build_8", label: t("hunter.command.buy_armor"), glyph: "GA" },
        { shopId: "build_20", label: t("hunter.command.buy_accessory"), glyph: "PK" },
      ] as const) {
        const buy = menuButton(shop.label, `request_hunter_shop_${shop.shopId}`, shop.glyph);
        buy.dataset.evidence = "user-confirmed-guided-hunter-purchase-flow";
        buy.addEventListener("click", () => {
          callbacks.onIntent({
            type: "request_hunter_shop",
            hunterEntityId: state.selection.entityId,
            shopId: shop.shopId,
          });
          transition({ type: "close" });
        });
        options.append(buy);
      }

      const enhance = menuButton(t("hunter.command.enhance_gear"), "request_hunter_gear_enhancement", "CH");
      enhance.dataset.evidence = "user-supplied-enhancement-flow";
      enhance.addEventListener("click", () => {
        callbacks.onEnhancementRequest?.({
          type: "request_hunter_gear_enhancement",
          hunterEntityId: state.selection.entityId,
          buildingId: "build_15",
        });
        transition({ type: "close" });
      });
      options.append(enhance);
    } else {
      for (const category of HUNTER_COMMAND_CATEGORIES) {
        const button = menuButton(category.label, category.id, category.fixtureGlyph);
        button.dataset.evidence = category.id === "movement" ? "user-screenshot-category" : "unresolved-icon-fixture";
        if (category.id !== "movement" && category.id !== "items") {
          button.title = t("hunter.command.unresolved_icon");
        }
        button.addEventListener("click", () => {
          if (category.id === "movement" || category.id === "items") {
            transition({ type: "open_category", category: category.id });
          }
          else callbacks.onUnavailable?.(category.id);
        });
        options.append(button);
      }
    }
    panel.append(options);
    root.append(panel);
  };

  return {
    selectHunter(selection) { transition({ type: "select_hunter", selection }); },
    close() { transition({ type: "close" }); },
    state() { return current; },
    selectedEntityId() { return current.mode === "closed" ? null : current.selection.entityId; },
    destroy() { root.remove(); current = { mode: "closed" }; },
  };
}

function iconButton(className: string, glyph: string, label: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = className;
  button.setAttribute("aria-label", label);
  const icon = document.createElement("span");
  icon.setAttribute("aria-hidden", "true");
  icon.textContent = glyph;
  button.append(icon);
  return button;
}

function menuButton(label: string, value: string, fixtureGlyph: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.dataset.value = value;
  const icon = document.createElement("span");
  icon.className = "hunter-command-fixture-icon";
  icon.setAttribute("aria-hidden", "true");
  icon.textContent = fixtureGlyph;
  const text = document.createElement("b");
  text.textContent = label;
  button.append(icon, text);
  return button;
}
