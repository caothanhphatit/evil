import type { EntityState, WorldSnapshot } from "../types";

export interface SnapshotPair {
  previous: WorldSnapshot;
  current: WorldSnapshot;
  alpha: number;
}

export class SnapshotBuffer {
  private readonly snapshots: WorldSnapshot[] = [];

  constructor(
    private readonly interpolationDelayMs = 150,
    private readonly maxSnapshots = 30,
  ) {}

  push(snapshot: WorldSnapshot): void {
    if (this.snapshots.some((item) => item.sequence === snapshot.sequence)) return;
    this.snapshots.push(snapshot);
    this.snapshots.sort((a, b) => a.serverTime - b.serverTime);
    if (this.snapshots.length > this.maxSnapshots) this.snapshots.splice(0, this.snapshots.length - this.maxSnapshots);
  }

  sample(nowMs: number): SnapshotPair | null {
    if (this.snapshots.length === 0) return null;
    if (this.snapshots.length === 1) {
      const only = this.snapshots[0];
      return { previous: only, current: only, alpha: 1 };
    }

    const renderTime = nowMs - this.interpolationDelayMs;
    let previous = this.snapshots[0];
    let current = this.snapshots[this.snapshots.length - 1];

    for (let index = 1; index < this.snapshots.length; index += 1) {
      if (this.snapshots[index].serverTime >= renderTime) {
        current = this.snapshots[index];
        previous = this.snapshots[index - 1];
        break;
      }
      previous = this.snapshots[index];
    }

    const duration = Math.max(1, current.serverTime - previous.serverTime);
    const alpha = Math.max(0, Math.min(1, (renderTime - previous.serverTime) / duration));
    return { previous, current, alpha };
  }
}

export function interpolateEntities(pair: SnapshotPair): EntityState[] {
  const previousById = new Map(pair.previous.entities.map((entity) => [entity.id, entity]));
  return pair.current.entities.map((entity) => {
    const previous = previousById.get(entity.id);
    if (!previous) return entity;
    return {
      ...entity,
      x: previous.x + (entity.x - previous.x) * pair.alpha,
      y: previous.y + (entity.y - previous.y) * pair.alpha,
      hp: previous.hp + (entity.hp - previous.hp) * pair.alpha,
    };
  });
}
