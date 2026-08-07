import { Spine } from "@esotericsoftware/spine-pixi-v8";
import { Application } from "pixi.js";
import { hunterActorVisual } from "../../game/hunter-actor-presentation";
import { applyHunterSpineSkin, removeHunterPaperDollEffects } from "../../game/hunter-spine-presentation";
import type { HunterView } from "../hunter-roster";
import { HUNTER_ATLAS_ALIAS, HUNTER_SKELETON_ALIAS, preloadHunterPresentationAssets } from "../hunter-presentation-assets";

export interface HunterInfoActorController {
  preload(): Promise<void>;
  render(host: HTMLElement, hunter: HunterView): Promise<void>;
  clear(): void;
}

export function createHunterInfoActor(): HunterInfoActorController {
  const app = new Application();
  let initialized: Promise<void> | null = null;
  let ready = false;
  let renderEpoch = 0;

  const initialize = (): Promise<void> => initialized ??= (async () => {
    await app.init({ backgroundAlpha: 0, antialias: false, width: 1, height: 1, resolution: Math.min(window.devicePixelRatio, 2) });
    ready = true;
    app.canvas.className = "hunter-info-actor-canvas";
    await preloadHunterPresentationAssets();
  })();

  return {
    preload: initialize,
    async render(host, hunter) {
      const epoch = ++renderEpoch;
      try {
        await initialize();
        if (epoch !== renderEpoch || !host.isConnected) return;
        app.stage.removeChildren().forEach((child) => child.destroy({ children: true }));
        if (!app.canvas.isConnected) host.append(app.canvas);
        const width = Math.max(1, host.clientWidth);
        const height = Math.max(1, host.clientHeight);
        app.renderer.resize(width, height);

        const spine = Spine.from({ skeleton: HUNTER_SKELETON_ALIAS, atlas: HUNTER_ATLAS_ALIAS, autoUpdate: true });
        const visual = hunterActorVisual({
          entity_id: hunter.id,
          hunter_id: hunter.numericId,
          class_family: hunter.classFamily,
          animation: hunter.animation,
        });
        applyHunterSpineSkin(spine, visual.skinNames, hunter.classFamily, "hunter-info-hunter");
        // Detail is a paper-doll view: combat/death animations can rotate or
        // widen the skeleton and must not affect the equipment layout.
        const animation = "hunter_stay";
        if (spine.skeleton.data.findAnimation(animation)) spine.state.setAnimation(0, animation, true);
        removeHunterPaperDollEffects(spine);
        spine.tint = visual.tint;
        const bounds = spine.getLocalBounds();
        const scale = Math.min(
          bounds.width > 0 ? width * 0.68 / bounds.width : 1,
          bounds.height > 0 ? height * 0.62 / bounds.height : 1,
          1.9,
        );
        spine.scale.set(scale);
        spine.x = width / 2 - (bounds.x + bounds.width / 2) * scale;
        spine.y = height - 5 - (bounds.y + bounds.height) * scale;
        app.stage.addChild(spine);
      } catch (error) {
        console.warn("Could not render Hunter detail actor.", error);
      }
    },
    clear() {
      renderEpoch += 1;
      if (!ready) return;
      app.stage.removeChildren().forEach((child) => child.destroy({ children: true }));
      app.canvas.remove();
    },
  };
}
