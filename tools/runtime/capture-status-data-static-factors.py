#!/usr/bin/env python3
"""Capture non-player static literals used by StatusData damage producers."""

from __future__ import annotations

import argparse
import json
import struct
import subprocess
from datetime import datetime, timezone
from pathlib import Path


DEFAULT_ADB = "/Users/trana/Library/Android/sdk/platform-tools/adb"
DEFAULT_PACKAGE = "com.superplanet.evilhunter"
CALC_LEVEL_FACTOR_OFFSET = 0xD2A96C
FAIRY_ATTACK_FACTOR_OFFSETS = {
    "fairy_index_78_418_599_600": 0xD2B4F8,
    "fairy_index_360": 0xD2BAEC,
    "fairy_index_748_773": 0xD2A6A0,
}
POLY_INDEX_49_MULTIPLIER_OFFSET = 0xD282F8
MONSTER_DAMAGE_FLOAT_OFFSETS = {
    "feel_ratio_80_percent": 0xD2A50C,
    "feel_ratio_60_percent": 0xD2A414,
    "feel_ratio_40_percent": 0xD2B814,
    "feel_ratio_20_percent": 0xD2AAB8,
    "integer_percent_scale": 0xD2AC8C,
}


def run(adb: str, *args: str) -> bytes:
    return subprocess.check_output([adb, *args])


def shell_text(adb: str, command: str) -> str:
    return run(adb, "shell", command).decode().strip()


def read_process_memory(adb: str, pid: int, address: int, size: int) -> bytes:
    command = (
        "su 0 sh -c 'dd if=/proc/"
        f"{pid}/mem bs=1 skip={address} count={size} 2>/dev/null'"
    )
    body = run(adb, "shell", command)
    if len(body) != size:
        raise RuntimeError(f"short process-memory read at 0x{address:x}: {len(body)} != {size}")
    return body


def find_module_base(maps: str) -> tuple[int, str]:
    for line in maps.splitlines():
        columns = line.split()
        if (
            len(columns) >= 6
            and columns[1] == "r--p"
            and columns[2] == "00684000"
            and columns[-1].endswith("/split_config.arm64_v8a.apk")
        ):
            return int(columns[0].split("-", 1)[0], 16), line
    raise RuntimeError("libil2cpp module base mapping was not found")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--adb", default=DEFAULT_ADB)
    parser.add_argument("--package", default=DEFAULT_PACKAGE)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--action", required=True)
    args = parser.parse_args()

    pid_text = shell_text(args.adb, f"pidof {args.package}")
    if not pid_text:
        raise RuntimeError(f"package is not running: {args.package}")
    pid = int(pid_text.split()[0])
    maps = shell_text(args.adb, f"su 0 sh -c 'cat /proc/{pid}/maps'")
    module_base, module_map = find_module_base(maps)
    calc_level_raw = read_process_memory(
        args.adb, pid, module_base + CALC_LEVEL_FACTOR_OFFSET, 4
    )
    fairy_factors = {}
    for name, offset in FAIRY_ATTACK_FACTOR_OFFSETS.items():
        raw = read_process_memory(args.adb, pid, module_base + offset, 4)
        fairy_factors[name] = {
            "moduleOffset": f"0x{offset:x}",
            "rawHex": raw.hex(),
            "float32": struct.unpack("<f", raw)[0],
        }
    poly_raw = read_process_memory(
        args.adb, pid, module_base + POLY_INDEX_49_MULTIPLIER_OFFSET, 8
    )
    monster_damage_factors = {}
    for name, offset in MONSTER_DAMAGE_FLOAT_OFFSETS.items():
        raw = read_process_memory(args.adb, pid, module_base + offset, 4)
        monster_damage_factors[name] = {
            "moduleOffset": f"0x{offset:x}",
            "rawHex": raw.hex(),
            "float32": struct.unpack("<f", raw)[0],
        }

    output = {
        "schemaVersion": 1,
        "contractType": "original-runtime-status-data-static-factor-capture",
        "runtimeCompatibility": "evidence-only",
        "capture": {
            "capturedAtUtc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
            "packageId": args.package,
            "pid": pid,
            "androidApi": shell_text(args.adb, "getprop ro.build.version.sdk"),
            "deviceAbi": shell_text(args.adb, "getprop ro.product.cpu.abi"),
            "action": args.action,
            "authority": "rooted guest tutorial emulator; static package literal only",
        },
        "module": {
            "name": "libil2cpp.so",
            "base": f"0x{module_base:x}",
            "mapLine": module_map,
        },
        "calcLevelFactor": {
            "moduleOffset": f"0x{CALC_LEVEL_FACTOR_OFFSET:x}",
            "rawHex": calc_level_raw.hex(),
            "float32": struct.unpack("<f", calc_level_raw)[0],
        },
        "fairyAttackFactors": fairy_factors,
        "polyIndex49Multiplier": {
            "moduleOffset": f"0x{POLY_INDEX_49_MULTIPLIER_OFFSET:x}",
            "rawHex": poly_raw.hex(),
            "float64": struct.unpack("<d", poly_raw)[0],
        },
        "monsterDamageFactors": monster_damage_factors,
        "limitations": [
            "No Hunter object, save value, account value, or credential was read.",
            "The absolute process address is intentionally omitted because it is session-specific.",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote StatusData static factor capture to {args.output}")


if __name__ == "__main__":
    main()
