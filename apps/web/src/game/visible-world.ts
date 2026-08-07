import { Spine } from "@esotericsoftware/spine-pixi-v8";
import { Skin } from "@esotericsoftware/spine-core";
import { Assets, Circle, Container, Graphics, Sprite, Texture } from "pixi.js";
import { loadVerifiedVisibleWorldRelease, type ActorBundle, type MonsterDensitySignboard, type TownBuilding } from "../assets/visible-world-release";
import type { BuildingInstanceSnapshot, CombatPresentationSnapshot, WorldDropProjection, WorldEntityProjection } from "../generated/protocol";
import { projectRenderableBuildingInstances } from "./building-placement";
import { panWorldViewport, worldPointVisible } from "./camera";
import { sceneDepthFromUnityZ, scenePieceDepth, villageActorDepth, villageBuildingDepth } from "./depth";
import { ProjectionBuffer, type ProjectionDriftDiagnostic } from "./projection-interpolation";
import {
  RANGER_PROJECTILE_SCALE,
  rangerProjectileOrigin,
  rangerProjectilePose,
  shouldStartRangerProjectile,
} from "./ranged-attack-presentation";
import {
  ACTOR_HP_BACKGROUND_COLOR,
  ACTOR_HP_FRAME_ASSET,
  ACTOR_HP_INNER_ASSET,
  actorHealthBarLayout,
  actorHealthPresentation,
} from "./actor-health-bar";
import { hunterActorVisual } from "./hunter-actor-presentation";
import { applyHunterSpineSkin } from "./hunter-spine-presentation";
import { ORIGINAL_DAMAGE_FONT_FAMILY } from "./combat-presentation";
import { WorldEffects } from "./world-effects";
import { actorScaleForFamily, facingScale } from "./actor-presentation";
export { groundDropIconScale, tradeSettlementText } from "./world-effects";
import {
  FIELD_CAMERA_CENTER,
  FIELD_CAMERA_ZOOM,
  MAX_CAMERA_ZOOM,
  MIN_CAMERA_ZOOM,
  projectScenePoint,
  projectWorldEntityPoint,
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
  presence: Graphics;
  highlight: Graphics;
  healthBar: ActorHealthBarView | null;
  animation: string;
  family: string;
  skinSignature: string;
  actionSequence: number;
  lootSequence: number;
  tradeSequence: number;
  speechLabel: string | null;
}
interface ActorHealthBarView { root: Container; fill: Sprite; }
interface BuildingFootprint { id: string; x: number; y: number; halfWidth: number; top: number; }
interface BuildingTemplate { visual: TownBuilding; texture: Texture; }
interface SignboardView { root: Container; sprite: Sprite; textures: Map<number, Texture>; densityLevel: number; }
interface RangedProjectileView { sprite: Sprite; start: { x: number; y: number }; end: { x: number; y: number }; spawnedAtMs: number; }

export class VisibleEntityWorld {
  readonly root = new Container();
  private readonly views = new Map<string, ActorView>();
  private readonly pending = new Set<string>();
  private readonly familyLoads = new Map<string, Promise<void>>();
  private readonly bundles = new Map<string, ActorBundle>();
  private readonly projectionBuffer = new ProjectionBuffer({
    // WorldFrame is published at the authoritative 10 Hz simulation cadence.
    // A stale 200 ms clock rendered movement at half speed and periodically
    // jumped forward even while the Pixi ticker correctly reported 60 FPS.
    tickDurationMs: 100,
    renderDelayMs: 100,
    maxExtrapolationTicks: 1,
    visualDriftWarningTicks: 3,
    onVisualDrift: (diagnostic) => this.onVisualDrift?.(diagnostic),
  });
  private buildingFootprints: BuildingFootprint[] = [];
  private readonly buildingTemplates = new Map<string, BuildingTemplate>();
  private readonly buildingInstances = new Map<string, BuildingInstanceSnapshot>();
  private readonly buildingSprites = new Map<string, Sprite>();
  private readonly worldLayer = new Container();
  private readonly staticLayer = new Container();
  private readonly signboards = new Map<string, SignboardView>();
  private readonly rangedProjectiles: RangedProjectileView[] = [];
  private readonly effects = new WorldEffects(
    this.worldLayer,
    (entityId) => this.views.get(entityId)?.root ?? null,
  );
  private rangerArrowTexture: Texture | null = null;
  private actorHpInnerTexture: Texture | null = null;
  private actorHpFrameTexture: Texture | null = null;
  private mode: "village" | "field" = "village";
  private latest: WorldEntityProjection[] = [];
  private viewportWidth = 1;
  private viewportHeight = 1;
  private cameraX: number = TOWN_CAMERA_CENTER.x;
  private cameraY: number = TOWN_CAMERA_CENTER.y;
  private cameraZoom = TOWN_CAMERA_ZOOM;
  private selectedEntityId: string | null = null;

