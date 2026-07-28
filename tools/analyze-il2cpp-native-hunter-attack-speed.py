#!/usr/bin/env python3
"""Normalize the exact Hunter attack-cadence field chain from API 35 captures."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "reverse-engineering/evidence"
DEFAULT_CAPTURE = EVIDENCE / "original-native-hunter-attack-speed-writers-api35-v1.json"
DEFAULT_TARGET = EVIDENCE / "original-native-hunter-attack-speed-target-resolution-api35-v1.json"
DEFAULT_REFRESH = EVIDENCE / "original-native-hunter-refresh-animation-api35-v1.json"
DEFAULT_CADENCE = EVIDENCE / "original-native-combat-cadence-stat-chain-v1.json"
DEFAULT_HUNTER_SCHEMA = EVIDENCE / "evil-ai-drop-runtime-schema-android-api35-v1.json"
DEFAULT_STATUS_SCHEMA = EVIDENCE / "status-data-runtime-schema-android-api35-v1.json"
DEFAULT_OUTPUT = EVIDENCE / "original-native-hunter-attack-speed-chain-v1.json"

EXPECTED_METHODS = {
    "SettingProperty": ("0x343688c", 580),
    "getStatusData": ("0x33f7900", 188),
    "HuntingAttackAction": ("0x3416a40", 8016),
}
REQUESTED_OFFSETS = {0x194, 0x1AC, 0x3D8, 0x6AC}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source(path: Path) -> dict[str, str]:
    return {"path": path.relative_to(ROOT).as_posix(), "sha256": sha256(path)}


def sign_extend(value: int, bits: int) -> int:
    sign = 1 << (bits - 1)
    return (value ^ sign) - sign


def words(body: bytes) -> list[int]:
    if len(body) % 4:
        raise ValueError("ARM64 body is not word aligned")
    return [int.from_bytes(body[index : index + 4], "little") for index in range(0, len(body), 4)]


def method_map(capture: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {
        method["methodName"]: method
        for method in capture["record"]["payload"]["methods"]
        if method["className"] == "HunterCtrl"
    }


def exact_body(method: dict[str, Any]) -> tuple[dict[str, Any], bytes]:
    candidates = method["candidates"]
    first = candidates[0]
    body = bytes.fromhex(first["codeHex"])
    if first["codeTruncated"] or len(body) != first["nativeSizeBytes"]:
        raise ValueError(f"{method['methodName']} is not an exact native body")
    for candidate in candidates[1:]:
        if candidate["moduleOffset"] != first["moduleOffset"] or candidate["codeHex"] != first["codeHex"]:
            raise ValueError(f"{method['methodName']} MethodInfo candidates disagree")
    return first, body


def require_words(body: bytes, byte_offset: int, expected: list[int], label: str) -> None:
    actual = words(body[byte_offset : byte_offset + len(expected) * 4])
    if actual != expected:
        raise ValueError(f"{label} instruction sequence changed at 0x{byte_offset:x}")


def bl_target(method_offset: int, instruction_offset: int, instruction: int) -> int:
    if instruction & 0xFC000000 != 0x94000000:
        raise ValueError("expected ARM64 BL instruction")
    return method_offset + instruction_offset + (sign_extend(instruction & 0x03FFFFFF, 26) << 2)


def class_fields(schema: dict[str, Any], class_name: str) -> dict[int, dict[str, Any]]:
    classes = schema["record"]["payload"]["classes"]
    cls = next(row for row in classes if row["name"] == class_name)
    return {field["offset"]: field for field in cls["fields"]}


def referenced_offsets(body: bytes, base_register: int) -> set[int]:
    result: set[int] = set()
    for word in words(body):
        if word & 0xFF000000 == 0x91000000 and ((word >> 5) & 0x1F) == base_register:
            shift = 12 if (word >> 22) & 1 else 0
            result.add(((word >> 10) & 0xFFF) << shift)
            continue
        memory_scales = {
            0xB9000000: 4,
            0xB9400000: 4,
            0xBD000000: 4,
            0xBD400000: 4,
            0xF9000000: 8,
            0xF9400000: 8,
            0x3D800000: 16,
            0x3DC00000: 16,
        }
        scale = memory_scales.get(word & 0xFFC00000)
        if scale is not None and ((word >> 5) & 0x1F) == base_register:
            result.add(((word >> 10) & 0xFFF) * scale)
    return result


def build(
    capture_path: Path,
    target_path: Path,
    refresh_path: Path,
    cadence_path: Path,
    hunter_schema_path: Path,
    status_schema_path: Path,
) -> dict[str, Any]:
    capture = json.loads(capture_path.read_text())
    target_capture = json.loads(target_path.read_text())
    refresh_capture = json.loads(refresh_path.read_text())
    cadence = json.loads(cadence_path.read_text())
    hunter_schema = json.loads(hunter_schema_path.read_text())
    status_schema = json.loads(status_schema_path.read_text())

    methods = method_map(capture)
    bodies: dict[str, bytes] = {}
    candidates: dict[str, dict[str, Any]] = {}
    for name, (module_offset, native_size) in EXPECTED_METHODS.items():
        candidate, body = exact_body(methods[name])
        if candidate["moduleOffset"] != module_offset or candidate["nativeSizeBytes"] != native_size:
            raise ValueError(f"{name} native boundary changed")
        candidates[name] = candidate
        bodies[name] = body

    status_end = candidates["getStatusData"]["boundaryModuleOffset"]
    if status_end != "0x33f79bc":
        raise ValueError("getStatusData boundary no longer ends at the unresolved target")

    target_methods = method_map(target_capture)
    target = target_methods.get("InitHunterHpBar")
    if target is None or target["token"] != 100686865:
        raise ValueError("0x33f79bc did not resolve to HunterCtrl.InitHunterHpBar token 0x06005C11")
    if any(candidate["moduleOffset"] != status_end for candidate in target["candidates"]):
        raise ValueError("resolved target candidates do not match getStatusData boundary")

    hunting = bodies["HuntingAttackAction"]
    require_words(
        hunting,
        0x1A10,
        [
            0x911AB268,  # add x8, x19, #0x6ac
            0xB946BE69,  # ldr w9, [x19, #0x6bc]
            0xBD43DA68,  # ldr s8, [x19, #0x3d8]
            0x3DC00100,  # ldr q0, [x8]
            0x9103C3E0,
            0xAA1F03E1,
            0xB90103E9,
            0x3D803FE0,
            0x97C0F887,  # ObscuredFloat decode helper
            0x1E200908,  # fmul s8, s8, s0
        ],
        "Hunter cadence inputs",
    )
    require_words(
        hunting,
        0x1A58,
        [0x1E281800, 0x1E212100, 0x1E270101, 0x1E21A502, 0xBD47F921, 0x1E218C00, 0xBD01AE60],
        "AttackAniTime branch and write",
    )
    require_words(
        hunting,
        0x1EF4,
        [0xAA1303E0, 0x97FF7BF2, 0xB4000260, 0xB9409808, 0x3CC88000, 0x91065269, 0xB901A668, 0x3D800120],
        "CalcAttackSpeed to mAttackDelay copy",
    )
    get_status_call = bl_target(int(candidates["HuntingAttackAction"]["moduleOffset"], 16), 0x1EF8, words(hunting)[0x1EF8 // 4])
    if get_status_call != int(candidates["getStatusData"]["moduleOffset"], 16):
        raise ValueError("HuntingAttackAction tail no longer calls getStatusData")

    setting = bodies["SettingProperty"]
    require_words(
        setting,
        0x10C,
        [0x911A6268, 0xAA1303E0, 0x3D800100, 0xB906AA69],
        "SettingProperty ObscuredFloat write",
    )

    refresh_method = method_map(refresh_capture)["RefreshAnimation"]
    refresh_candidate, refresh_body = exact_body(refresh_method)
    refresh_requested_accesses = sorted(referenced_offsets(refresh_body, 19) & REQUESTED_OFFSETS)
    if refresh_requested_accesses:
        raise ValueError(f"RefreshAnimation unexpectedly references requested offsets: {refresh_requested_accesses}")

    hunter_fields = class_fields(hunter_schema, "HunterCtrl")
    status_fields = class_fields(status_schema, "StatusData")
    expected_hunter_fields = {
        0x194: ("mAttackDelay", "ObscuredFloat"),
        0x1AC: ("AttackAniTime", "System.Single"),
        0x3D8: ("DANCPPLMKIK", "System.Single"),
        0x698: ("PGDMKPKELMM", "ObscuredFloat"),
        0x6AC: ("BCEBGLKCDHN", "ObscuredFloat"),
    }
    for offset, (name, type_suffix) in expected_hunter_fields.items():
        field = hunter_fields[offset]
        if field["name"] != name or not field["type"].endswith(type_suffix):
            raise ValueError(f"HunterCtrl field schema changed at 0x{offset:x}")
    calc_attack_speed = status_fields[0x88]
    if calc_attack_speed["name"] != "<CalcAttackSpeed>k__BackingField" or not calc_attack_speed["type"].endswith("ObscuredFloat"):
        raise ValueError("StatusData.CalcAttackSpeed schema changed")

    cadence_equation = next(row for row in cadence["exactEquations"] if row["id"] == "hunter-attack-animation-time")
    expected_equation = "composite = DANCPPLMKIK * decode(BCEBGLKCDHN); AttackAniTime = composite > 1.0 ? 0.333 / composite : 0.7"
    if cadence_equation["equation"] != expected_equation:
        raise ValueError("previously normalized cadence equation changed")

    return {
        "schemaVersion": 1,
        "contractType": "original-native-hunter-attack-speed-chain-evidence",
        "runtimeCompatibility": "evidence-only",
        "package": cadence["package"],
        "sources": [
            source(capture_path),
            source(target_path),
            source(refresh_path),
            source(cadence_path),
            source(hunter_schema_path),
            source(status_schema_path),
        ],
        "resolvedBoundaryTarget": {
            "moduleOffset": status_end,
            "type": "HunterCtrl",
            "method": "InitHunterHpBar",
            "token": "0x06005C11",
            "parameterTypes": [],
            "returnType": "System.Void",
            "relationship": "This is the method immediately following getStatusData in the native method table; it is not an attack-speed helper.",
        },
        "methods": [
            {
                "type": "HunterCtrl",
                "method": name,
                "token": f"0x{methods[name]['token']:08X}",
                "moduleOffset": candidates[name]["moduleOffset"],
                "nativeSizeBytes": candidates[name]["nativeSizeBytes"],
                "bodySha256": hashlib.sha256(bodies[name]).hexdigest(),
            }
            for name in EXPECTED_METHODS
        ]
        + [
            {
                "type": "HunterCtrl",
                "method": "RefreshAnimation",
                "token": f"0x{refresh_method['token']:08X}",
                "moduleOffset": refresh_candidate["moduleOffset"],
                "nativeSizeBytes": refresh_candidate["nativeSizeBytes"],
                "bodySha256": hashlib.sha256(refresh_body).hexdigest(),
            }
        ],
        "fieldChain": {
            "cadenceInputs": [
                {"owner": "HunterCtrl", "name": "DANCPPLMKIK", "offset": 0x3D8, "type": "System.Single", "access": "read"},
                {"owner": "HunterCtrl", "name": "BCEBGLKCDHN", "offset": 0x6AC, "type": "ObscuredFloat", "access": "read and decode"},
            ],
            "cadenceOutput": {"owner": "HunterCtrl", "name": "AttackAniTime", "offset": 0x1AC, "type": "System.Single", "access": "write"},
            "equation": expected_equation,
            "statusCopy": {
                "source": {"owner": "StatusData", "name": "CalcAttackSpeed", "offset": 0x88, "type": "ObscuredFloat"},
                "destination": {"owner": "HunterCtrl", "name": "mAttackDelay", "offset": 0x194, "type": "ObscuredFloat"},
                "copyBytes": 20,
                "nativeOperations": "ldur q0 [status+0x88] and ldr w8 [status+0x98], then str q0 [hunter+0x194] and str w8 [hunter+0x1A4]",
                "semanticStatus": "exact raw ObscuredFloat copy confirmed",
            },
        },
        "negativeFindings": [
            {
                "method": "HunterCtrl.SettingProperty",
                "finding": "writes PGDMKPKELMM at 0x698..0x6A8, not BCEBGLKCDHN at 0x6AC",
                "scope": "captured exact method body",
            },
            {
                "method": "HunterCtrl.RefreshAnimation",
                "finding": "does not reference mAttackDelay, AttackAniTime, DANCPPLMKIK, or BCEBGLKCDHN offsets",
                "scope": "captured exact method body",
            },
        ],
        "unresolved": [
            "The writers and semantic sources of HunterCtrl.DANCPPLMKIK and HunterCtrl.BCEBGLKCDHN are not present in the selected exact method bodies.",
            "The formula that produces StatusData.CalcAttackSpeed is not recovered by this pass.",
            "The reader that turns HunterCtrl.mAttackDelay into an attack-FSM wait or gate is not recovered by this pass.",
            "Gear-index-to-Spine weapon-skin binding and target-axis facing remain governed by the separate weapon-presentation evidence and are not resolved here.",
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture", type=Path, default=DEFAULT_CAPTURE)
    parser.add_argument("--target", type=Path, default=DEFAULT_TARGET)
    parser.add_argument("--refresh", type=Path, default=DEFAULT_REFRESH)
    parser.add_argument("--cadence", type=Path, default=DEFAULT_CADENCE)
    parser.add_argument("--hunter-schema", type=Path, default=DEFAULT_HUNTER_SCHEMA)
    parser.add_argument("--status-schema", type=Path, default=DEFAULT_STATUS_SCHEMA)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    result = build(args.capture, args.target, args.refresh, args.cadence, args.hunter_schema, args.status_schema)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote {args.output}")


if __name__ == "__main__":
    main()
