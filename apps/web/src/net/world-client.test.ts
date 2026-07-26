import { describe, expect, it, vi } from "vitest";
import { PROTOCOL_VERSION } from "../generated/protocol";
import { apiBaseUrlFor, EnvelopeSequencer, ServerSequenceGuard, webSocketUrlFor, WorldClient } from "./world-client";

const TOKEN = "123e4567-e89b-42d3-a456-426614174000";

describe("original-flow protocol adapter", () => {
  it("creates complete_boot and menu envelopes with authoritative session state", () => {
    const sequencer = new EnvelopeSequencer();
    sequencer.acceptWelcome("00000000-0000-4000-8000-000000000001");
    expect(sequencer.wrap({ type: "complete_boot" }, TOKEN)).toEqual({ version: PROTOCOL_VERSION, sequence: 1, session_id: "00000000-0000-4000-8000-000000000001", correlation_id: TOKEN, payload: { type: "complete_boot" } });
    expect(sequencer.wrap({ type: "select_bottom_menu", menu: "character" }, TOKEN).sequence).toBe(2);
  });
});

describe("reconnect sequencing", () => {
  it("starts a new ordered envelope stream after reset", () => {
    const sequencer = new EnvelopeSequencer();
    sequencer.acceptWelcome("00000000-0000-4000-8000-000000000001");
    expect(sequencer.isReady()).toBe(true);
    sequencer.reset();
    expect(sequencer.isReady()).toBe(false);
    expect(sequencer.wrap({ type: "request_resync" }, TOKEN).sequence).toBe(1);
  });
});

describe("server sequence guard", () => {
  it("rejects duplicates and gaps", () => {
    const guard = new ServerSequenceGuard();
    expect(guard.accept(1)).toBe(true);
    expect(guard.accept(3)).toBe(false);
    expect(guard.accept(2)).toBe(true);
    expect(guard.accept(2)).toBe(false);
  });
});

describe("runtime endpoints", () => {
  it("defaults production traffic to the current origin", () => {
    expect(apiBaseUrlFor({ origin: "https://game.example" })).toBe("https://game.example");
    expect(webSocketUrlFor({ protocol: "https:", host: "game.example" })).toBe("wss://game.example/ws");
  });

  it("accepts runtime or build-time endpoint overrides", () => {
    expect(apiBaseUrlFor({ origin: "https://game.example" }, "https://api.example/")).toBe("https://api.example");
    expect(webSocketUrlFor({ protocol: "http:", host: "game.example" }, "wss://socket.example/live")).toBe("wss://socket.example/live");
  });
});