  constructor(
    private readonly onSelect: (entityId: string, screenPoint?: { x: number; y: number }) => void,
    private readonly onBuildingSelect?: (instance: BuildingInstanceSnapshot, visual: TownBuilding) => void,
    private readonly onDensityCycle?: (regionId: string, nextLevel: number) => void,
    private readonly onVisualDrift?: (diagnostic: ProjectionDriftDiagnostic) => void,
  ) {
    this.root.sortableChildren = true;
    this.worldLayer.sortableChildren = true;
    this.staticLayer.sortableChildren = true;
  }

  async initialize(onProgress?: (loaded: number, total: number) => void): Promise<VisibleWorldDiagnostics> {
    const manifest = await loadVerifiedVisibleWorldRelease(fetch, onProgress);
    for (const bundle of manifest.actors) this.bundles.set(bundle.family, bundle);

    await this.addScenePieces(manifest.village.tiles, this.staticLayer);
    await this.addScenePieces([
      ...runtimeScenePieces(manifest.village.foreground),
      ...manifest.village.decorations,
    ]);
    if (manifest.village.buildings?.length) await this.loadTownBuildingTemplates(manifest.village.buildings);
    await this.loadMonsterDensitySignboards(manifest.village.signboards);
    const damageFont = new FontFace(
      ORIGINAL_DAMAGE_FONT_FAMILY,
      "url('/content/releases/original-flow-v1/fonts/DefaultFont2__197.ttf')",
    );
    document.fonts.add(await damageFont.load());
    [this.rangerArrowTexture, this.actorHpInnerTexture, this.actorHpFrameTexture] = await Promise.all([
      Assets.load<Texture>("/content/releases/original-flow-v1/sprites/atk_ranger__3599.png"),
      Assets.load<Texture>(ACTOR_HP_INNER_ASSET),
      Assets.load<Texture>(ACTOR_HP_FRAME_ASSET),
    ]);
    this.root.addChild(this.staticLayer, this.worldLayer);
    // The large town and farm backgrounds never change during a session. Bake
    // only those tiles; foreground pieces keep dynamic depth sorting.
    this.staticLayer.cacheAsTexture({ antialias: false });
    return { fixture: manifest.runtimeDiagnostics.fixture, unresolved: manifest.runtimeDiagnostics.unresolved };
  }
  private async addScenePieces(
    pieces: Array<{ id?: string; publicPath: string; x: number; y: number; z?: number; scale?: number; anchor?: { x?: number; y?: number } }>,
    target = this.worldLayer,
  ): Promise<void> {
    await Promise.all(pieces.map(async (piece) => {
      const texture = await Assets.load<Texture>(piece.publicPath);
      const sprite = new Sprite(texture);
      const position = projectScenePoint(piece.x, piece.y);
      sprite.anchor.set(piece.anchor?.x ?? 0.5, piece.anchor?.y ?? 0.5);
      sprite.position.set(position.x, position.y);
      sprite.scale.set(piece.scale ?? 1);
      sprite.zIndex = scenePieceDepth(piece.id, piece.z ?? 499);
      target.addChild(sprite);
    }));
  }
  private async loadTownBuildingTemplates(buildings: TownBuilding[]): Promise<void> {
    await Promise.all(buildings.map(async (building) => {
      const texture = await Assets.load<Texture>(building.publicPath);
      this.buildingTemplates.set(building.id, { visual: building, texture });
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
  setMonsterDensityLevels(fields: ReadonlyArray<{ id: string; densityLevel: number }>): void {
    for (const field of fields) {
      const view = this.signboards.get(field.id);
      const texture = view?.textures.get(field.densityLevel);
      if (!view || !texture) continue;
      view.densityLevel = field.densityLevel;
      view.sprite.texture = texture;
    }
  }

  setBuildingPresentation(instances: BuildingInstanceSnapshot[]): void {
    // The visible-world release already checksum-pins every renderable building
    // template. Do not make town rendering wait for the much larger UI registry.
    const publishedVisuals = new Set(this.buildingTemplates.keys());
    const placements = projectRenderableBuildingInstances(instances.map((instance) => ({
      instanceId: instance.instance_id,
      buildingId: instance.building_id,
      spriteAssetId: instance.sprite_asset_id,
      gridX: instance.grid_x,
      gridY: instance.grid_y,
      width: instance.grid_width,
      height: instance.grid_height,
    })), publishedVisuals, BUILDING_GRID_CELL_WIDTH, BUILDING_GRID_CELL_HEIGHT, BUILDING_GRID_ORIGIN_X, BUILDING_GRID_ORIGIN_Y);
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
        this.worldLayer.addChild(sprite);
        this.buildingSprites.set(placement.instanceId, sprite);
      }
      sprite.texture = template.texture;
      sprite.anchor.set(template.visual.anchor.x, template.visual.anchor.y);
      sprite.position.set(placement.x, placement.y);
      sprite.scale.set(template.visual.scale);
      sprite.zIndex = villageBuildingDepth(placement.y, SCENE_WORLD_HEIGHT);
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
    this.cameraZoom = Math.max(MIN_CAMERA_ZOOM, Math.min(MAX_CAMERA_ZOOM, this.cameraZoom + delta));
    this.applyCamera();
  }

  setSelectedEntity(entityId: string | null): void {
    this.selectedEntityId = entityId;
    for (const [id, view] of this.views) view.highlight.visible = id === entityId;
  }

  screenPointForEntity(entityId: string): { x: number; y: number } | null {
    const view = this.views.get(entityId);
    if (!view?.root.renderable) return null;
    return {
      x: this.root.position.x + view.root.x * this.root.scale.x,
      y: this.root.position.y + view.root.y * this.root.scale.y,
    };
  }

  focusEntity(entityId: string): boolean {
    const entity = this.latest.find((candidate) => candidate.descriptor.entity_id === entityId);
    if (!entity) return false;
    const projected = projectWorldEntityPoint(entity.x, entity.y);
    const position = entity.descriptor.kind === "hunter"
      ? projected
      : this.resolveActorPosition(projected.x, projected.y);
    this.cameraX = position.x;
    this.cameraY = position.y;
    this.setSelectedEntity(entityId);
    this.applyCamera();
    return true;
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
    this.updateActorVisibility(transform);
  }

  private updateActorVisibility(transform = {
    scale: this.root.scale.x,
    x: this.root.position.x,
    y: this.root.position.y,
  }): void {
    for (const view of this.views.values()) {
      const visible = worldPointVisible(
        view.root.x,
        view.root.y,
        this.viewportWidth,
        this.viewportHeight,
        transform,
      );
      view.root.renderable = visible;
      view.spine.state.timeScale = visible ? 1 : 0;
    }
  }

  update(
    entities: WorldEntityProjection[],
    visualTick: number,
    combatPresentations: CombatPresentationSnapshot[] = [],
    drops: WorldDropProjection[] = [],
    receivedAtMs = performance.now(),
  ): void {
    this.projectionBuffer.push(this.mode, visualTick, entities, receivedAtMs);
    const sample = this.projectionBuffer.sample(receivedAtMs);
    if (sample) this.applyProjection(sample.entities);
    this.effects.update(combatPresentations, drops, receivedAtMs);
  }

  tick(nowMs = performance.now()): void {
    const sample = this.projectionBuffer.sample(nowMs);
    if (sample) this.applyProjection(sample.entities);
    this.updateRangedProjectiles(nowMs);
    this.effects.tick(nowMs);
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
    this.updateActorVisibility();
  }

  destroy(): void {
    this.root.destroy({ children: true });
    this.views.clear();
    this.pending.clear();
    this.rangedProjectiles.length = 0;
    this.effects.destroy();
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
      const presence = new Graphics().ellipse(0, 2, 25, 10).fill({ color: 0x321914, alpha: 0.42 });
      presence.visible = current.descriptor.kind === "monster";
      const highlight = new Graphics().ellipse(0, 2, 27, 12).fill({ color: 0x536b28, alpha: 0.48 }).stroke({ color: 0xe4dc83, width: 2 });
      highlight.visible = id === this.selectedEntityId;
      root.eventMode = current.selectable ? "static" : "none";
      root.cursor = current.selectable ? "pointer" : "default";
      root.on("pointertap", (event) => this.onSelect(id, { x: event.global.x, y: event.global.y }));
      const healthBar = this.createHealthBar(current);
      root.addChild(presence, highlight, spine);
      if (healthBar) root.addChild(healthBar.root);
      this.worldLayer.addChild(root);
      const view = {
        root, spine, presence, highlight, healthBar, animation: "", family, skinSignature,
        actionSequence: 0, lootSequence: current.loot_sequence,
        tradeSequence: current.trade_sequence, speechLabel: null,
      };
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
    const projected = projectWorldEntityPoint(entity.x, entity.y);
    const position = entity.descriptor.kind === "hunter"
      ? projected
      : this.resolveActorPosition(projected.x, projected.y);
    // Walking positions already come from the client-side render timeline.
    // Applying another target blend here makes every server confirmation pull
    // actors backward and makes animation speed lag behind their route.
    view.root.position.set(position.x, position.y);
    view.root.zIndex = villageActorDepth(position.y, SCENE_WORLD_HEIGHT);
    view.presence.visible = entity.descriptor.kind === "monster";
    view.root.eventMode = entity.selectable ? "static" : "none";
    view.root.cursor = entity.selectable ? "pointer" : "default";
    view.spine.scale.set(actorScaleForFamily(view.family) * facingScale(view.family, entity), actorScaleForFamily(view.family));
    if (view.family === "hunter") {
      const visual = hunterActorVisual(entity);
      view.spine.tint = visual.tint;
      if (visual.signature !== view.skinSignature) view.skinSignature = this.applyProjectionVisualSkin(view.spine, view.family, entity);
    }
    this.projectHealthBar(view.healthBar, entity);
    this.maybeStartRangerProjectile(view, entity);
    if (entity.loot_sequence > view.lootSequence && entity.loot_label) {
      view.lootSequence = entity.loot_sequence;
      this.effects.showLootPickup(view.root, entity.loot_label);
    }
    if (entity.trade_sequence > view.tradeSequence) {
      view.tradeSequence = entity.trade_sequence;
      this.effects.showTradeSettlement(view.root, entity);
    }
    if (entity.speech_label && entity.speech_label !== view.speechLabel) {
      view.speechLabel = entity.speech_label;
      this.effects.showSpeech(view.root, entity.speech_label);
    } else if (!entity.speech_label) {
      view.speechLabel = null;
    }
    const requestedAnimation = view.family === "hunter" ? hunterActorVisual(entity).animation ?? entity.animation : entity.animation;
    if (view.animation === requestedAnimation && view.actionSequence === entity.action_sequence) return;
    if (view.spine.skeleton.data.findAnimation(requestedAnimation)) {
      const loop = entity.action_state !== "attacking" && entity.action_state !== "dead";
      view.spine.state.setAnimation(0, requestedAnimation, loop);
      view.animation = requestedAnimation;
      view.actionSequence = entity.action_sequence;
    } else {
      console.warn(`Animation ${requestedAnimation} is missing from ${view.family}; setup pose retained.`);
      view.spine.state.clearTrack(0);
      view.animation = "";
    }
  }

  private createHealthBar(entity: WorldEntityProjection): ActorHealthBarView | null {
    const layout = actorHealthBarLayout(entity.descriptor.kind);
    if (!layout || !this.actorHpInnerTexture || !this.actorHpFrameTexture) return null;
    const root = new Container();
    root.position.set(0, layout.y);

    const background = new Sprite(this.actorHpInnerTexture);
    background.anchor.set(0, 0.5);
    background.position.x = layout.innerX;
    background.tint = ACTOR_HP_BACKGROUND_COLOR;

    const fill = new Sprite(this.actorHpInnerTexture);
    fill.anchor.set(0, 0.5);
    fill.position.x = layout.innerX;

    const frame = new Sprite(this.actorHpFrameTexture);
    frame.anchor.set(0.5);
    frame.position.x = layout.frameX;
    root.addChild(background, fill, frame);
    return { root, fill };
  }

  private projectHealthBar(view: ActorHealthBarView | null, entity: WorldEntityProjection): void {
    if (!view) return;
    const presentation = actorHealthPresentation(entity);
    view.root.visible = presentation !== null;
    if (!presentation) return;
    view.fill.scale.x = presentation.ratio;
    view.fill.tint = presentation.color;
  }

  private maybeStartRangerProjectile(view: ActorView, entity: WorldEntityProjection): void {
    const previousSequence = view.actionSequence;
    if (!shouldStartRangerProjectile(previousSequence, entity) || !this.rangerArrowTexture) return;
    const target = entity.target_entity_id ? this.views.get(entity.target_entity_id) : null;
    const targetProjection = entity.target_entity_id
      ? this.latest.find((candidate) => candidate.descriptor.entity_id === entity.target_entity_id)
      : null;
    if (!target && !targetProjection) return;
    const start = rangerProjectileOrigin(entity);
    const end = target
      ? { x: target.root.position.x, y: target.root.position.y - 12 }
      : { x: targetProjection!.x, y: targetProjection!.y - 12 };
    const sprite = new Sprite(this.rangerArrowTexture);
    sprite.anchor.set(0.5);
    sprite.scale.set(RANGER_PROJECTILE_SCALE);
    sprite.position.set(start.x, start.y);
    sprite.zIndex = villageActorDepth(start.y, SCENE_WORLD_HEIGHT) + 0.05;
    this.worldLayer.addChild(sprite);
    this.rangedProjectiles.push({ sprite, start, end, spawnedAtMs: performance.now() });
  }

  private updateRangedProjectiles(nowMs: number): void {
    for (let index = this.rangedProjectiles.length - 1; index >= 0; index -= 1) {
      const projectile = this.rangedProjectiles[index]!;
      const pose = rangerProjectilePose(projectile.start, projectile.end, nowMs - projectile.spawnedAtMs);
      projectile.sprite.position.set(pose.x, pose.y);
      projectile.sprite.rotation = pose.rotation;
      projectile.sprite.zIndex = villageActorDepth(pose.y, SCENE_WORLD_HEIGHT) + 0.05;
      if (!pose.done) continue;
      projectile.sprite.destroy();
      this.rangedProjectiles.splice(index, 1);
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

  private async loadMonsterDensitySignboards(signboards: MonsterDensitySignboard[]): Promise<void> {
    await Promise.all(signboards.map(async (signboard) => {
      const textures = new Map<number, Texture>();
      await Promise.all(signboard.states.map(async (state) => {
        textures.set(state.densityLevel, await Assets.load<Texture>(state.publicPath));
      }));
      const initialTexture = textures.get(1);
      if (!initialTexture) throw new Error(`Density I sign texture is missing for ${signboard.regionId}`);
      const root = new Container();
      const sprite = new Sprite(initialTexture);
      sprite.anchor.set(0.5);
      root.addChild(sprite);
      root.eventMode = "static";
      root.cursor = "pointer";
      root.hitArea = new Circle(0, 0, Math.max(28, signboard.colliderRadius * 100));
      root.on("pointertap", () => {
        const current = this.signboards.get(signboard.regionId);
        if (!current) return;
        this.onDensityCycle?.(signboard.regionId, current.densityLevel % 3 + 1);
      });
      const position = projectScenePoint(signboard.x, signboard.y);
      root.position.set(position.x, position.y);
      root.zIndex = sceneDepthFromUnityZ(signboard.z);
      this.worldLayer.addChild(root);
      this.signboards.set(signboard.regionId, { root, sprite, textures, densityLevel: 1 });
    }));
  }

  private applyProjectionVisualSkin(spine: Spine, family: string, entity: WorldEntityProjection): string {
    const candidates: Record<string, string[]> = {
      Chief: ["chief_body_01", "cos_01"],
      Npc: ["npc_01"],
      npc_animal: ["1"],
      mon_goldblin: ["lv1"],
      mon_a_01_1: ["lv1"],
    };
    const projected = family === "hunter" ? hunterActorVisual(entity) : null;
    const skinNames = projected?.skinNames ?? candidates[family] ?? [];
    if (family === "hunter") {
      applyHunterSpineSkin(spine, skinNames, entity.class_family, `visible-world:${family}`);
    } else if (skinNames.length === 1 && spine.skeleton.data.findSkin(skinNames[0])) {
      spine.skeleton.setSkinByName(skinNames[0]);
    } else if (skinNames.length > 1) {
      const composition = new Skin(`visible-world:${family}`);
      for (const skinName of skinNames) {
        const skin = spine.skeleton.data.findSkin(skinName);
        if (skin) composition.addSkin(skin);
      }
      spine.skeleton.setSkin(composition);
    }
    if (family !== "hunter") spine.skeleton.setSlotsToSetupPose();
    return projected?.signature ?? skinNames.join("|");
  }
}
