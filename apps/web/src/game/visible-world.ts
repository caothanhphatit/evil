import { Spine } from "@esotericsoftware/spine-pixi-v8";
import { Skin } from "@esotericsoftware/spine-core";
import { Assets, Container, Sprite, Texture } from "pixi.js";
import { loadVerifiedVisibleWorldRelease, type ActorBundle, type TownBuilding } from "../assets/visible-world-release";
import type { BuildingInstanceSnapshot, WorldEntityProjection } from "../generated/protocol";
import { projectRenderableBuildingInstances } from "./building-placement";
import { panWorldViewport } from "./camera";
import { sceneDepthFromUnityZ, villageActorDepth } from "./depth";
import { ProjectionBuffer } from "./projection-interpolation";
import { hunterActorVisual } from "../ui/hunter-roster";
import {
  FIELD_CAMERA_CENTER,
  FIELD_CAMERA_ZOOM,
  projectNormalizedEntityPoint,
  projectScenePoint,
  runtimeScenePieces,
  SCENE_WORLD_HEIGHT,
  SCENE_WORLD_WIDTH,
  TOWN_BUILDING_GRID,
  TOWN_CAMERA_CENTER,
  TOWN_CAMERA_ZOOM,
} from "./scene-projection";

const BUILDING_GRID_CELL_WIDTH = TOWN_BUILDING_GRID.cellWidth;
const BUILDING_GRID_CELL_HEIGHT = TOWN_BUILDING_GRID.cellHeight;
const BUILDING_GRID_ORIGIN_X = TOWN_BUILDING_GRID.originX;
const BUILDING_GRID_ORIGIN_Y = TOWN_BUILDING_GRID.originY;

export interface VisibleWorldDiagnostics {
  fixture: boolean;
  unresolved: string[];
}

interface ActorView {
  root: Container;
  spine: Spine;
  animation: string;
  family: string;
  skinSignature: string;
}
interface BuildingFootprint { id: string; x: number; y: number; halfWidth: number; top: number; }
interface BuildingTemplate { visual: TownBuilding; texture: Texture; }

export class VisibleEntityWorld {
  readonly root = new Container();
  private readonly views = new Map<string, ActorView>();
  private readonly pending = new Set<string>();
  private readonly familyLoads = new Map<string, Promise<void>>();
  private readonly bundles = new Map<string, ActorBundle>();
  private readonly projectionBuffer = new ProjectionBuffer();
  private buildingFootprints: BuildingFootprint[] = [];
  private readonly buildingTemplates = new Map<string, BuildingTemplate>();
  private readonly buildingInstances = new Map<string, BuildingInstanceSnapshot>();
  private readonly buildingSprites = new Map<string, Sprite>();
  private readonly villageLayer = new Container();
  private mode: "village" | "field" = "village";
  private latest: WorldEntityProjection[] = [];
  private viewportWidth = 1;
  private viewportHeight = 1;
  private cameraX: number = TOWN_CAMERA_CENTER.x;
  private cameraY: number = TOWN_CAMERA_CENTER.y;
  private cameraZoom = TOWN_CAMERA_ZOOM;

  constructor(
    private readonly onSelect: (entityId: string) => void,
    private readonly onBuildingSelect?: (instance: BuildingInstanceSnapshot, visual: TownBuilding) => void,
  ) {
    this.root.sortableChildren = true;
    this.villageLayer.sortableChildren = true;
  }

  async initialize(onProgress?: (loaded: number, total: number) => void): Promise<VisibleWorldDiagnostics> {
    const manifest = await loadVerifiedVisibleWorldRelease(fetch, onProgress);
    for (const bundle of manifest.actors) this.bundles.set(bundle.family, bundle);

    await this.addScenePieces([
      ...manifest.village.tiles,
      ...runtimeScenePieces(manifest.village.foreground),
      ...manifest.village.decorations,
    ]);
    if (manifest.village.buildings?.length) await this.loadTownBuildingTemplates(manifest.village.buildings);
    this.root.addChild(this.villageLayer);
    this.setMode(this.mode);
    return { fixture: manifest.runtimeDiagnostics.fixture, unresolved: manifest.runtimeDiagnostics.unresolved };
  }

  private async addScenePieces(pieces: Array<{ publicPath: string; x: number; y: number; z?: number; scale?: number; anchor?: { x?: number; y?: number } }>): Promise<void> {
    await Promise.all(pieces.map(async (piece) => {
      const texture = await Assets.load<Texture>(piece.publicPath);
      const sprite = new Sprite(texture);
      const position = projectScenePoint(piece.x, piece.y);
      sprite.anchor.set(piece.anchor?.x ?? 0.5, piece.anchor?.y ?? 0.5);
      sprite.position.set(position.x, position.y);
      sprite.scale.set(piece.scale ?? 1);
      sprite.zIndex = sceneDepthFromUnityZ(piece.z ?? 499);
      this.villageLayer.addChild(sprite);
    }));
  }

