import { Assets, Container, Sprite, Text, Texture } from "pixi.js";
import type {
  CombatPresentationSnapshot,
  WorldDropProjection,
  WorldEntityProjection,
} from "../generated/protocol";
import { formatNumber, t } from "../i18n";
import { villageActorDepth } from "./depth";
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
import { SCENE_WORLD_HEIGHT } from "./scene-projection";

interface PendingCombatPresentation {
  event: CombatPresentationSnapshot;
  receivedAtMs: number;
}

interface CombatPresentationView {
  root: Container;
  originY: number;
  spawnedAtMs: number;
}

interface DropView {
  root: Container;
  quantity: Text;
}

interface LootPickupView {
  root: Container;
  spawnedAtMs: number;
}

export function groundDropIconScale(itemId: string): number {
  return itemId === "gold" ? 0.55 : 0.72;
}

export function tradeSettlementText(
  settlement: Pick<WorldEntityProjection, "trade_gold" | "trade_materials">,
): string {
  const materials = settlement.trade_materials
    .map((material) => t("world.trade_material_sold", {
      name: material.display_name,
      quantity: formatNumber(material.quantity),
    }))
    .join(" · ");
  return [
    t("world.trade_gold_received", { amount: formatNumber(settlement.trade_gold) }),
    materials,
  ].filter(Boolean).join("\n");
}

export class WorldEffects {
  private readonly pendingCombatPresentations: PendingCombatPresentation[] = [];
  private readonly combatPresentationViews: CombatPresentationView[] = [];
  private readonly seenCombatPresentationSequences = new Set<number>();
  private readonly dropViews = new Map<string, DropView>();
  private readonly pendingDrops = new Set<string>();
  private readonly lootPickupViews: LootPickupView[] = [];

  constructor(
    private readonly worldLayer: Container,
    private readonly actorRoot: (entityId: string) => Container | null,
  ) {}

  update(
    combatPresentations: CombatPresentationSnapshot[],
    drops: WorldDropProjection[],
    receivedAtMs: number,
  ): void {
    this.queueCombatPresentations(combatPresentations, receivedAtMs);
    this.applyDrops(drops);
    this.spawnPendingCombatPresentations(receivedAtMs);
  }

  tick(nowMs: number): void {
    this.updateLootPickupViews(nowMs);
    this.spawnPendingCombatPresentations(nowMs);
    this.updateCombatPresentationViews(nowMs);
  }

  showLootPickup(actorRoot: Container, label: string): void {
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
    actorRoot.addChild(root);
    this.lootPickupViews.push({ root, spawnedAtMs: performance.now() });
  }

  showTradeSettlement(actorRoot: Container, entity: WorldEntityProjection): void {
    this.showLootPickup(actorRoot, tradeSettlementText(entity));
  }

  showSpeech(actorRoot: Container, label: string): void {
    const root = new Container();
    const text = new Text({
      text: label,
      style: {
        fontFamily: ORIGINAL_DAMAGE_FONT_FAMILY,
        fontSize: 12,
        fill: 0xffffff,
        stroke: { color: 0x2b2418, width: 4 },
        wordWrap: true,
        wordWrapWidth: 180,
      },
    });
    text.anchor.set(0.5, 1);
    root.position.set(0, -78);
    root.addChild(text);
    actorRoot.addChild(root);
    this.lootPickupViews.push({ root, spawnedAtMs: performance.now() });
  }

  destroy(): void {
    this.pendingCombatPresentations.length = 0;
    this.combatPresentationViews.length = 0;
    this.seenCombatPresentationSequences.clear();
    this.dropViews.clear();
    this.pendingDrops.clear();
    this.lootPickupViews.length = 0;
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
        for (const x of [-6, 6]) {
          const coin = new Sprite(texture);
          coin.anchor.set(0.5);
          coin.position.set(x, 4);
          coin.scale.set(0.42);
          root.addChild(coin);
        }
      }
      const quantity = new Text({
        text: drop.quantity > 1 ? `x${drop.quantity}` : "",
        style: {
          fontFamily: ORIGINAL_DAMAGE_FONT_FAMILY,
          fontSize: 12,
          fill: 0xffefba,
          stroke: { color: 0x261d0d, width: 3 },
        },
      });
      quantity.anchor.set(0.5, 0);
      quantity.position.set(0, 10);
      root.addChild(sprite, quantity);
      root.position.set(drop.x, drop.y - 14);
      root.zIndex = villageActorDepth(drop.y, SCENE_WORLD_HEIGHT) - 1;
      this.worldLayer.addChild(root);
      this.dropViews.set(drop.drop_id, { root, quantity });
    } finally {
      this.pendingDrops.delete(drop.drop_id);
    }
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

  private queueCombatPresentations(
    events: CombatPresentationSnapshot[],
    receivedAtMs: number,
  ): void {
    for (const event of events) {
      if (this.seenCombatPresentationSequences.has(event.sequence)) continue;
      this.seenCombatPresentationSequences.add(event.sequence);
      if (!combatPresentationHasValidPayload(event.kind, event.amount)) continue;
      this.pendingCombatPresentations.push({ event, receivedAtMs });
    }
    if (this.seenCombatPresentationSequences.size > 512) {
      const newest = [...this.seenCombatPresentationSequences]
        .sort((a, b) => b - a)
        .slice(0, 256);
      this.seenCombatPresentationSequences.clear();
      for (const sequence of newest) this.seenCombatPresentationSequences.add(sequence);
    }
  }

  private spawnPendingCombatPresentations(nowMs: number): void {
    for (let index = this.pendingCombatPresentations.length - 1; index >= 0; index -= 1) {
      const pending = this.pendingCombatPresentations[index]!;
      const target = this.actorRoot(pending.event.target_entity_id);
      if (!target) {
        if (nowMs - pending.receivedAtMs < ORIGINAL_DAMAGE_TEXT_IDEAL_DURATION_MS) continue;
        this.pendingCombatPresentations.splice(index, 1);
        continue;
      }
      const root = this.createCombatPresentationView(pending.event);
      const originY = target.position.y - 52;
      root.position.set(target.position.x, originY);
      root.zIndex = villageActorDepth(target.position.y, SCENE_WORLD_HEIGHT) + 0.1;
      this.worldLayer.addChild(root);
      this.combatPresentationViews.push({ root, originY, spawnedAtMs: nowMs });
      this.pendingCombatPresentations.splice(index, 1);
    }
  }

  private createCombatPresentationView(event: CombatPresentationSnapshot): Container {
    const root = new Container();
    const lines = combatPresentationText(event);
    if (event.kind === "critical_damage") {
      const label = this.createCombatText(
        lines[0]!,
        COMBAT_CRITICAL_LABEL_SIZE_PX,
        ORIGINAL_CRITICAL_LABEL_COLOR,
      );
      const amount = this.createCombatText(
        lines[1]!,
        COMBAT_DAMAGE_FONT_SIZE_PX,
        ORIGINAL_NORMAL_DAMAGE_COLOR,
      );
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
      style: { fontFamily: ORIGINAL_DAMAGE_FONT_FAMILY, fontSize, fill, align: "center" },
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
}
