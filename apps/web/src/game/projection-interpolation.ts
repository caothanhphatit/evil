import type { WorldEntityProjection } from "../generated/protocol";

export type ProjectionMode = "village" | "field";
export type ProjectionPushResult = "buffered" | "snapped" | "ignored";

export interface ProjectionDriftDiagnostic {
  authoritativeTick: number;
  visualTick: number;
  driftTicks: number;
  thresholdTicks: number;
}

interface ProjectionFrame {
  mode: ProjectionMode;
  visualTick: number;
  receivedAtMs: number;
  entities: WorldEntityProjection[];
}

export interface ProjectionSample {
  mode: ProjectionMode;
  visualTick: number;
  entities: WorldEntityProjection[];
}

export interface ProjectionBufferOptions {
  tickDurationMs?: number;
  renderDelayMs?: number;
  maxFrames?: number;
  maxTickGap?: number;
  teleportDistance?: number;
  maxExtrapolationTicks?: number;
  visualDriftWarningTicks?: number;
  onVisualDrift?: (diagnostic: ProjectionDriftDiagnostic) => void;
}

export class ProjectionBuffer {
  private readonly frames: ProjectionFrame[] = [];
  private readonly tickDurationMs: number;
  private readonly renderDelayTicks: number;
  private readonly maxFrames: number;
  private readonly maxTickGap: number;
  private readonly teleportDistance: number;
  private readonly maxExtrapolationTicks: number;
  private readonly visualDriftWarningTicks: number;
  private readonly onVisualDrift: ((diagnostic: ProjectionDriftDiagnostic) => void) | null;
  private visualDriftActive = false;
  private renderTick: number | null = null;
  private sampledAtMs: number | null = null;

  constructor(options: ProjectionBufferOptions = {}) {
    this.tickDurationMs = options.tickDurationMs ?? 100;
    this.renderDelayTicks = (options.renderDelayMs ?? 0) / this.tickDurationMs;
    this.maxFrames = options.maxFrames ?? 12;
    this.maxTickGap = options.maxTickGap ?? 5;
    this.teleportDistance = options.teleportDistance ?? 220;
    this.maxExtrapolationTicks = options.maxExtrapolationTicks ?? 3;
    this.visualDriftWarningTicks = options.visualDriftWarningTicks ?? Number.POSITIVE_INFINITY;
    this.onVisualDrift = options.onVisualDrift ?? null;
  }

  push(mode: ProjectionMode, visualTick: number, entities: WorldEntityProjection[], receivedAtMs: number): ProjectionPushResult {
    let newest = this.frames.at(-1);
    const restartedTimeline = newest !== undefined && newest.visualTick > this.maxTickGap && visualTick === 0;
    if (restartedTimeline) {
      // A restored server session starts a fresh visual timeline at tick zero.
      // Treat that explicit boundary as a snap instead of ignoring every frame
      // until the new session catches up with the previous session's tick.
      this.reset();
      newest = undefined;
    }
    const matching = this.frames.find((frame) => frame.mode === mode && frame.visualTick === visualTick);
    if (matching) {
      if (newest && matching.visualTick < newest.visualTick) return "ignored";
      matching.entities = entities;
      return "buffered";
    }

    // TCP preserves order within one connection, but reconnect/application
    // scheduling can still deliver an already superseded projection. It must
    // never rewind the visual timeline or trigger a false teleport snap.
    if (newest && visualTick < newest.visualTick) return "ignored";
    const discontinuity = newest && (
      newest.mode !== mode
      || visualTick - newest.visualTick > this.maxTickGap
      || hasTeleport(newest.entities, entities, this.teleportDistance)
    );
    if (discontinuity) {
      this.frames.length = 0;
      this.renderTick = null;
      this.sampledAtMs = null;
    }

    this.frames.push({ mode, visualTick, receivedAtMs, entities });
    this.frames.sort((left, right) => left.visualTick - right.visualTick);
    if (this.frames.length > this.maxFrames) this.frames.splice(0, this.frames.length - this.maxFrames);
    return restartedTimeline || discontinuity ? "snapped" : "buffered";
  }

