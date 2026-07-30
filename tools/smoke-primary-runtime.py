#!/usr/bin/env python3
"""Exercise the primary protocol without trusting client-computed outcomes."""

import base64
import hashlib
import http.client
import json
import os
import secrets
import socket
import struct
import time
import uuid
from pathlib import Path
from urllib.parse import urlparse


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
PROTOCOL_VERSION = json.loads(
    (REPOSITORY_ROOT / "packages/protocol/world-v1.schema.json").read_text(encoding="utf-8")
)["schemaVersion"]
LOOT_ITEM_ID = 2001


class WebSocketConnection:
    def __init__(self, url: str, cookie: str, origin: str):
        parsed = urlparse(url)
        if parsed.scheme != "ws":
            raise RuntimeError("The smoke test currently supports ws:// URLs only")
        self.socket = socket.create_connection((parsed.hostname, parsed.port or 80), timeout=5)
        self.buffer = b""
        key = base64.b64encode(secrets.token_bytes(16)).decode()
        path = parsed.path or "/"
        request = (
            f"GET {path} HTTP/1.1\r\n"
            f"Host: {parsed.netloc}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\n"
            "Sec-WebSocket-Version: 13\r\n"
            f"Origin: {origin}\r\n"
            f"Cookie: {cookie}\r\n\r\n"
        )
        self.socket.sendall(request.encode())
        response = self._read_until(b"\r\n\r\n")
        status = response.split(b"\r\n", 1)[0]
        if b" 101 " not in status:
            raise RuntimeError(f"WebSocket upgrade failed: {status.decode(errors='replace')}")
        expected = base64.b64encode(
            hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest()
        )
        if expected.lower() not in response.lower():
            raise RuntimeError("WebSocket accept key mismatch")

    def _read_until(self, marker: bytes) -> bytes:
        while marker not in self.buffer:
            chunk = self.socket.recv(65536)
            if not chunk:
                raise RuntimeError("Connection closed during handshake")
            self.buffer += chunk
        end = self.buffer.index(marker) + len(marker)
        result, self.buffer = self.buffer[:end], self.buffer[end:]
        return result

    def _read_exact(self, length: int) -> bytes:
        while len(self.buffer) < length:
            chunk = self.socket.recv(65536)
            if not chunk:
                raise RuntimeError("WebSocket closed unexpectedly")
            self.buffer += chunk
        result, self.buffer = self.buffer[:length], self.buffer[length:]
        return result

    def send_json(self, value: dict) -> None:
        self._send_frame(0x1, json.dumps(value, separators=(",", ":")).encode())

    def _send_frame(self, opcode: int, payload: bytes) -> None:
        mask = secrets.token_bytes(4)
        length = len(payload)
        if length < 126:
            header = bytes((0x80 | opcode, 0x80 | length))
        elif length <= 0xFFFF:
            header = bytes((0x80 | opcode, 0xFE)) + struct.pack("!H", length)
        else:
            header = bytes((0x80 | opcode, 0xFF)) + struct.pack("!Q", length)
        masked = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
        self.socket.sendall(header + mask + masked)

    def receive_json(self, timeout: float) -> dict:
        deadline = time.monotonic() + timeout
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("Timed out waiting for a WebSocket message")
            self.socket.settimeout(remaining)
            first, second = self._read_exact(2)
            opcode = first & 0x0F
            length = second & 0x7F
            if length == 126:
                length = struct.unpack("!H", self._read_exact(2))[0]
            elif length == 127:
                length = struct.unpack("!Q", self._read_exact(8))[0]
            masked = bool(second & 0x80)
            mask = self._read_exact(4) if masked else b""
            payload = self._read_exact(length)
            if masked:
                payload = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
            if opcode == 0x9:
                self._send_frame(0xA, payload)
                continue
            if opcode == 0x8:
                raise RuntimeError("Server closed the WebSocket")
            if opcode != 0x1:
                continue
            return json.loads(payload)

    def close(self) -> None:
        try:
            self._send_frame(0x8, struct.pack("!H", 1000))
        finally:
            self.socket.close()


def bootstrap(api_url: str) -> str:
    parsed = urlparse(api_url)
    connection = http.client.HTTPConnection(parsed.hostname, parsed.port, timeout=5)
    connection.request("POST", "/session/bootstrap", body=b"")
    response = connection.getresponse()
    response.read()
    if response.status != 200:
        raise RuntimeError(f"Session bootstrap failed: HTTP {response.status}")
    cookie = response.getheader("Set-Cookie")
    if not cookie:
        raise RuntimeError("Session bootstrap did not set a cookie")
    return cookie.split(";", 1)[0]