describe("session bootstrap", () => {
  it("bootstraps with credentials before opening the WebSocket", async () => {
    let finishBootstrap: ((response: Response) => void) | undefined;
    const fetchFn = vi.fn(() => new Promise<Response>((resolve) => { finishBootstrap = resolve; }));
    const socketFactory = vi.fn(() => fakeSocket());
    const statuses: string[] = [];
    const client = new WorldClient(() => undefined, (status) => statuses.push(status), undefined, undefined, undefined, {
      apiBaseUrl: "https://game.test/api/",
      webSocketUrl: "wss://game.test/ws",
      fetchFn,
      socketFactory,
    });

    client.connect();
    expect(fetchFn).toHaveBeenCalledWith("https://game.test/api/session/bootstrap", {
      method: "POST",
      credentials: "include",
      headers: { Accept: "application/json" },
    });
    expect(socketFactory).not.toHaveBeenCalled();

    finishBootstrap?.(new Response('{"status":"ready"}', { status: 200 }));
    await Promise.resolve();
    await Promise.resolve();

    expect(socketFactory).toHaveBeenCalledOnce();
    expect(socketFactory).toHaveBeenCalledWith("wss://game.test/ws");
    expect(statuses).toEqual(["connecting"]);
    client.disconnect();
  });

  it("fails closed when bootstrap is rejected", async () => {
    const fetchFn = vi.fn(async () => new Response(null, { status: 503 }));
    const socketFactory = vi.fn(() => fakeSocket());
    const client = new WorldClient(() => undefined, () => undefined, undefined, undefined, undefined, {
      apiBaseUrl: "http://game.test",
      webSocketUrl: "ws://game.test/ws",
      fetchFn,
      socketFactory,
      reconnectDelayMs: 60_000,
    });

    client.connect();
    await Promise.resolve();
    await Promise.resolve();

    expect(socketFactory).not.toHaveBeenCalled();
    client.disconnect();
  });

  it("queues an early boot intent until the welcome establishes a session", async () => {
    const socket = new FakeSocket();
    const client = new WorldClient(() => undefined, () => undefined, undefined, undefined, undefined, {
      apiBaseUrl: "http://game.test",
      webSocketUrl: "ws://game.test/ws",
      fetchFn: vi.fn(async () => new Response(null, { status: 204 })),
      socketFactory: () => socket as unknown as WebSocket,
    });

    expect(client.completeBoot()).toBe(true);
    client.connect();
    await settlePromises();
    expect(socket.sent).toEqual([]);

    socket.emit("message", { data: serverEnvelope(1, "welcome") });

    expect(socket.sent.map((wire) => JSON.parse(wire).payload.type)).toEqual(["request_resync", "complete_boot"]);
    client.disconnect();
  });

  it("sends generated placement command shapes without client-side outcomes", async () => {
    const socket = new FakeSocket();
    const client = new WorldClient(() => undefined, () => undefined, undefined, undefined, undefined, {
      apiBaseUrl: "http://game.test",
      webSocketUrl: "ws://game.test/ws",
      fetchFn: vi.fn(async () => new Response(null, { status: 204 })),
      socketFactory: () => socket as unknown as WebSocket,
    });

    client.connect();
    await settlePromises();
    socket.emit("message", { data: serverEnvelope(1, "welcome") });
    expect(client.constructBuildingAt("build_4", 12, 7)).toBe(true);
    expect(client.moveBuilding("building-instance-4", 14, 9)).toBe(true);
    expect(client.startBuildingService("building-instance-13", 7, "product:10")).toBe(true);
    expect(client.banishHunter(7)).toBe(true);

    expect(socket.sent.slice(-4).map((wire) => JSON.parse(wire).payload)).toEqual([
      { type: "construct_building_at", building_id: "build_4", grid_x: 12, grid_y: 7 },
      { type: "move_building", instance_id: "building-instance-4", grid_x: 14, grid_y: 9 },
      { type: "start_building_service", instance_id: "building-instance-13", hunter_id: 7, product_id: "product:10" },
      { type: "banish_hunter", hunter_id: 7 },
    ]);
    client.disconnect();
  });

  it("closes and reconnects when the server sequence has a gap", async () => {
    vi.useFakeTimers();
    const firstSocket = new FakeSocket();
    const secondSocket = new FakeSocket();
    const sockets = [firstSocket, secondSocket];
    const statuses: string[] = [];
    const snapshots: unknown[] = [];
    const client = new WorldClient((snapshot) => snapshots.push(snapshot), (status) => statuses.push(status), undefined, undefined, undefined, {
      apiBaseUrl: "http://game.test",
      webSocketUrl: "ws://game.test/ws",
      fetchFn: vi.fn(async () => new Response(null, { status: 204 })),
      socketFactory: () => sockets.shift() as unknown as WebSocket,
      reconnectDelayMs: 10,
    });

    client.connect();
    await settlePromises();
    firstSocket.emit("message", { data: serverEnvelope(1, "welcome") });
    firstSocket.emit("message", { data: serverEnvelope(3, "world_update") });

    expect(firstSocket.close).toHaveBeenCalledWith(4002, "Protocol error");
    expect(statuses.at(-1)).toBe("reconnecting");
    expect(snapshots).toHaveLength(1);

    await vi.advanceTimersByTimeAsync(10);
    expect(secondSocket.listeners.size).toBeGreaterThan(0);
    client.disconnect();
    vi.useRealTimers();
  });

  it("treats malformed server data as a protocol fault", async () => {
    const socket = new FakeSocket();
    const client = new WorldClient(() => undefined, () => undefined, undefined, undefined, undefined, {
      apiBaseUrl: "http://game.test",
      webSocketUrl: "ws://game.test/ws",
      fetchFn: vi.fn(async () => new Response(null, { status: 204 })),
      socketFactory: () => socket as unknown as WebSocket,
      reconnectDelayMs: 60_000,
    });

    client.connect();
    await settlePromises();
    socket.emit("message", { data: "not-json" });

    expect(socket.close).toHaveBeenCalledWith(4002, "Protocol error");
    client.disconnect();
  });
});

function fakeSocket(): WebSocket {
  return {
    close: vi.fn(),
    addEventListener: vi.fn(),
  } as unknown as WebSocket;
}

class FakeSocket {
  readonly listeners = new Map<string, Array<(event: MessageEvent) => void>>();
  readonly sent: string[] = [];
  readonly close = vi.fn();
  readyState = WebSocket.OPEN;

  addEventListener(type: string, listener: EventListenerOrEventListenerObject): void {
    const callback = listener as (event: MessageEvent) => void;
    this.listeners.set(type, [...(this.listeners.get(type) ?? []), callback]);
  }

  send(wire: string): void { this.sent.push(wire); }

  emit(type: string, event: { data: string }): void {
    for (const listener of this.listeners.get(type) ?? []) listener(event as MessageEvent);
  }
}

function serverEnvelope(sequence: number, type: "welcome" | "world_update"): string {
  const snapshot = {
    screen: "boot",
    content_release_id: "original-flow-v1",
    content_release_runnable: false,
    flow_order: ["boot", "village", "hunter_roster", "field"],
    village: {},
    hunter_roster: {},
    field: {},
    world: { entities: [] },
  };
  const payload = type === "welcome"
    ? { type, player_token: TOKEN, session_id: "00000000-0000-4000-8000-000000000001", snapshot }
    : { type, snapshot };
  return JSON.stringify({
    version: PROTOCOL_VERSION,
    sequence,
    session_id: "00000000-0000-4000-8000-000000000001",
    correlation_id: null,
    payload,
  });
}

async function settlePromises(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}
