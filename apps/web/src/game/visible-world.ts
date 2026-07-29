import { Spine } from "@esotericsoftware/spine-pixi-v8";
import { Skin } from "@esotericsoftware/spine-core";
import { Assets, Circle, Container, Graphics, Sprite, Text, Texture } from "pixi.js";
import { loadVerifiedVisibleWorldRelease, type ActorBundle, type MonsterDensitySignboard, type TownBuilding } from "../assets/visible-world-release";
import type { BuildingInstanceSnapshot, CombatPresentationSnapshot, WorldDropProjection, WorldEntityProjection } from "../generated/protocol";
import { projectRenderableBuildingInstances } from "./building-placement";
import { panWorldViewport, worldPointVisible } from "./camera";
import { sceneDepthFromUnityZ, scenePieceDepth, villageActorDepth, villageBuildingDepth } from "./depth";
import { ProjectionBuffer } from "./projection-interpolation";
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
import { hunterActorVisual } from "../ui/hunter-roster";
import { applyHunterSpineSkin } from "../ui/hunter-spine-presentation";
import {
  COMBAT_CRITICAL_AMOUNT_OFFSET_Y,
  COMBAT_CRITICAL_LABEL_OFFSET_Y,
  COMBAT_CRITICAL_LABEL_SIZE_PX,
  COMBAT_DAMAGE_FONT_SIZE_PX,
  ORIGINAL_CRITICAL_LABEL_COLOR,
  ORIGINAL_DAMAGE_FONT_FAMILY,
  ORIGINAL_DAMAGE_TEXT_IDEAL_DURATION_MS,
  ORIGINAL_EVADE_COLOR,
  ORIGINAL_INCOMING_DAMAGE_COLOR,
  ORIGINAL_MISS_COLOR,
  ORIGINAL_NORMAL_DAMAGE_COLOR,
  REBUILD_EXPERIENCE_COLOR,
  combatPresentationHasValidPayload,
  combatPresentationText,
  originalDamageMotionAt,
} from "./combat-presentation";
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
}
interface ActorHealthBarView { root: Container; fill: Sprite; }
interface BuildingFootprint { id: string; x: number; y: number; halfWidth: number; top: number; }
interface BuildingTemplate { visual: TownBuilding; texture: Texture; }
interface SignboardView { root: Container; sprite: Sprite; textures: Map<number, Texture>; densityLevel: number; }
interface RangedProjectileView { sprite: Sprite; start: { x: number; y: number }; end: { x: number; y: number }; spawnedAtMs: number; }
interface PendingCombatPresentation { event: CombatPresentationSnapshot; receivedAtMs: number; }
interface CombatPresentationView { root: Container; originY: number; spawnedAtMs: number; }
interface DropView { root: Container; sprite: Sprite; quantity: Text; iconPath: string; }
interface LootPickupView { root: Container; spawnedAtMs: number; }

export function groundDropIconScale(itemId: string): number {
  return itemId === "gold" ? 0.55 : 0.72;
}

