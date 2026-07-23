import { describe, expect, it } from "vitest";
import { interpolateEntities, SnapshotBuffer } from "./interpolation";
import type { WorldSnapshot } from "../types";

const snapshot = (sequence: number, serverTime: number, x: number): WorldSnapshot => ({
  sequence,
  serverTime,
  fighting: true,
  gold: 10,
  entities: [{ id: 1, kind: "hunter", x, y: 0, hp: 100, max_hp: 100, alive: true, state: "moving", name: "Hunter", facing: "front" }],
  inventory: [],
  equippedItemId: null,
  groundDrops: [],
});

describe("SnapshotBuffer", () => {
  it("sorts snapshots and interpolates authoritative positions at render time", () => {
    const buffer = new SnapshotBuffer(100);
    buffer.push(snapshot(2, 200, 20));
    buffer.push(snapshot(1, 100, 0));

    const pair = buffer.sample(250);
    expect(pair).not.toBeNull();
    expect(pair?.alpha).toBe(0.5);
    expect(interpolateEntities(pair!)[0]?.x).toBe(10);
  });

  it("ignores duplicate tick numbers", () => {
    const buffer = new SnapshotBuffer(0);
    buffer.push(snapshot(1, 100, 4));
    buffer.push(snapshot(1, 100, 99));
    expect(interpolateEntities(buffer.sample(100)!)[0]?.x).toBe(4);
  });
});
