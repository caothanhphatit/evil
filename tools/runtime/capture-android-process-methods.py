#!/usr/bin/env python3
"""Capture reviewed native method ranges from a rooted Android process."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path


def adb(adb_path: str, *args: str, text: bool = True) -> str | bytes:
    result = subprocess.run([adb_path, *args], check=True, capture_output=True)
    return result.stdout.decode().strip() if text else result.stdout


def parse_maps(payload: str, package_fragment: str) -> list[tuple[int, int, int, str]]:
    mappings = []
    for line in payload.splitlines():
        parts = line.split(maxsplit=5)
        if len(parts) < 6 or package_fragment not in parts[5]:
            continue
        start_text, end_text = parts[0].split("-", 1)
        mappings.append((int(start_text, 16), int(end_text, 16), int(parts[2], 16), parts[5]))
    return mappings


def resolve_module_base(mappings: list[tuple[int, int, int, str]], offsets: list[int]) -> int:
    candidates: dict[int, int] = {}
    for start, end, file_offset, _ in mappings:
        base = start - file_offset
        covered = sum(start <= base + offset < end for offset in offsets)
        candidates[base] = candidates.get(base, 0) + covered
    if not candidates:
        raise RuntimeError("No package executable mappings found")
    base, score = max(candidates.items(), key=lambda item: item[1])
    if score != len(offsets):
        raise RuntimeError(f"Unable to resolve one module base for all offsets: best={score}/{len(offsets)}")
    return base


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--adb", default="adb")
    parser.add_argument("--package", default="com.superplanet.evilhunter")
    parser.add_argument("--method-index", type=Path, required=True)
    parser.add_argument("--token", type=int, action="append", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--action", required=True)
    args = parser.parse_args()

    pids = adb(args.adb, "shell", "pidof", args.package).split()
    if not pids:
        raise RuntimeError(f"Package is not running: {args.package}")
    pid = int(pids[0])
    index = json.loads(args.method_index.read_text())
    methods = index["record"]["payload"]["methods"]
    ordered = []
    for method in methods:
        candidates = method.get("candidates") or []
        offset = candidates[0].get("moduleOffset") if candidates else None
        if offset:
            ordered.append((int(offset, 16), method))
    ordered.sort(key=lambda item: item[0])
    requested = []
    token_set = set(args.token)
    for position, (offset, method) in enumerate(ordered):
        if method.get("token") not in token_set:
            continue
        if position + 1 >= len(ordered):
            raise RuntimeError(f"No following boundary for token {method['token']}")
        requested.append((offset, ordered[position + 1][0] - offset, method))
    if {method[2]["token"] for method in requested} != token_set:
        raise RuntimeError("One or more requested tokens are absent from the method index")

    maps = adb(args.adb, "shell", "cat", f"/proc/{pid}/maps")
    mappings = parse_maps(maps, "split_config.arm64_v8a.apk")
    base = resolve_module_base(mappings, [offset for offset, _, _ in requested])
    captures = []
    for offset, size, method in requested:
        address = base + offset
        command = f"dd if=/proc/{pid}/mem bs=1 skip={address} count={size} 2>/dev/null"
        payload = adb(args.adb, "exec-out", "sh", "-c", command, text=False)
        if len(payload) != size:
            raise RuntimeError(f"Short read for token {method['token']}: {len(payload)}/{size}")
        captures.append({
            "className": method["className"],
            "methodName": method["methodName"],
            "token": method["token"],
            "moduleOffset": f"0x{offset:x}",
            "size": size,
            "sha256": hashlib.sha256(payload).hexdigest(),
            "codeHex": payload.hex(),
        })

    evidence = {
        "schemaVersion": 1,
        "contractType": "android-external-native-method-capture",
        "capture": {
            "capturedAtUtc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
            "packageId": args.package,
            "pid": pid,
            "deviceAbi": adb(args.adb, "shell", "getprop", "ro.product.cpu.abi"),
            "androidApi": adb(args.adb, "shell", "getprop", "ro.build.version.sdk"),
            "moduleBase": f"0x{base:x}",
            "action": args.action,
            "source": "/proc/PID/mem external root read; no in-process agent",
        },
        "methods": captures,
        "limitations": [
            "ASLR addresses are session-specific; tokens and module offsets are stable identifiers.",
            "Native bodies alone do not prove product-facing meanings for obfuscated fields or callees.",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, indent=2) + "\n")
    print(f"Captured {len(captures)} methods from PID {pid}")


if __name__ == "__main__":
    main()
