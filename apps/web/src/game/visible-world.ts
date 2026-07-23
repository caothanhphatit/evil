import { Spine } from "@esotericsoftware/spine-pixi-v8";
import { Skin } from "@esotericsoftware/spine-core";
import { Assets, Container, Sprite, Texture } from "pixi.js";

const RELEASE_MANIFEST = "/content/releases/visible-world-v1/release.json";
const WORLD_SIZE = 1000;

export interface VisualEntityProjection {
  descriptor: {
    entity_id: string;
    kind: "hunter" | "npc" | "monster";
    asset_bundle_id: string;
    source_skeleton_name: string;
  };
  x: number;
  y: number;
  facing: "left" | "right";
  animation: string;
  selectable: boolean;
}

interface ActorBundle {
  family: string;
  skeleton: { publicPath: string };
  atlas: { publicPath: string };
}

interface VisibleWorldManifest {
  releaseId: "visible-world-v1";
  map: { publicPath: string };
  village?: {
    tiles?: ScenePiece[];
    foreground?: ScenePiece[];
    decorations?: ScenePiece[];
  };
  actors: ActorBundle[];
}

interface ScenePiece {
  id?: string;
  publicPath: string;
  x: number;
  y: number;
  z?: number;
  scale?: number;
  anchor?: { x?: number; y?: number };
}

interface ActorView {
  root: Container;
  spine: Spine;
  animation: string;
  family: string;
  targetX: number;
  targetY: number;
}

export class VisibleEntityWorld {
  readonly root = new Container();
  private readonly entities = new Container();
  private readonly views = new Map<string, ActorView>();
  private readonly pending = new Set<string>();
  private readonly familyLoads = new Map<string, Promise<void>>();
  private readonly bundles = new Map<string, ActorBundle>();
  private readonly villageLayer = new Container();
  private readonly fieldLayer = new Container();
  private mode: "village" | "field" = "village";
  private latest: VisualEntityProjection[] = [];

  constructor(private readonly onSelect: (entityId: string) => void) {
    this.root.sortableChildren = true;
    this.entities.sortableChildren = true;
    this.villageLayer.sortableChildren = true;
    this.fieldLayer.sortableChildren = true;
  }

  async initialize(): Promise<void> {
    const response = await fetch(RELEASE_MANIFEST);
    if (!response.ok) throw new Error(`Visible-world manifest returned ${response.status}`);
    const manifest = await response.json() as VisibleWorldManifest;
    if (manifest.releaseId !== "visible-world-v1" || !Array.isArray(manifest.actors)) throw new Error("Visible-world manifest is invalid");
    for (const bundle of manifest.actors) this.bundles.set(bundle.family, bundle);

    await this.buildVillage(manifest);
    const texture = await Assets.load<Texture>(manifest.map.publicPath);
    const map = new Sprite(texture);
    map.anchor.set(0.5);
    map.position.set(WORLD_SIZE / 2);
    const scale = Math.max(WORLD_SIZE / texture.width, WORLD_SIZE / texture.height);
    map.scale.set(scale);
    this.fieldLayer.addChild(map);
    this.root.addChild(this.villageLayer, this.fieldLayer, this.entities);
    this.setMode(this.mode);
  }

  setMode(mode: "village" | "field"): void {
    this.mode = mode;
    this.villageLayer.visible = mode === "village";
    this.fieldLayer.visible = mode === "field";
  }

  resize(width: number, height: number): void {
    const scale = Math.max(width / WORLD_SIZE, height / WORLD_SIZE);
    this.root.scale.set(scale);
    this.root.position.set((width - WORLD_SIZE * scale) / 2, (height - WORLD_SIZE * scale) / 2);
  }

  update(entities: VisualEntityProjection[]): void {
    this.latest = entities;
    const active = new Set(entities.map((entity) => entity.descriptor.entity_id));
    for (const entity of entities) {
      const view = this.views.get(entity.descriptor.entity_id);
      if (view) this.project(view, entity);
      else void this.create(entity);
    }
    for (const [id, view] of this.views) {
      if (!active.has(id)) {
        view.root.destroy({ children: true });
        this.views.delete(id);
      }
    }
  }

  tick(deltaSeconds: number): void {
    const blend = 1 - Math.exp(-Math.min(deltaSeconds, 0.1) * 12);
    for (const view of this.views.values()) {
      view.root.position.x += (view.targetX - view.root.position.x) * blend;
      view.root.position.y += (view.targetY - view.root.position.y) * blend;
      view.root.zIndex = view.root.position.y;
    }
  }

  destroy(): void {
    this.root.destroy({ children: true });
    this.views.clear();
    this.pending.clear();
  }

