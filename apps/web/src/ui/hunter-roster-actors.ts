import { Spine } from "@esotericsoftware/spine-pixi-v8";
import { Application, Graphics } from "pixi.js";
import type { HunterView } from "./hunter-roster";
import { prepareHunterPaperDoll } from "../game/hunter-paper-doll";
import { HUNTER_ATLAS_ALIAS, HUNTER_SKELETON_ALIAS, preloadHunterPresentationAssets } from "./hunter-presentation-assets";

export interface HunterRosterActorController {
  preload(): Promise<void>;
  render(hunters: HunterView[]): Promise<void>;
  refresh(): void;
}

export function createHunterRosterActors(host: HTMLElement): HunterRosterActorController {
  const app = new Application();
  let initialized: Promise<void> | null = null;
  let latest: HunterView[] = [];
  let ready = false;
  const entries = new Map<string, { key: string; spine: Spine; clip: Graphics }>();
  const redraw = (): void => {
    if (ready && app.canvas.isConnected) draw(app, host, latest, entries);
  };

  const initialize = (): Promise<void> => initialized ??= (async () => {
    await app.init({ backgroundAlpha: 0, antialias: false, resizeTo: host, resolution: Math.min(window.devicePixelRatio, 2) });
    ready = true;
    app.canvas.className = "hunter-roster-actor-canvas";
    await preloadHunterPresentationAssets();
    host.addEventListener("scroll", redraw, { passive: true });
    new ResizeObserver(redraw).observe(host);
  })();

  return {
    preload: initialize,
    refresh() {
      redraw();
    },
    async render(hunters) {
      latest = hunters;
      await initialize();
      if (!app.canvas.isConnected) host.append(app.canvas);
      draw(app, host, latest, entries);
    },
  };
}

function draw(app: Application, host: HTMLElement, hunters: HunterView[], entries: Map<string, { key: string; spine: Spine; clip: Graphics }>): void {
  const hostBounds = host.getBoundingClientRect();
  const cards = [...host.querySelectorAll<HTMLElement>(".hunter-roster-card:not(.empty)")];
  const activeIds = new Set(hunters.map((hunter) => hunter.id));
  for (const [id, entry] of entries) {
    if (activeIds.has(id)) continue;
    entry.spine.destroy({ children: true });
    entry.clip.destroy();
    entries.delete(id);
  }
  hunters.forEach((hunter, index) => {
    if (hunter.portrait || !cards[index]) {
      const stale = entries.get(hunter.id);
      if (stale) {
        stale.spine.destroy({ children: true });
        stale.clip.destroy();
        entries.delete(hunter.id);
      }
      return;
    }
    const avatar = cards[index].querySelector<HTMLElement>(".hunter-avatar");
    if (!avatar) return;
    const avatarBounds = avatar.getBoundingClientRect();
    // The slot itself is outside the compact roster card, but its identity
    // still invalidates the actor so a purchase/equip cannot leave stale art.
    const key = `${hunter.classFamily ?? ""}:${hunter.animation ?? ""}:${hunter.equippedWeaponInstanceId ?? ""}`;
    let entry = entries.get(hunter.id);
    if (!entry || entry.key !== key) {
      if (entry) {
        entry.spine.destroy({ children: true });
        entry.clip.destroy();
      }
      const spine = Spine.from({ skeleton: HUNTER_SKELETON_ALIAS, atlas: HUNTER_ATLAS_ALIAS, autoUpdate: true });
      prepareHunterPaperDoll(spine, {
        entity_id: hunter.id,
        hunter_id: hunter.numericId,
        class_family: hunter.classFamily,
        animation: hunter.animation,
      }, hunter.classFamily, "roster-hunter");
      entry = { key, spine, clip: new Graphics() };
      entries.set(hunter.id, entry);
      app.stage.addChild(entry.clip, entry.spine);
    }
    const spine = entry.spine;
    const bounds = spine.getLocalBounds();
    const scale = Math.min(
      bounds.width > 0 ? avatarBounds.width * 0.82 / bounds.width : 1,
      bounds.height > 0 ? avatarBounds.height * 0.82 / bounds.height : 1,
    );
    spine.scale.set(scale);
    spine.x = avatarBounds.left - hostBounds.left + avatarBounds.width / 2 - (bounds.x + bounds.width / 2) * scale;
    spine.y = avatarBounds.bottom - hostBounds.top - 2 - (bounds.y + bounds.height) * scale;
    entry.clip.clear()
      .rect(
        avatarBounds.left - hostBounds.left,
        avatarBounds.top - hostBounds.top,
        avatarBounds.width,
        avatarBounds.height,
      )
      .fill(0xffffff);
    spine.mask = entry.clip;
  });
}
