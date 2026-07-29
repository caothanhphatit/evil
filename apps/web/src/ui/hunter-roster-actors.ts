import { Spine } from "@esotericsoftware/spine-pixi-v8";
import { Application, Assets, Graphics } from "pixi.js";
import type { HunterView } from "./hunter-roster";
import { hunterActorVisual } from "./hunter-roster";
import { applyHunterSpineSkin } from "./hunter-spine-presentation";

const SKELETON_ALIAS = "roster:hunter:skeleton";
const ATLAS_ALIAS = "roster:hunter:atlas";
const SKELETON_PATH = "/content/releases/visible-world-v1/actors/hunter/hunter.json";
const ATLAS_PATH = "/content/releases/visible-world-v1/actors/hunter/hunter.atlas";

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

  const initialize = (): Promise<void> => initialized ??= (async () => {
    await app.init({ backgroundAlpha: 0, antialias: false, resizeTo: host, resolution: Math.min(window.devicePixelRatio, 2) });
    ready = true;
    app.canvas.className = "hunter-roster-actor-canvas";
    if (!Assets.cache.has(SKELETON_ALIAS)) Assets.add({ alias: SKELETON_ALIAS, src: SKELETON_PATH });
    if (!Assets.cache.has(ATLAS_ALIAS)) Assets.add({ alias: ATLAS_ALIAS, src: ATLAS_PATH });
    await Assets.load([SKELETON_ALIAS, ATLAS_ALIAS]);
  })();

  return {
    preload: initialize,
    refresh() {
      if (ready && app.canvas.isConnected) draw(app, host, latest);
    },
    async render(hunters) {
      latest = hunters;
      await initialize();
      if (!app.canvas.isConnected) host.append(app.canvas);
      draw(app, host, latest);
    },
  };
}

function draw(app: Application, host: HTMLElement, hunters: HunterView[]): void {
  app.stage.removeChildren().forEach((child) => child.destroy({ children: true }));
  const hostBounds = host.getBoundingClientRect();
  const cards = [...host.querySelectorAll<HTMLElement>(".hunter-roster-card:not(.empty)")];
  hunters.forEach((hunter, index) => {
    if (hunter.portrait || !cards[index]) return;
    const avatar = cards[index].querySelector<HTMLElement>(".hunter-avatar");
    if (!avatar) return;
    const avatarBounds = avatar.getBoundingClientRect();
    const spine = Spine.from({ skeleton: SKELETON_ALIAS, atlas: ATLAS_ALIAS, autoUpdate: true });
    const visual = hunterActorVisual({
      entity_id: hunter.id,
      hunter_id: hunter.numericId,
      class_family: hunter.classFamily,
      animation: hunter.animation,
    });
    applyHunterSpineSkin(spine, visual.skinNames, hunter.classFamily, "roster-hunter");
    // Roster cards are paper-doll previews. Runtime combat/death animation is
    // reserved for the Pixi world actor and must not distort card composition.
    const animation = "hunter_stay";
    if (spine.skeleton.data.findAnimation(animation)) spine.state.setAnimation(0, animation, true);
    spine.tint = visual.tint;
    const bounds = spine.getLocalBounds();
    const scale = Math.min(
      bounds.width > 0 ? avatarBounds.width * 0.88 / bounds.width : 1,
      bounds.height > 0 ? avatarBounds.height * 0.9 / bounds.height : 1,
      2.15,
    );
    spine.scale.set(scale);
    spine.x = avatarBounds.left - hostBounds.left + avatarBounds.width / 2 - (bounds.x + bounds.width / 2) * scale;
    spine.y = avatarBounds.bottom - hostBounds.top - 2 - (bounds.y + bounds.height) * scale;
    const clip = new Graphics()
      .rect(
        avatarBounds.left - hostBounds.left,
        avatarBounds.top - hostBounds.top,
        avatarBounds.width,
        avatarBounds.height,
      )
      .fill(0xffffff);
    spine.mask = clip;
    app.stage.addChild(clip, spine);
  });
}
