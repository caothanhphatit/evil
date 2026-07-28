export const HUNTER_COMMAND_CATEGORIES = [
  { id: "items", label: "Vật Phẩm", fixtureGlyph: "VP" },
  { id: "movement", label: "Di Chuyển", fixtureGlyph: "DC" },
  { id: "learn", label: "Học", fixtureGlyph: "H" },
  { id: "daily-life", label: "Sinh Hoạt", fixtureGlyph: "SH" },
  { id: "management", label: "Quản Lý", fixtureGlyph: "QL" },
] as const;

export const HUNTER_HUNTING_REGIONS = [
  { id: "map_new01", label: "Thuộc Địa", fixtureGlyph: "I", tone: "colony" },
  { id: "background_08", label: "Tử Địa", fixtureGlyph: "II", tone: "dead-land" },
  { id: "background_11", label: "Ma Giới", fixtureGlyph: "III", tone: "demon-world" },
] as const;

export type HunterCommandCategory = typeof HUNTER_COMMAND_CATEGORIES[number]["id"];
export type HunterHuntingRegionId = typeof HUNTER_HUNTING_REGIONS[number]["id"];

export interface HunterWorldCommandIntent {
  type: "assign_hunter_hunting_region";
  hunterEntityId: string;
  regionId: HunterHuntingRegionId;
}

export interface HunterWorldCommandSelection {
  entityId: string;
  displayName: string;
  screenPoint: { x: number; y: number };
}

export type HunterWorldCommandState =
  | { mode: "closed" }
  | { mode: "categories"; selection: HunterWorldCommandSelection }
  | { mode: "movement"; selection: HunterWorldCommandSelection };

export type HunterWorldCommandEvent =
  | { type: "select_hunter"; selection: HunterWorldCommandSelection }
  | { type: "open_category"; category: HunterCommandCategory }
  | { type: "back" }
  | { type: "close" };

export interface HunterWorldCommandCallbacks {
  onInfo: (entityId: string) => void;
  onIntent: (intent: HunterWorldCommandIntent) => void;
  onRelease: (entityId: string) => void;
  onUnavailable?: (category: Exclude<HunterCommandCategory, "movement">) => void;
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
  return state;
}

export function createHunterWorldCommandMenu(
  host: HTMLElement,
  callbacks: HunterWorldCommandCallbacks,
): HunterWorldCommandMenu {
  const root = document.createElement("section");
  root.className = "hunter-world-command-layer";
  root.hidden = true;
  root.setAttribute("aria-label", "Hunter commands");
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
    actionBubble.setAttribute("aria-label", `${state.selection.displayName} actions`);

    const info = iconButton("hunter-world-info", "i", `Xem thông tin ${state.selection.displayName}`);
    info.dataset.evidence = "screenshot-reconstruction";
    info.addEventListener("click", () => callbacks.onInfo(state.selection.entityId));
    const command = iconButton("hunter-world-command", "X", `Ra lệnh cho ${state.selection.displayName}`);
    command.dataset.evidence = "screenshot-reconstruction";
    command.setAttribute("aria-label", `Bỏ chọn ${state.selection.displayName}`);
    command.addEventListener("click", () => {
      callbacks.onRelease(state.selection.entityId);
      transition({ type: "close" });
    });
    actionBubble.append(info, command);
    root.append(actionBubble);

    const panel = document.createElement("section");
    panel.className = "hunter-world-command-panel";
    panel.setAttribute("aria-label", `${state.selection.displayName} command menu`);
    const speech = document.createElement("p");
    speech.className = "hunter-world-command-line";
    speech.textContent = state.mode === "movement"
      ? "Không phải bạn bảo đang cần đến nơi nào thật gấp sao?"
      : "Thời tiết hôm nay thật đẹp... Có mưa không nhỉ?";
    panel.append(speech);

    const options = document.createElement("nav");
    options.className = state.mode === "movement" ? "hunter-region-options" : "hunter-command-categories";
    options.setAttribute("aria-label", state.mode === "movement" ? "Hunting regions" : "Hunter command categories");
    if (state.mode === "movement") {
      const back = menuButton("Quay Lại", "back", "←");
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
    } else {
      for (const category of HUNTER_COMMAND_CATEGORIES) {
        const button = menuButton(category.label, category.id, category.fixtureGlyph);
        button.dataset.evidence = category.id === "movement" ? "user-screenshot-category" : "unresolved-icon-fixture";
        if (category.id !== "movement") button.title = "Icon binding unresolved; command label follows the supplied screenshot.";
        button.addEventListener("click", () => {
          if (category.id === "movement") transition({ type: "open_category", category: category.id });
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
