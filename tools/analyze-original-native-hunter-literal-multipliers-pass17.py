#!/usr/bin/env python3
"""Classify exact literal-multiplier Hunter getDamage caller bodies."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs


ROOT = Path(__file__).resolve().parents[1]
CALLERS = ROOT / "reverse-engineering/evidence/original-native-hunter-getdamage-callers-api35-v1.json"
ACTIONS = ROOT / "reverse-engineering/evidence/original-native-hunter-skill-coefficient-action-targets-api35-v1.json"
CONSTANT_SCHEMA = ROOT / "reverse-engineering/evidence/constant-data-runtime-schema-api35-v1.json"
OUTPUT = ROOT / "reverse-engineering/evidence/original-native-hunter-literal-multipliers-pass17.json"


def source(path: Path) -> dict:
    raw = path.read_bytes()
    return {"path": path.relative_to(ROOT).as_posix(), "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def methods(path: Path) -> list[dict]:
    return json.loads(path.read_text())["record"]["payload"]["methods"]


def find(rows: list[dict], class_name: str, method_name: str) -> dict:
    return next(row for row in rows if row["className"] == class_name and row["methodName"] == method_name)


def decode(method: dict) -> tuple[bytes, list]:
    candidate = method["candidates"][0]
    raw = bytes.fromhex(candidate["codeHex"])
    if candidate["codeTruncated"] or candidate["nativeSizeBytes"] != len(raw):
        raise ValueError(f"incomplete exact body: {method['methodName']}")
    return raw, list(Cs(CS_ARCH_ARM64, CS_MODE_ARM).disasm(raw, int(candidate["moduleOffset"], 16)))


def descriptor(method: dict) -> dict:
    raw, _ = decode(method)
    candidate = method["candidates"][0]
    return {
        "className": method["className"], "methodName": method["methodName"],
        "parameterTypes": method["parameterTypes"], "token": method["token"],
        "moduleOffset": candidate["moduleOffset"], "nativeSizeBytes": len(raw),
        "bodySha256": hashlib.sha256(raw).hexdigest(),
    }


def require(method: dict, anchors: list[tuple[int, str, str]]) -> None:
    _, instructions = decode(method)
    observed = {(row.address, row.mnemonic, row.op_str) for row in instructions}
    missing = [anchor for anchor in anchors if anchor not in observed]
    if missing:
        raise ValueError(f"native anchors changed for {method['methodName']}: {missing}")


def constant_fields(document: dict) -> dict[int, dict]:
    pending = [document]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            if value.get("name") == "ConstantData" and isinstance(value.get("fields"), list):
                return {field["offset"]: field for field in value["fields"]}
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)
    raise ValueError("ConstantData schema not found")


def build(callers_path: Path, actions_path: Path, constant_schema_path: Path) -> dict:
    caller_rows = methods(callers_path)
    action_rows = methods(actions_path)
    constants = constant_fields(json.loads(constant_schema_path.read_text()))
    expected_objects = {0x1158: "DIVINEATTACK_OBJ_NAME", 0x1160: "FLAMEEXPLOSION_OBJ_NAME"}
    for offset, name in expected_objects.items():
        field = constants[offset]
        if field["name"] != name or field["type"] != "System.String":
            raise ValueError(f"ConstantData object-name field changed at 0x{offset:X}")

    specs = {
        "MEDDIMPJHDA": [
            (0x3437268, "mov", "w1, #1"), (0x343726C, "mov", "w2, wzr"),
            (0x3437270, "mov", "w3, #1"), (0x3437274, "bl", "#0x33f51c4"),
            (0x3437278, "mov", "w8, #0x438f0000"), (0x343728C, "bl", "#0x245787c"),
            (0x34372B4, "bl", "#0x24567e8"), (0x34372D4, "bl", "#0x245668c"),
            (0x34372F8, "fmul", "s0, s0, s1"), (0x3437310, "fcvtzs", "x10, s0"),
            (0x34373A0, "ldr", "x24, [x8, #0x1160]"), (0x34373A4, "ldp", "x8, x1, [x21, #0x30]"),
            (0x34373AC, "bl", "#0x245672c"), (0x34373B4, "mov", "w5, w0"),
            (0x34373C8, "mov", "x4, x22"), (0x34373CC, "mov", "w6, #1"),
            (0x34373D4, "bl", "#0x2c572b0"),
        ],
        "BOKBBDIDLJG": [
            (0x3420CFC, "mov", "w1, #1"), (0x3420D00, "mov", "w2, wzr"),
            (0x3420D04, "mov", "w3, #1"), (0x3420D08, "bl", "#0x33f51c4"),
            (0x3420D0C, "fmov", "s0, #5.00000000"), (0x3420D1C, "bl", "#0x245787c"),
            (0x3420D44, "bl", "#0x24567e8"), (0x3420D64, "bl", "#0x245668c"),
            (0x3420D88, "fmul", "s0, s0, s1"), (0x3420DA0, "fcvtzs", "x10, s0"),
            (0x3420E2C, "ldr", "x24, [x8, #0x1160]"), (0x3420E30, "ldp", "x8, x1, [x21, #0x30]"),
            (0x3420E38, "bl", "#0x245672c"), (0x3420E40, "mov", "w5, w0"),
            (0x3420E54, "mov", "x4, x22"), (0x3420E58, "mov", "w6, wzr"),
            (0x3420E60, "bl", "#0x2c572b0"),
        ],
        "KKHDNNMAOKA": [
            (0x3420258, "mov", "w1, #1"), (0x342025C, "mov", "w2, wzr"),
            (0x3420260, "mov", "w3, #1"), (0x3420264, "bl", "#0x33f51c4"),
            (0x3420268, "fmov", "s0, #3.00000000"), (0x3420278, "bl", "#0x245787c"),
            (0x34202A0, "bl", "#0x24567e8"), (0x34202C0, "bl", "#0x245668c"),
            (0x34202E4, "fmul", "s0, s0, s1"), (0x34202FC, "fcvtzs", "x10, s0"),
            (0x3420324, "ldr", "x24, [x9, #0x1158]"), (0x3420374, "ldp", "x8, x1, [x21, #0x30]"),
            (0x3420388, "bl", "#0x245672c"), (0x3420390, "mov", "w4, w0"),
            (0x34203A0, "mov", "x3, x22"), (0x34203A8, "bl", "#0x2b2b734"),
        ],
    }
    selected = {}
    for name, anchors in specs.items():
        method = find(caller_rows, "HunterCtrl", name)
        if method["parameterTypes"] != ["EvilCtrl"]:
            raise ValueError(f"signature changed for {name}")
        require(method, anchors)
        selected[name] = descriptor(method)

    flame = descriptor(find(action_rows, "FlameExplosionCtrl", "Action"))
    flame_object = {**constants[0x1160], "runtimeDataOffset": "0x1160"}
    divine_object = {**constants[0x1158], "runtimeDataOffset": "0x1158"}

    return {
        "schemaVersion": 1,
        "contractType": "original-native-hunter-literal-multipliers-pass17",
        "runtimeCompatibility": "evidence-only-disconnected",
        "sources": [source(callers_path), source(actions_path), source(constant_schema_path)],
        "family": {
            "id": "literal-obscured-float-roundtrip-multiplier",
            "pipeline": "initialize ObscuredFloat from exact inline float32 literal; decode getDamage result member at +0x10; decode multiplier; multiply as float32; FCVTZS",
            "rounding": "final FCVTZS truncates toward zero",
            "members": [
                {
                    "method": selected["MEDDIMPJHDA"], "getDamageVector": [True, False, True],
                    "literalMultiplier": {"instructionBits": "0x438f0000", "float32": 286.0},
                    "equation": "damage = trunc_i64(float32(baseDamage) * 286.0f)",
                    "action": {"target": "FlameExplosionCtrl.Action", "objectNameSource": flame_object, "damageParameter": 4, "selectorParameter6": 1},
                    "resultPayload": {"sourceOffset": "getDamage result +0x30", "operation": "decoded and forwarded unchanged", "actionParameter": 5, "semanticStatus": "unresolved discriminator; no normal/critical enum label assigned"},
                },
                {
                    "method": selected["BOKBBDIDLJG"], "getDamageVector": [True, False, True],
                    "literalMultiplier": {"encoding": "ARM64 immediate", "float32": 5.0},
                    "equation": "damage = trunc_i64(float32(baseDamage) * 5.0f)",
                    "action": {"target": "FlameExplosionCtrl.Action", "objectNameSource": flame_object, "damageParameter": 4, "selectorParameter6": 0},
                    "resultPayload": {"sourceOffset": "getDamage result +0x30", "operation": "decoded and forwarded unchanged", "actionParameter": 5, "semanticStatus": "unresolved discriminator; no normal/critical enum label assigned"},
                },
                {
                    "method": selected["KKHDNNMAOKA"], "getDamageVector": [True, False, True],
                    "literalMultiplier": {"encoding": "ARM64 immediate", "float32": 3.0},
                    "equation": "damage = trunc_i64(float32(baseDamage) * 3.0f)",
                    "action": {"targetModuleOffset": "0x2b2b734", "managedIdentity": None, "objectNameSource": divine_object, "damageArgumentRegister": "x3"},
                    "resultPayload": {"sourceOffset": "getDamage result +0x30", "operation": "decoded and forwarded unchanged", "actionArgumentRegister": "w4", "semanticStatus": "unresolved discriminator; no normal/critical enum label assigned"},
                },
            ],
        },
        "resolvedActionTargets": {"flameExplosion": flame},
        "classification": {
            "arithmeticClassifiedThisPass": 3,
            "fullyClosedCallersThisPass": 0,
            "reasonNotFullyClosed": "public skill mappings are unresolved and target 0x2b2b734 lacks a managed identity; the +0x30 result discriminator has no proven enum meaning",
        },
        "unresolved": [
            "managed identity and parameter contract of native target 0x2b2b734",
            "field names and enum values of HunterCtrl.PMPKOIHFNCE result members at +0x10 and +0x30",
            "whether any +0x30 discriminator value corresponds to normal, critical, or another presentation category",
            "public skill-row mappings for all three obfuscated methods",
        ],
        "integrationStatus": "disconnected_no_result_enum_guess_or_live_combat_use",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--callers", type=Path, default=CALLERS)
    parser.add_argument("--actions", type=Path, default=ACTIONS)
    parser.add_argument("--constant-schema", type=Path, default=CONSTANT_SCHEMA)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    args.output.write_text(json.dumps(build(args.callers, args.actions, args.constant_schema), ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote Hunter literal multiplier Pass17 evidence to {args.output}")


if __name__ == "__main__":
    main()