  private async loadTownBuildingTemplates(buildings: TownBuilding[]): Promise<void> {
    await Promise.all(buildings.map(async (building) => {
      try {
        const texture = await Assets.load<Texture>(building.publicPath);
        this.buildingTemplates.set(building.id, { visual: building, texture });
      } catch (error) {
        console.warn(`Could not load town building ${building.publicPath}.`, error);
      }
    }));
  }

  setMode(mode: "village" | "field"): void {
    if (mode === this.mode) return;
    this.mode = mode;
    const focus = mode === "village" ? TOWN_CAMERA_CENTER : FIELD_CAMERA_CENTER;
    this.cameraX = focus.x;
    this.cameraY = focus.y;
    this.cameraZoom = mode === "village" ? TOWN_CAMERA_ZOOM : FIELD_CAMERA_ZOOM;
    this.applyCamera();
  }

  setBuildingPresentation(instances: BuildingInstanceSnapshot[], resolvedSpriteIds: Iterable<string>): void {
    const resolved = new Set(resolvedSpriteIds);
    const placements = projectRenderableBuildingInstances(instances.map((instance) => ({
      instanceId: instance.instance_id,
      buildingId: instance.building_id,
      spriteAssetId: instance.sprite_asset_id,
      gridX: instance.grid_x,
      gridY: instance.grid_y,
      width: instance.grid_width,
      height: instance.grid_height,
    })), resolved, BUILDING_GRID_CELL_WIDTH, BUILDING_GRID_CELL_HEIGHT, BUILDING_GRID_ORIGIN_X, BUILDING_GRID_ORIGIN_Y);
    const active = new Set<string>();
    this.buildingInstances.clear();
    for (const instance of instances) this.buildingInstances.set(instance.instance_id, instance);
    this.buildingFootprints = [];

    for (const placement of placements) {
      const template = placement.spriteAssetId ? this.buildingTemplates.get(placement.spriteAssetId) : null;
      if (!template) continue;
      active.add(placement.instanceId);
      let sprite = this.buildingSprites.get(placement.instanceId);
      if (!sprite) {
        sprite = new Sprite(template.texture);
        sprite.eventMode = "static";
        sprite.cursor = "pointer";
        sprite.on("pointertap", () => {
          const current = this.buildingInstances.get(placement.instanceId);
          const currentTemplate = current?.sprite_asset_id ? this.buildingTemplates.get(current.sprite_asset_id) : null;
          if (current && currentTemplate) this.onBuildingSelect?.(current, currentTemplate.visual);
        });
        this.villageLayer.addChild(sprite);
        this.buildingSprites.set(placement.instanceId, sprite);
      }
      sprite.texture = template.texture;
      sprite.anchor.set(template.visual.anchor.x, template.visual.anchor.y);
      sprite.position.set(placement.x, placement.y);
      sprite.scale.set(template.visual.scale);
      sprite.zIndex = villageActorDepth(placement.y, SCENE_WORLD_HEIGHT);
      sprite.visible = true;
      this.buildingFootprints.push({
        id: placement.instanceId,
        x: placement.x,
        y: placement.y,
        halfWidth: template.texture.width * template.visual.scale * 0.32,
        top: placement.y - template.texture.height * template.visual.scale * 0.72,
      });
    }
    for (const [instanceId, sprite] of this.buildingSprites) {
      if (active.has(instanceId)) continue;
      sprite.destroy();
      this.buildingSprites.delete(instanceId);
    }
  }

  resize(width: number, height: number): void {
    this.viewportWidth = width;
    this.viewportHeight = height;
    this.applyCamera();
  }

  panBy(screenDeltaX: number, screenDeltaY: number): void {
    const scale = Math.max(0.001, this.root.scale.x);
    this.cameraX -= screenDeltaX / scale;
    this.cameraY -= screenDeltaY / scale;
    this.applyCamera();
  }

  zoomBy(delta: number): void {
    this.cameraZoom = Math.max(1, Math.min(1.8, this.cameraZoom + delta));
    this.applyCamera();
  }

  private applyCamera(): void {
    const transform = panWorldViewport(
      this.viewportWidth,
      this.viewportHeight,
      SCENE_WORLD_WIDTH,
      SCENE_WORLD_HEIGHT,
      this.cameraX,
      this.cameraY,
      this.cameraZoom,
    );
    this.root.scale.set(transform.scale);
    this.root.position.set(transform.x, transform.y);
  }

