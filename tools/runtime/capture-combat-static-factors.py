#!/usr/bin/env python3
"""Capture non-player ConstantData combat factors from rooted Android memory."""

from __future__ import annotations

import argparse
import json
import struct
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path


DEFAULT_ADB = "/Users/trana/Library/Android/sdk/platform-tools/adb"
DEFAULT_PACKAGE = "com.superplanet.evilhunter"
CONSTANT_DATA_GLOBAL_OFFSET = 0x601C6E0


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


def read_pointer(adb: str, pid: int, address: int) -> tuple[int, str]:
    body = read_process_memory(adb, pid, address, 8)
    return struct.unpack("<Q", body)[0], body.hex()


def decode_obscured_float(raw: bytes) -> float:
    if len(raw) != 20:
        raise ValueError(f"ObscuredFloat requires 20 bytes, got {len(raw)}")
    current_crypto_key = struct.unpack_from("<I", raw, 0)[0]
    hidden_value = bytearray(raw[4:8])
    # ACTkByte4.UnShuffle swaps the two middle bytes before the key XOR.
    hidden_value[1], hidden_value[2] = hidden_value[2], hidden_value[1]
    decrypted_bits = current_crypto_key ^ struct.unpack("<I", hidden_value)[0]
    return struct.unpack("<f", struct.pack("<I", decrypted_bits))[0]


def find_module_base(maps: str) -> tuple[int, str]:
    for line in maps.splitlines():
        columns = line.split()
        if len(columns) < 6:
            continue
        if columns[1] == "r--p" and columns[2] == "00684000" and columns[-1].endswith(
            "/split_config.arm64_v8a.apk"
        ):
            return int(columns[0].split("-", 1)[0], 16), line
    raise RuntimeError("libil2cpp read-only segment was not found in process maps")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--adb", default=DEFAULT_ADB)
    parser.add_argument("--package", default=DEFAULT_PACKAGE)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--action", required=True)
    parser.add_argument("--retries", type=int, default=10)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    last_error = None
    for attempt in range(args.retries):
        try:
            pid_text = shell_text(args.adb, f"pidof {args.package}")
            if not pid_text:
                raise RuntimeError(f"package is not running: {args.package}")
            pid = int(pid_text.split()[0])
            maps = shell_text(args.adb, f"su 0 sh -c 'cat /proc/{pid}/maps'")
            module_base, module_map = find_module_base(maps)
            global_pointer, global_pointer_hex = read_pointer(
                args.adb, pid, module_base + CONSTANT_DATA_GLOBAL_OFFSET
            )
            class_pointer, class_pointer_hex = read_pointer(args.adb, pid, global_pointer)
            runtime_data_pointer, runtime_data_pointer_hex = read_pointer(
                args.adb, pid, class_pointer + 0xB8
            )
            if runtime_data_pointer == 0:
                raise RuntimeError("GameManager runtime data is not initialized")
            armor_array_pointer, armor_array_pointer_hex = read_pointer(
                args.adb, pid, runtime_data_pointer + 0xF8
            )
            if armor_array_pointer == 0:
                raise RuntimeError("armor factor array is not initialized")
            armor_array_raw = read_process_memory(args.adb, pid, armor_array_pointer, 52)
            armor_array_length = struct.unpack_from("<Q", armor_array_raw, 24)[0]
            if armor_array_length != 5:
                raise RuntimeError(f"unexpected armor factor count: {armor_array_length}")
            armor_factors = list(struct.unpack_from("<5f", armor_array_raw, 32))
            final_factor_raw = read_process_memory(
                args.adb, pid, runtime_data_pointer + 0x114, 20
            )
            decoded_final_factor = decode_obscured_float(final_factor_raw)
            break
        except (RuntimeError, subprocess.CalledProcessError) as error:
            last_error = error
            if attempt + 1 == args.retries:
                raise RuntimeError("combat static factors did not initialize") from last_error
            time.sleep(0.5)

    output = {
        "schemaVersion": 1,
        "contractType": "original-runtime-combat-static-factor-capture",
        "runtimeCompatibility": "evidence-only",
        "capture": {
            "capturedAtUtc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
            "packageId": args.package,
            "pid": pid,
            "androidApi": shell_text(args.adb, "getprop ro.build.version.sdk"),
            "deviceAbi": shell_text(args.adb, "getprop ro.product.cpu.abi"),
            "action": args.action,
            "authority": "rooted guest tutorial emulator; static GameManager data only",
        },
        "module": {
            "name": "libil2cpp.so",
            "base": f"0x{module_base:x}",
            "mapLine": module_map,
            "constantDataGlobalOffset": f"0x{CONSTANT_DATA_GLOBAL_OFFSET:x}",
        },
        "pointerChain": [
            { "readOffset": f"libil2cpp+0x{CONSTANT_DATA_GLOBAL_OFFSET:x}", "rawHex": global_pointer_hex, "value": f"0x{global_pointer:x}" },
            { "readOffset": "global+0x0", "rawHex": class_pointer_hex, "value": f"0x{class_pointer:x}" },
            { "readOffset": "class+0xb8", "rawHex": runtime_data_pointer_hex, "value": f"0x{runtime_data_pointer:x}" },
            { "readOffset": "runtimeData+0xf8", "rawHex": armor_array_pointer_hex, "value": f"0x{armor_array_pointer:x}" },
        ],
        "armorFactorArray": {
            "runtimeDataOffset": 248,
            "managedArrayRawHex": armor_array_raw.hex(),
            "length": armor_array_length,
            "values": armor_factors,
        },
        "selectedFinalFactor": {
            "runtimeDataOffset": 276,
            "owner": "ConstantData",
            "field": "DEFALUT_DAMAGE_DECREASE_VALUE",
            "obscuredFloatRawHex": final_factor_raw.hex(),
            "decode": "float32(currentCryptoKey XOR ACTkByte4.UnShuffle(hiddenValue))",
            "decodedValue": decoded_final_factor,
            "semanticStatus": "static initonly field and 0.75 constructor writer confirmed",
        },
        "limitations": [
            "No Hunter object, save value, account value, or service credential was read.",
            "Absolute pointers and ACTk crypto keys are session-specific runtime evidence.",
            "The decoded DEFALUT_DAMAGE_DECREASE_VALUE is cross-checked against the ConstantData static-constructor writer.",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote combat static factor capture to {args.output}")


if __name__ == "__main__":
    main()
