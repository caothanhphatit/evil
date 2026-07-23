import { describe, expect, it, vi } from "vitest";
import { PROTOCOL_VERSION } from "../generated/protocol";
import { EnvelopeSequencer, ServerSequenceGuard, WorldClient } from "./world-client";

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
});

function fakeSocket(): WebSocket {
  return {
    close: vi.fn(),
    addEventListener: vi.fn(),
  } as unknown as WebSocket;
}
