import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";

// Historical protocol-v2 fixture evidence. It is intentionally excluded from the v4 quality gate.

const worldUrl = process.env.WORLD_WS_URL ?? "ws://localhost:8080/ws";
const timeoutMs = Number(process.env.WORLD_SMOKE_TIMEOUT_MS ?? 30_000);

function connect() {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(worldUrl);
    const timeout = setTimeout(() => {
      socket.close();
      reject(new Error(`Timed out connecting to ${worldUrl}`));
    }, timeoutMs);
    socket.addEventListener("open", () => {
      clearTimeout(timeout);
      resolve(socket);
    }, { once: true });
    socket.addEventListener("error", () => {
      clearTimeout(timeout);
      reject(new Error(`WebSocket connection failed: ${worldUrl}`));
    }, { once: true });
  });
}

function journey(socket, { equip = false } = {}) {
  return new Promise((resolve, reject) => {
    let serverSequence = 0;
    let clientSequence = 0;
    let sessionId;
    let equipCommandId;
    const timeout = setTimeout(() => {
      socket.close();
      reject(new Error("Timed out waiting for the authoritative world journey"));
    }, timeoutMs);

    const finish = (snapshot) => {
      clearTimeout(timeout);
      socket.removeEventListener("message", onMessage);
      resolve(snapshot);
    };

    const send = (payload, correlationId = randomUUID()) => {
      clientSequence += 1;
      socket.send(JSON.stringify({
        version: 2,
        sequence: clientSequence,
        session_id: sessionId,
        correlation_id: correlationId,
        payload,
      }));
    };

    const onMessage = (event) => {
      try {
        const envelope = JSON.parse(String(event.data));
        assert.equal(envelope.version, 2);
        assert.equal(envelope.sequence, serverSequence + 1);
        serverSequence = envelope.sequence;
        const message = envelope.payload;

        if (message.type === "welcome") {
          sessionId = message.session_id;
          assert.equal(envelope.session_id, sessionId);
          send({ type: "request_resync" });
          if (!equip && message.snapshot.equipped_item_id === 2001) finish(message.snapshot);
          return;
        }

        const snapshot = message.snapshot ?? (message.type === "snapshot" ? message : null);
        if (!snapshot) return;

        if (equip && !equipCommandId && snapshot.inventory.some((item) => item.item_id === 2001)) {
          equipCommandId = randomUUID();
          send({ type: "equip_item", command_id: equipCommandId, item_id: 2001 }, equipCommandId);
          return;
        }

        if (equip && message.type === "command_result" && message.command_id === equipCommandId) {
          assert.equal(message.accepted, true);
          assert.equal(snapshot.equipped_item_id, 2001);
          assert.ok(snapshot.gold > 0);
          finish(snapshot);
        }
      } catch (error) {
        clearTimeout(timeout);
        reject(error);
      }
    };

    socket.addEventListener("message", onMessage);
  });
}

const firstSocket = await connect();
const equippedState = await journey(firstSocket, { equip: true });
firstSocket.close();
await new Promise((resolve) => setTimeout(resolve, 300));

const reconnectSocket = await connect();
const restoredState = await journey(reconnectSocket);
reconnectSocket.close();

assert.equal(restoredState.equipped_item_id, equippedState.equipped_item_id);
assert.equal(restoredState.gold, equippedState.gold);
assert.deepEqual(restoredState.inventory, equippedState.inventory);
console.log(`World smoke passed at tick ${restoredState.tick}: gold=${restoredState.gold}, equipped=${restoredState.equipped_item_id}`);