export class VisibleEntityWorld {
  readonly root = new Container();
  private readonly views = new Map<string, ActorView>();
  private readonly pending = new Set<string>();
  private readonly familyLoads = new Map<string, Promise<void>>();
  private readonly bundles = new Map<string, ActorBundle>();
  private readonly projectionBuffer = new ProjectionBuffer({
    tickDurationMs: 200,
    renderDelayMs: 200,
    maxExtrapolationTicks: 1,
  });
  private buildingFootprints: BuildingFootprint[] = [];
  private readonly buildingTemplates = new Map<string, BuildingTemplate>();
  private readonly buildingInstances = new Map<string, BuildingInstanceSnapshot>();
  private readonly buildingSprites = new Map<string, Sprite>();
  private readonly worldLayer = new Container();
  private readonly staticLayer = new Container();
  private readonly signboards = new Map<string, SignboardView>();
  private readonly rangedProjectiles: RangedProjectileView[] = [];
  private readonly pendingCombatPresentations: PendingCombatPresentation[] = [];
  private readonly combatPresentationViews: CombatPresentationView[] = [];
  private readonly seenCombatPresentationSequences = new Set<number>();
  private readonly dropViews = new Map<string, DropView>();
  private readonly pendingDrops = new Set<string>();
  private readonly lootPickupViews: LootPickupView[] = [];
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
    this.queueCombatPresentations(combatPresentations, receivedAtMs);
    const sample = this.projectionBuffer.sample(receivedAtMs);
    if (sample) this.applyProjection(sample.entities);
    this.applyDrops(drops);
    this.spawnPendingCombatPresentations(receivedAtMs);
  }

  tick(nowMs = performance.now()): void {
    const sample = this.projectionBuffer.sample(nowMs);
    if (sample) this.applyProjection(sample.entities);
    this.updateRangedProjectiles(nowMs);
    this.updateLootPickupViews(nowMs);
    this.spawnPendingCombatPresentations(nowMs);
    this.updateCombatPresentationViews(nowMs);
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
    this.pendingCombatPresentations.length = 0;
    this.combatPresentationViews.length = 0;
    this.seenCombatPresentationSequences.clear();
    this.dropViews.clear();
    this.pendingDrops.clear();
    this.lootPickupViews.length = 0;
    this.projectionBuffer.reset();
  }

  private applyDrops(drops: WorldDropProjection[]): void {
    const active = new Set(drops.map((drop) => drop.drop_id));
    for (const drop of drops) {
      const view = this.dropViews.get(drop.drop_id);
      if (view) {
        view.root.position.set(drop.x, drop.y - 14);
        view.root.zIndex = villageActorDepth(drop.y, SCENE_WORLD_HEIGHT) - 1;
        view.quantity.text = drop.quantity > 1 ? `x${drop.quantity}` : "";
      } else if (!this.pendingDrops.has(drop.drop_id) && drop.icon_path) {
        void this.createDrop(drop);
      }
    }
    for (const [dropId, view] of this.dropViews) {
      if (!active.has(dropId)) {
        view.root.destroy({ children: true });
        this.dropViews.delete(dropId);
      }
    }
  }

  private async createDrop(drop: WorldDropProjection): Promise<void> {
    this.pendingDrops.add(drop.drop_id);
    try {
      const texture = await Assets.load<Texture>(drop.icon_path);
      const root = new Container();
      const sprite = new Sprite(texture);
      sprite.anchor.set(0.5);
      sprite.scale.set(groundDropIconScale(drop.item_id));
      if (drop.item_id === "gold") {
        // The only confirmed gold sprite is small HUD art; layer copies into a
        // readable ground pile without claiming a missing original drop asset.
        const leftCoin = new Sprite(texture);
        leftCoin.anchor.set(0.5);
        leftCoin.position.set(-6, 4);
        leftCoin.scale.set(0.42);
        const rightCoin = new Sprite(texture);
        rightCoin.anchor.set(0.5);
        rightCoin.position.set(6, 4);
        rightCoin.scale.set(0.42);
        root.addChild(leftCoin, rightCoin);
      }
      const quantity = new Text({
        text: drop.quantity > 1 ? `x${drop.quantity}` : "",
        style: { fontFamily: ORIGINAL_DAMAGE_FONT_FAMILY, fontSize: 12, fill: 0xffefba, stroke: { color: 0x261d0d, width: 3 } },
      });
      quantity.anchor.set(0.5, 0);
      quantity.position.set(0, 10);
      root.addChild(sprite, quantity);
      root.position.set(drop.x, drop.y - 14);
      root.zIndex = villageActorDepth(drop.y, SCENE_WORLD_HEIGHT) - 1;
      this.worldLayer.addChild(root);
      this.dropViews.set(drop.drop_id, { root, sprite, quantity, iconPath: drop.icon_path });
    } finally {
      this.pendingDrops.delete(drop.drop_id);
    }
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
    // The recovered Hunter setup pose faces left; mirror it only for right-facing movement.
    // Hunter and monster Spine setup poses both face left. Their authoritative
    // left/right value therefore uses the same mirror inversion.
    const direction = view.family === "hunter" || entity.descriptor.kind === "monster"
      ? (entity.facing === "left" ? 1 : -1)
      : (entity.facing === "left" ? -1 : 1);
    const actorScale: Record<string, number> = {
      hunter: 1.02,
      Chief: 1.08,
      Npc: 0.80,
      npc_animal: 0.68,
      pet: 0.58,
      mon_goldblin: 1.15,
      mon_a_01_1: 1.15,
    };
    const scale = actorScale[view.family] ?? 0.72;
    view.spine.scale.set(scale * direction, scale);
    if (view.family === "hunter") {
      const visual = hunterActorVisual(entity);
      view.spine.tint = visual.tint;
      if (visual.signature !== view.skinSignature) view.skinSignature = this.applyProjectionVisualSkin(view.spine, view.family, entity);
    }
    this.projectHealthBar(view.healthBar, entity);
    this.maybeStartRangerProjectile(view, entity);
    if (entity.loot_sequence > view.lootSequence && entity.loot_label) {
      view.lootSequence = entity.loot_sequence;
      this.showLootPickup(view, entity.loot_label);
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

  private showLootPickup(view: ActorView, label: string): void {
    const root = new Container();
    const text = new Text({
      text: label,
      style: {
        fontFamily: ORIGINAL_DAMAGE_FONT_FAMILY,
        fontSize: 14,
        fill: 0xffefba,
        stroke: { color: 0x261d0d, width: 4 },
      },
    });
    text.anchor.set(0.5);
    root.addChild(text);
    root.position.set(0, -72);
    view.root.addChild(root);
    this.lootPickupViews.push({ root, spawnedAtMs: performance.now() });
  }

  private updateLootPickupViews(nowMs: number): void {
    for (let index = this.lootPickupViews.length - 1; index >= 0; index -= 1) {
      const pickup = this.lootPickupViews[index]!;
      const elapsed = nowMs - pickup.spawnedAtMs;
      pickup.root.y = -72 - Math.min(18, elapsed * 0.018);
      pickup.root.alpha = Math.max(0, 1 - Math.max(0, elapsed - 700) / 500);
      if (elapsed < 1_200) continue;
      pickup.root.destroy({ children: true });
      this.lootPickupViews.splice(index, 1);
    }
  }

  private queueCombatPresentations(events: CombatPresentationSnapshot[], receivedAtMs: number): void {
    for (const event of events) {
      if (this.seenCombatPresentationSequences.has(event.sequence)) continue;
      this.seenCombatPresentationSequences.add(event.sequence);
      if (!combatPresentationHasValidPayload(event.kind, event.amount)) continue;
      this.pendingCombatPresentations.push({ event, receivedAtMs });
    }
    if (this.seenCombatPresentationSequences.size > 512) {
      const newest = [...this.seenCombatPresentationSequences].sort((a, b) => b - a).slice(0, 256);
      this.seenCombatPresentationSequences.clear();
      for (const sequence of newest) this.seenCombatPresentationSequences.add(sequence);
    }
  }

  private spawnPendingCombatPresentations(nowMs: number): void {
    for (let index = this.pendingCombatPresentations.length - 1; index >= 0; index -= 1) {
      const pending = this.pendingCombatPresentations[index]!;
      const target = this.views.get(pending.event.target_entity_id);
      if (!target) {
        if (nowMs - pending.receivedAtMs < ORIGINAL_DAMAGE_TEXT_IDEAL_DURATION_MS) continue;
        this.pendingCombatPresentations.splice(index, 1);
        continue;
      }
      const root = this.createCombatPresentationView(pending.event);
      const originY = target.root.position.y - 52;
      root.position.set(target.root.position.x, originY);
      root.zIndex = villageActorDepth(target.root.position.y, SCENE_WORLD_HEIGHT) + 0.1;
      this.worldLayer.addChild(root);
      this.combatPresentationViews.push({ root, originY, spawnedAtMs: nowMs });
      this.pendingCombatPresentations.splice(index, 1);
    }
  }

  private createCombatPresentationView(event: CombatPresentationSnapshot): Container {
    const root = new Container();
    const lines = combatPresentationText(event);
    if (event.kind === "critical_damage") {
      const label = this.createCombatText(lines[0]!, COMBAT_CRITICAL_LABEL_SIZE_PX, ORIGINAL_CRITICAL_LABEL_COLOR);
      const amount = this.createCombatText(lines[1]!, COMBAT_DAMAGE_FONT_SIZE_PX, ORIGINAL_NORMAL_DAMAGE_COLOR);
      label.position.y = COMBAT_CRITICAL_LABEL_OFFSET_Y;
      amount.position.y = COMBAT_CRITICAL_AMOUNT_OFFSET_Y;
      root.addChild(label, amount);
      return root;
    }
    const color = event.kind === "evade"
      ? ORIGINAL_EVADE_COLOR
      : event.kind === "miss"
        ? ORIGINAL_MISS_COLOR
        : event.kind === "experience"
          ? REBUILD_EXPERIENCE_COLOR
        : event.kind === "incoming_damage"
          ? ORIGINAL_INCOMING_DAMAGE_COLOR
          : ORIGINAL_NORMAL_DAMAGE_COLOR;
    root.addChild(this.createCombatText(lines[0]!, COMBAT_DAMAGE_FONT_SIZE_PX, color));
    return root;
  }

  private createCombatText(value: string, fontSize: number, fill: number): Text {
    const text = new Text({
      text: value,
      style: {
        fontFamily: ORIGINAL_DAMAGE_FONT_FAMILY,
        fontSize,
        fill,
        align: "center",
      },
    });
    text.anchor.set(0.5);
    return text;
  }

  private updateCombatPresentationViews(nowMs: number): void {
    for (let index = this.combatPresentationViews.length - 1; index >= 0; index -= 1) {
      const view = this.combatPresentationViews[index]!;
      const motion = originalDamageMotionAt(nowMs - view.spawnedAtMs);
      view.root.position.y = view.originY - motion.yOffset;
      view.root.scale.set(motion.scale);
      if (!motion.done) continue;
      view.root.destroy({ children: true });
      this.combatPresentationViews.splice(index, 1);
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
      hunter: ["All_h1"],
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