  private async create(entity: VisualEntityProjection): Promise<void> {
    const id = entity.descriptor.entity_id;
    if (this.pending.has(id)) return;
    this.pending.add(id);
    try {
      const family = entity.descriptor.source_skeleton_name;
      const bundle = this.bundles.get(family);
      if (!bundle) throw new Error(`Visible-world bundle is missing for ${family}`);
      const skeletonAlias = `visible-world:${family}:skeleton`;
      const atlasAlias = `visible-world:${family}:atlas`;
      await this.loadFamily(family, skeletonAlias, atlasAlias, bundle);

      const current = this.latest.find((candidate) => candidate.descriptor.entity_id === id);
      if (!current) return;
      const spine = Spine.from({ skeleton: skeletonAlias, atlas: atlasAlias, autoUpdate: true });
      this.applyMigrationVisualSkin(spine, family);
      const root = new Container();
      root.eventMode = current.selectable ? "static" : "none";
      root.cursor = current.selectable ? "pointer" : "default";
      root.on("pointertap", () => this.onSelect(id));
      root.addChild(spine);
      this.entities.addChild(root);
      const view = { root, spine, animation: "", family, targetX: current.x, targetY: current.y };
      root.position.set(current.x, current.y);
      this.views.set(id, view);
      this.project(view, current);
    } catch (error) {
      console.warn(`Could not render authoritative entity ${id}.`, error);
    } finally {
      this.pending.delete(id);
    }
  }

  private project(view: ActorView, entity: VisualEntityProjection): void {
    view.targetX = entity.x;
    view.targetY = entity.y;
    view.root.eventMode = entity.selectable ? "static" : "none";
    view.root.cursor = entity.selectable ? "pointer" : "default";
    const direction = entity.facing === "left" ? -1 : 1;
    view.spine.scale.set(2.15 * direction, 2.15);
    if (view.animation === entity.animation) return;
    if (view.spine.skeleton.data.findAnimation(entity.animation)) {
      view.spine.state.setAnimation(0, entity.animation, true);
      view.animation = entity.animation;
    } else {
      console.warn(`Animation ${entity.animation} is missing from ${view.family}; setup pose retained.`);
      view.spine.state.clearTrack(0);
      view.animation = "";
    }
  }

  private loadFamily(family: string, skeletonAlias: string, atlasAlias: string, bundle: ActorBundle): Promise<void> {
    const existing = this.familyLoads.get(family);
    if (existing) return existing;
    const load = (async () => {
      if (Assets.cache.has(skeletonAlias) && Assets.cache.has(atlasAlias)) return;
      Assets.add({ alias: skeletonAlias, src: bundle.skeleton.publicPath });
      Assets.add({ alias: atlasAlias, src: bundle.atlas.publicPath });
      await Assets.load([skeletonAlias, atlasAlias]);
    })();
    this.familyLoads.set(family, load);
    return load;
  }

  private applyMigrationVisualSkin(spine: Spine, family: string): void {
    const candidates: Record<string, string[]> = {
      hunter: ["All_h1"],
      Chief: ["chief_body_01", "cos_01"],
      Npc: ["npc_01"],
      npc_animal: ["1"],
      mon_goldblin: ["lv1"],
      mon_a_01_1: ["lv1"],
    };
    const skinNames = candidates[family] ?? [];
    if (skinNames.length === 1 && spine.skeleton.data.findSkin(skinNames[0])) {
      spine.skeleton.setSkinByName(skinNames[0]);
    } else if (skinNames.length > 1) {
      const composition = new Skin(`visible-world:${family}`);
      for (const skinName of skinNames) {
        const skin = spine.skeleton.data.findSkin(skinName);
        if (skin) composition.addSkin(skin);
      }
      spine.skeleton.setSkin(composition);
    }
    spine.skeleton.setSlotsToSetupPose();
  }

  private async buildVillage(manifest: VisibleWorldManifest): Promise<void> {
    const fallback: ScenePiece[] = [
      ["background_01__1548.png", 4.30, 14.11], ["background_02__1515.png", 9.42, 14.11],
      ["background_05__1522.png", 24.78, 15.39], ["background_06__1547.png", 29.90, 14.11],
      ["background_07__1533.png", 4.30, 8.99], ["background_08__1530.png", 9.42, 8.99],
      ["background_11__1508.png", 24.78, 7.71], ["background_12__1519.png", 29.90, 8.99],
      ["background_13__1506.png", 4.30, 3.87], ["background_14__1541.png", 9.42, 3.87],
      ["background_15__1542.png", 14.54, 3.87], ["background_16__1517.png", 19.66, 3.87],
      ["background_17__1516.png", 24.78, 3.87], ["background_18__1535.png", 29.90, 3.87],
    ].map(([name, x, y]) => ({ publicPath: `/content/releases/original-flow-v1/sprites/${name}`, x: x as number, y: y as number, z: 499 }));
    const village = manifest.village;
    const pieces = village?.tiles?.length ? village.tiles : fallback;
    await this.addScenePieces(this.villageLayer, pieces, 499);
    await this.addScenePieces(this.villageLayer, village?.foreground ?? [], 486);
    await this.addScenePieces(this.villageLayer, village?.decorations ?? [], 492);
  }

  private async addScenePieces(layer: Container, pieces: ScenePiece[], defaultZ: number): Promise<void> {
    await Promise.all(pieces.map(async (piece) => {
      try {
        const texture = await Assets.load<Texture>(piece.publicPath);
        const sprite = new Sprite(texture);
        sprite.anchor.set(piece.anchor?.x ?? 0.5, piece.anchor?.y ?? 0.5);
        // Unity scene units map to the recovered 32 px grid used by the web viewport.
        sprite.position.set(piece.x * 31.25, (18 - piece.y) * 31.25);
        sprite.scale.set(piece.scale ?? 0.3125);
        sprite.zIndex = piece.z ?? defaultZ;
        layer.addChild(sprite);
      } catch (error) {
        console.warn(`Could not load village piece ${piece.publicPath}.`, error);
      }
    }));
  }
}