def wait_payload(connection: WebSocketConnection, predicate, timeout: float = 30) -> dict:
    deadline = time.monotonic() + timeout
    while True:
        message = connection.receive_json(max(0.1, deadline - time.monotonic()))
        if message.get("version") != PROTOCOL_VERSION:
            raise RuntimeError(f"Unexpected protocol version: {message.get('version')}")
        payload = message.get("payload", {})
        if predicate(payload):
            return payload


def command(sequence: int, session_id: str, payload: dict) -> dict:
    return {
        "version": PROTOCOL_VERSION,
        "sequence": sequence,
        "session_id": session_id,
        "correlation_id": str(uuid.uuid4()),
        "payload": payload,
    }


def connect_with_retry(ws_url: str, cookie: str, origin: str) -> WebSocketConnection:
    deadline = time.monotonic() + 10
    last_error = None
    while time.monotonic() < deadline:
        try:
            return WebSocketConnection(ws_url, cookie, origin)
        except (OSError, RuntimeError) as error:
            last_error = error
            time.sleep(0.25)
    raise RuntimeError(f"Could not reconnect: {last_error}")


def main() -> None:
    api_url = os.environ.get("EVIL_API_URL", "http://127.0.0.1:28080")
    ws_url = os.environ.get("EVIL_WS_URL", "ws://127.0.0.1:28080/ws")
    origin = os.environ.get("EVIL_WEB_ORIGIN", "http://localhost:25173")
    cookie = bootstrap(api_url)
    connection = connect_with_retry(ws_url, cookie, origin)
    welcome = wait_payload(connection, lambda payload: payload.get("type") == "welcome", 5)
    session_id = welcome["session_id"]
    sequence = 1

    connection.send_json(command(sequence, session_id, {"type": "complete_boot"}))
    wait_payload(connection, lambda payload: payload.get("type") == "intent_result" and payload["snapshot"]["screen"] == "village")
    sequence += 1
    connection.send_json(command(sequence, session_id, {"type": "enter_field"}))
    wait_payload(connection, lambda payload: payload.get("type") == "intent_result" and payload["snapshot"]["screen"] == "field")

    def owns_fixture_item(payload: dict) -> bool:
        snapshot = payload.get("snapshot", {})
        projection = snapshot.get("migration_fixture_combat", {})
        if projection.get("evidence_label") != "deterministic_migration_fixture_not_legacy_balance":
            return False
        inventory = projection.get("world", {}).get("inventory", [])
        return any(stack.get("item_id") == LOOT_ITEM_ID and stack.get("quantity", 0) > 0 for stack in inventory)

    loot_message = wait_payload(connection, owns_fixture_item, 35)
    combat = loot_message["snapshot"]["migration_fixture_combat"]["world"]
    if combat["gold"] < 10:
        raise RuntimeError("Authoritative loot did not grant fixture gold")

    sequence += 1
    connection.send_json(command(sequence, session_id, {
        "type": "equip_hunter_item", "hunter_id": 1, "item_id": LOOT_ITEM_ID,
    }))
    equipped = wait_payload(
        connection,
        lambda payload: payload.get("type") == "intent_result"
        and payload.get("accepted") is True
        and payload["snapshot"]["migration_fixture_combat"]["world"].get("equipped_item_id") == LOOT_ITEM_ID,
    )
    equipped_tick = equipped["snapshot"]["migration_fixture_combat"]["world"]["tick"]
    connection.close()

    connection = connect_with_retry(ws_url, cookie, origin)
    restored = wait_payload(connection, lambda payload: payload.get("type") == "welcome", 10)
    restored_combat = restored["snapshot"]["migration_fixture_combat"]["world"]
    if restored["snapshot"]["screen"] != "field":
        raise RuntimeError("Reconnect did not restore the field screen")
    if restored_combat.get("equipped_item_id") != LOOT_ITEM_ID or restored_combat.get("gold", 0) < 10:
        raise RuntimeError("Reconnect did not restore authoritative loot/equipment")
    if restored_combat.get("tick", 0) < equipped_tick:
        raise RuntimeError("Reconnect moved the authoritative combat clock backwards")
    connection.close()
    print(json.dumps({
        "status": "ok",
        "protocol": PROTOCOL_VERSION,
        "gold": restored_combat["gold"],
        "equipped_item_id": restored_combat["equipped_item_id"],
        "restored_tick": restored_combat["tick"],
    }))


if __name__ == "__main__":
    main()
