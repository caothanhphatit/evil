import { Spine } from "@esotericsoftware/spine-pixi-v8";
import { Skin } from "@esotericsoftware/spine-core";
import { Assets, Container, Graphics, Sprite, Text, Texture } from "pixi.js";
import { requireContentAsset } from "../assets/catalog";
import type { ContentRelease, EntityState, WorldSnapshot } from "../types";
import { animationFor, animationLoops } from "./animation";

interface ActorView {
  root: Container;
  actor: Spine | Graphics;
  hp: Graphics;
  label: Text;
  animation: string;
  equippedItemId: number | null;
}

export class VillageWorld {
  readonly root = new Container();
  private readonly actorLayer = new Container();
  private readonly dropLayer = new Container();
  private readonly views = new Map<number, ActorView>();
  private readonly drops = new Map<string, Container>();
  private release: ContentRelease | null = null;
  private dropTexture: Texture | null = null;

  async initialize(release: ContentRelease): Promise<void> {
    this.release = release;
    this.root.sortableChildren = true;
    this.drawArena();
    this.actorLayer.zIndex = 10;
    this.dropLayer.zIndex = 8;
    this.root.addChild(this.dropLayer, this.actorLayer);

    const icon = release.assets.get("icon.equipment.unique.image");
    if (icon) this.dropTexture = await Assets.load<Texture>(icon.publicPath);
    await this.preloadSpine("hunter", "actor.hunter.primary");
    await this.preloadSpine("monster", "actor.monster.a01.level1");
  }

  resize(width: number, height: number): void {
    const scale = Math.min(width / 900, height / 560);
    this.root.scale.set(Math.max(0.55, scale));
    this.root.position.set((width - 900 * this.root.scale.x) / 2, (height - 560 * this.root.scale.y) / 2);
  }

  update(snapshot: WorldSnapshot, entities: EntityState[]): void {
    const active = new Set<number>();
    for (const entity of entities) {
      active.add(entity.id);
      const view = this.views.get(entity.id) ?? this.createActor(entity, snapshot.equippedItemId);
      view.root.position.set(entity.x, entity.y);
      view.root.zIndex = entity.y;
      this.updateActor(view, entity, snapshot.equippedItemId);
    }

    for (const [id, view] of this.views) {
      if (!active.has(id)) {
        view.root.destroy({ children: true });
        this.views.delete(id);
      }
    }
    this.updateDrops(snapshot);
  }

  destroy(): void {
    this.root.destroy({ children: true });
    this.views.clear();
    this.drops.clear();
  }

  private async preloadSpine(alias: string, unitId: string): Promise<void> {
    if (!this.release) throw new Error("Content release is not initialized");
    const atlas = requireContentAsset(this.release, `${unitId}.atlas`);
    const skeleton = requireContentAsset(this.release, `${unitId}.skeleton`);
    Assets.add({ alias: `${alias}-atlas`, src: atlas.publicPath });
    Assets.add({ alias: `${alias}-skeleton`, src: skeleton.publicPath });
    await Assets.load([`${alias}-atlas`, `${alias}-skeleton`]);
  }

  private drawArena(): void {
    const ground = new Graphics()
      .rect(0, 0, 900, 560)
      .fill(0x253b2d)
      .ellipse(450, 330, 620, 245)
      .fill(0x65764a)
      .ellipse(450, 350, 520, 170)
      .fill({ color: 0xb49a67, alpha: 0.7 });
    const border = new Graphics()
      .roundRect(65, 88, 770, 390, 28)
      .stroke({ color: 0xd3b66d, width: 3, alpha: 0.28 });
    const title = new Text({ text: "SLICE 001  ·  AUTHORITATIVE TRAINING GROUND", style: { fontFamily: "Georgia", fontSize: 13, fill: 0xead8a4, letterSpacing: 2 } });
    title.anchor.set(0.5);
    title.position.set(450, 116);
    this.root.addChild(ground, border, title);
  }

