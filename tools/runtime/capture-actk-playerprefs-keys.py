#!/usr/bin/env python3
"""Capture ACTk plaintext key names without exporting stored values."""

from __future__ import annotations

import argparse
import json
import subprocess
import threading
import time
from datetime import datetime, timezone
from pathlib import Path

import frida


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SCRIPT = ROOT / "tools/runtime/actk-playerprefs-key-trace.js"


def adb(executable: str, *args: str) -> str:
    return subprocess.run(
        [executable, *args], check=True, capture_output=True, text=True
    ).stdout.strip()


def wait_for_pid(executable: str, package: str, timeout: float = 5.0) -> int:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = subprocess.run(
            [executable, "shell", "pidof", package],
            check=False,
            capture_output=True,
            text=True,
        ).stdout.strip()
        if result:
            return int(result.split()[0])
        time.sleep(0.05)
    raise TimeoutError(f"Timed out waiting for {package}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--adb", default="adb")
    parser.add_argument("--package", default="com.superplanet.evilhunter")
    parser.add_argument(
        "--activity", default="com.google.firebase.MessagingUnityPlayerActivity"
    )
    parser.add_argument("--script", type=Path, default=DEFAULT_SCRIPT)
    parser.add_argument("--duration", type=float, default=10.0)
    parser.add_argument("--attach-delay", type=float, default=2.0)
    parser.add_argument(
        "--trigger-home-on-ready",
        action="store_true",
        help="Send Android HOME after hooks are ready to capture pause/save key access.",
    )
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    adb(args.adb, "shell", "am", "force-stop", args.package)
    adb(
        args.adb,
        "shell",
        "am",
        "start",
        "-n",
        f"{args.package}/{args.activity}",
    )
    pid = wait_for_pid(args.adb, args.package)
    time.sleep(max(0.0, args.attach_delay))
    device = frida.get_usb_device(timeout=5)
    session = device.attach(pid)
    ready = threading.Event()
    records: list[dict[str, object]] = []
    errors: list[dict[str, object]] = []

    def on_message(message: dict[str, object], data: bytes | None) -> None:
        if message.get("type") == "send" and isinstance(message.get("payload"), dict):
            record = message["payload"]
            kind = record.get("kind")
            if kind == "actk-playerprefs-hooks-ready":
                ready.set()
            elif kind == "actk-playerprefs-key":
                payload = record.get("payload")
                if isinstance(payload, dict):
                    records.append(payload)
            elif kind == "actk-playerprefs-trace-error":
                errors.append(record)
                ready.set()
        elif message.get("type") == "error":
            errors.append(message)
            ready.set()

    script = session.create_script(args.script.read_text())
    script.on("message", on_message)
    script.load()
    ready.wait(min(args.duration, 5.0))
    if args.trigger_home_on_ready and ready.is_set():
        adb(args.adb, "shell", "input", "keyevent", "KEYCODE_HOME")
    threading.Event().wait(max(0.0, args.duration - 5.0))

    unique = sorted(
        {
            (
                str(record.get("method")),
                str(record.get("key")),
                record.get("parameterCount"),
                record.get("secondStringLength"),
            )
            for record in records
            if record.get("key")
        }
    )
    evidence = {
        "schemaVersion": 1,
        "contractType": "actk-playerprefs-key-trace",
        "capturedAtUtc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "packageId": args.package,
        "pid": pid,
        "durationSeconds": args.duration,
        "valuePolicy": "Plaintext key names and value lengths only; stored values omitted.",
        "records": [
            {
                "method": method,
                "key": key,
                "parameterCount": parameter_count,
                "secondStringLength": second_length,
            }
            for method, key, parameter_count, second_length in unique
        ],
        "errors": errors,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    print(f"Captured {len(unique)} unique ACTk key calls to {args.output}")


if __name__ == "__main__":
    main()