  update(entities: WorldEntityProjection[], visualTick: number, receivedAtMs = performance.now()): void {
    this.projectionBuffer.push(this.mode, visualTick, entities, receivedAtMs);
    const sample = this.projectionBuffer.sample(receivedAtMs);
    if (sample) this.applyProjection(sample.entities);
  }

  tick(nowMs = performance.now()): void {
    const sample = this.projectionBuffer.sample(nowMs);
    if (sample) this.applyProjection(sample.entities);
  }

  private applyProjection(entities: WorldEntityProjection[]): void {
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

  destroy(): void {
    this.root.destroy({ children: true });
    this.views.clear();
    this.pending.clear();
    this.projectionBuffer.reset();
  }

  private async create(entity: WorldEntityProjection): Promise<void> {
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
      const skinSignature = this.applyProjectionVisualSkin(spine, family, current);
      const root = new Container();
      root.eventMode = current.selectable ? "static" : "none";
      root.cursor = current.selectable ? "pointer" : "default";
      root.on("pointertap", () => this.onSelect(id));
      root.addChild(spine);
      this.activeLayer().addChild(root);
      const view = { root, spine, animation: "", family, skinSignature };
      root.position.set(current.x, current.y);
      this.views.set(id, view);
      this.project(view, current);
    } catch (error) {
      console.warn(`Could not render authoritative entity ${id}.`, error);
    } finally {
      this.pending.delete(id);
    }
  }

  private project(view: ActorView, entity: WorldEntityProjection): void {
    const projected = projectNormalizedEntityPoint(this.mode, entity.x, entity.y);
    const position = this.resolveActorPosition(projected.x, projected.y);
    view.root.position.set(position.x, position.y);
    view.root.zIndex = villageActorDepth(position.y, SCENE_WORLD_HEIGHT);
    view.root.eventMode = entity.selectable ? "static" : "none";
    view.root.cursor = entity.selectable ? "pointer" : "default";
    const direction = entity.facing === "left" ? -1 : 1;
    const actorScale: Record<string, number> = {
      hunter: 1.02,
      Chief: 1.08,
      Npc: 0.80,
      npc_animal: 0.68,
      pet: 0.58,
      mon_goldblin: 0.88,
      mon_a_01_1: 0.88,
    };
    const scale = actorScale[view.family] ?? 0.72;
    view.spine.scale.set(scale * direction, scale);
    if (view.family === "hunter") {
      const visual = hunterActorVisual(entity);
      view.root.tint = visual.tint;
      if (visual.signature !== view.skinSignature) view.skinSignature = this.applyProjectionVisualSkin(view.spine, view.family, entity);
    }
    const requestedAnimation = view.family === "hunter" ? hunterActorVisual(entity).animation ?? entity.animation : entity.animation;
    if (view.animation === requestedAnimation) return;
    if (view.spine.skeleton.data.findAnimation(requestedAnimation)) {
      view.spine.state.setAnimation(0, requestedAnimation, true);
      view.animation = requestedAnimation;
    } else {
      console.warn(`Animation ${requestedAnimation} is missing from ${view.family}; setup pose retained.`);
      view.spine.state.clearTrack(0);
      view.animation = "";
    }
  }

  private resolveActorPosition(x: number, y: number): { x: number; y: number } {
    for (const footprint of this.buildingFootprints) {
      if (!this.buildingSprites.has(footprint.id)) continue;
      if (Math.abs(x - footprint.x) > footprint.halfWidth || y < footprint.top || y > footprint.y + 24) continue;
      // Keep actors on a walkable lane in front of or behind a building, never inside its body.
      const behind = footprint.top - 18;
      const inFront = footprint.y + 28;
      y = Math.abs(y - behind) < Math.abs(y - inFront) ? behind : inFront;
    }
    return { x, y };
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

  private applyProjectionVisualSkin(spine: Spine, family: string, entity: WorldEntityProjection): string {
    const candidates: Record<string, string[]> = {
      hunter: ["All_h1"],
      Chief: ["chief_body_01", "cos_01"],
      Npc: ["npc_01"],
      npc_animal: ["1"],
      mon_goldblin: ["lv1"],
      mon_a_01_1: ["lv1"],
    };
    const projected = family === "hunter" ? hunterActorVisual(entity) : null;
    const skinNames = projected?.skinNames ?? candidates[family] ?? [];
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
    return projected?.signature ?? skinNames.join("|");
  }

  private activeLayer(): Container {
    return this.villageLayer;
  }
}