  sample(nowMs: number): ProjectionSample | null {
    if (this.frames.length === 0) return null;
    const newest = this.frames[this.frames.length - 1];
    if (this.frames.length === 1) return sampleFrame(newest);

    if (this.renderTick === null || this.sampledAtMs === null) {
      this.renderTick = newest.visualTick
        - this.renderDelayTicks
        + Math.max(0, nowMs - newest.receivedAtMs) / this.tickDurationMs;
    } else {
      this.renderTick += Math.max(0, nowMs - this.sampledAtMs) / this.tickDurationMs;
    }
    this.sampledAtMs = nowMs;
    // A throttled tab or a long main-thread pause can advance the wall-clock
    // timeline far beyond the newest authoritative frame. Keep the visual
    // clock bounded so later confirmations do not remain permanently behind
    // an accumulated extrapolation debt.
    const driftTicks = this.renderTick - newest.visualTick;
    if (driftTicks > this.visualDriftWarningTicks) {
      if (!this.visualDriftActive) {
        this.onVisualDrift?.({
          authoritativeTick: newest.visualTick,
          visualTick: this.renderTick,
          driftTicks,
          thresholdTicks: this.visualDriftWarningTicks,
        });
      }
      this.visualDriftActive = true;
    } else {
      this.visualDriftActive = false;
    }
    const maxRenderTick = newest.visualTick + Math.max(0, this.maxExtrapolationTicks);
    if (this.renderTick > maxRenderTick) this.renderTick = maxRenderTick;
    const targetTick = this.renderTick;
    const oldest = this.frames[0];
    if (targetTick <= oldest.visualTick) return sampleFrame(oldest);
    if (targetTick >= newest.visualTick) {
      if (this.maxExtrapolationTicks <= 0) return sampleFrame(newest);
      const previous = this.frames[this.frames.length - 2];
      const duration = newest.visualTick - previous.visualTick;
      if (duration <= 0 || duration > this.maxTickGap) return sampleFrame(newest);
      const extrapolation = Math.min(
        this.maxExtrapolationTicks,
        (targetTick - newest.visualTick) / duration,
      );
      return {
        mode: newest.mode,
        visualTick: newest.visualTick + extrapolation * duration,
        entities: extrapolateMovingEntities(previous.entities, newest.entities, extrapolation),
      };
    }

    for (let index = 1; index < this.frames.length; index += 1) {
      const current = this.frames[index];
      if (current.visualTick < targetTick) continue;
      const previous = this.frames[index - 1];
      const duration = Math.max(1, current.visualTick - previous.visualTick);
      const alpha = Math.max(0, Math.min(1, (targetTick - previous.visualTick) / duration));
      return {
        mode: current.mode,
        visualTick: targetTick,
        entities: interpolateEntities(previous.entities, current.entities, alpha),
      };
    }
    return sampleFrame(newest);
  }

  reset(): void {
    this.frames.length = 0;
    this.renderTick = null;
    this.sampledAtMs = null;
    this.visualDriftActive = false;
  }

  bufferedTicks(): number[] { return this.frames.map((frame) => frame.visualTick); }
}

function sampleFrame(frame: ProjectionFrame): ProjectionSample {
  return { mode: frame.mode, visualTick: frame.visualTick, entities: frame.entities };
}

function interpolateEntities(previous: WorldEntityProjection[], current: WorldEntityProjection[], alpha: number): WorldEntityProjection[] {
  const previousById = new Map(previous.map((entity) => [entity.descriptor.entity_id, entity]));
  return current.map((entity) => {
    const earlier = previousById.get(entity.descriptor.entity_id);
    if (!earlier) return entity;
    return {
      ...entity,
      x: earlier.x + (entity.x - earlier.x) * alpha,
      y: earlier.y + (entity.y - earlier.y) * alpha,
    };
  });
}

function extrapolateMovingEntities(
  previous: WorldEntityProjection[],
  current: WorldEntityProjection[],
  ticks: number,
): WorldEntityProjection[] {
  const previousById = new Map(previous.map((entity) => [entity.descriptor.entity_id, entity]));
  return current.map((entity) => {
    const earlier = previousById.get(entity.descriptor.entity_id);
    if (!earlier || entity.action_state !== "walking" || earlier.action_state !== "walking") {
      return entity;
    }
    return {
      ...entity,
      x: entity.x + (entity.x - earlier.x) * ticks,
      y: entity.y + (entity.y - earlier.y) * ticks,
    };
  });
}

function hasTeleport(previous: WorldEntityProjection[], current: WorldEntityProjection[], threshold: number): boolean {
  const previousById = new Map(previous.map((entity) => [entity.descriptor.entity_id, entity]));
  return current.some((entity) => {
    const earlier = previousById.get(entity.descriptor.entity_id);
    return earlier !== undefined && Math.hypot(entity.x - earlier.x, entity.y - earlier.y) > threshold;
  });
}