  private createActor(entity: EntityState, equippedItemId: number | null): ActorView {
    const root = new Container();
    let actor: Spine | Graphics;
    try {
      actor = Spine.from({ skeleton: `${entity.kind}-skeleton`, atlas: `${entity.kind}-atlas`, autoUpdate: true });
      actor.scale.set(entity.kind === "hunter" ? 2.25 : 1.8);
      if (entity.kind === "hunter") this.applyHunterSkin(actor, equippedItemId);
      else actor.skeleton.setSkinByName("lv1");
      actor.skeleton.setSlotsToSetupPose();
    } catch (error) {
      if (!import.meta.env.DEV) throw error;
      console.warn(`Spine actor ${entity.kind} failed to initialize; using development fallback.`, error);
      actor = new Graphics().circle(0, -20, 20).fill(entity.kind === "hunter" ? 0xd3a64d : 0x713d56);
    }

    const hp = new Graphics();
    const label = new Text({ text: entity.name, style: { fontFamily: "Georgia", fontSize: 12, fill: 0xfff3cf, stroke: { color: 0x111d16, width: 3 } } });
    label.anchor.set(0.5, 0);
    label.position.set(0, 28);
    root.addChild(actor, hp, label);
    this.actorLayer.addChild(root);
    const view = { root, actor, hp, label, animation: "", equippedItemId };
    this.views.set(entity.id, view);
    return view;
  }

  private updateActor(view: ActorView, entity: EntityState, equippedItemId: number | null): void {
    const ratio = entity.max_hp > 0 ? Math.max(0, Math.min(1, entity.hp / entity.max_hp)) : 0;
    view.hp.clear().roundRect(-30, -73, 60, 7, 3).fill(0x1a1713).roundRect(-29, -72, 58 * ratio, 5, 2).fill(ratio > 0.35 ? 0x71bf62 : 0xd34f42);
    view.label.text = entity.name;
    if (!(view.actor instanceof Spine)) return;

    if (entity.kind === "hunter" && view.equippedItemId !== equippedItemId) {
      this.applyHunterSkin(view.actor, equippedItemId);
      view.equippedItemId = equippedItemId;
    }
    const animation = animationFor(entity.kind, entity.state, entity.facing);
    if (view.animation !== animation) {
      view.actor.state.setAnimation(0, animation, animationLoops(entity.state));
      view.animation = animation;
    }
  }

  private applyHunterSkin(actor: Spine, equippedItemId: number | null): void {
    const skin = new Skin("slice1-authorized-composition");
    const base = actor.skeleton.data.findSkin("All_h1");
    if (!base) throw new Error("Verified hunter skin All_h1 is missing");
    skin.addSkin(base);
    if (equippedItemId !== null) {
      const weapon = actor.skeleton.data.findSkin("weapon_h1a_a_01");
      if (!weapon) throw new Error("Verified hunter weapon skin is missing");
      skin.addSkin(weapon);
    }
    actor.skeleton.setSkin(skin);
    actor.skeleton.setSlotsToSetupPose();
  }

  private updateDrops(snapshot: WorldSnapshot): void {
    const active = new Set<string>();
    for (const drop of snapshot.groundDrops) {
      active.add(drop.drop_id);
      let view = this.drops.get(drop.drop_id);
      if (!view) {
        view = new Container();
        const glow = new Graphics().circle(0, 0, 27).fill({ color: 0xffd75e, alpha: 0.2 }).circle(0, 0, 18).stroke({ color: 0xffdf7a, width: 2, alpha: 0.75 });
        const icon = this.dropTexture ? new Sprite(this.dropTexture) : new Graphics().rect(-10, -10, 20, 20).fill(0xe2b856);
        if (icon instanceof Sprite) {
          icon.anchor.set(0.5);
          icon.scale.set(Math.min(34 / icon.texture.width, 34 / icon.texture.height));
        }
        const quantity = new Text({ text: `×${drop.quantity}`, style: { fontFamily: "Georgia", fontSize: 11, fill: 0xffefba, stroke: { color: 0x261d0d, width: 3 } } });
        quantity.position.set(14, 8);
        view.addChild(glow, icon, quantity);
        this.dropLayer.addChild(view);
        this.drops.set(drop.drop_id, view);
      }
      view.position.set(drop.x, drop.y - 14);
    }
    for (const [id, view] of this.drops) {
      if (!active.has(id)) {
        view.destroy({ children: true });
        this.drops.delete(id);
      }
    }
  }
}
