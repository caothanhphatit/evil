#!/usr/bin/env python3
"""Recover the internal ObscuredInt-percent Hunter damage caller cluster."""

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
OUTPUT = ROOT / "reverse-engineering/evidence/original-native-hunter-skill-coefficients-pass13.json"


def source(path: Path) -> dict:
    raw = path.read_bytes()
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "bytes": len(raw),
        "sha256": hashlib.sha256(raw).hexdigest(),
    }


def methods(path: Path) -> list[dict]:
    return json.loads(path.read_text())["record"]["payload"]["methods"]


def find(rows: list[dict], class_name: str, method_name: str) -> dict:
    return next(
        row
        for row in rows
        if row["className"] == class_name and row["methodName"] == method_name
    )


def decode(method: dict) -> tuple[dict, bytes, list]:
    candidate = method["candidates"][0]
    raw = bytes.fromhex(candidate["codeHex"])
    if candidate["codeTruncated"] or candidate["nativeSizeBytes"] != len(raw):
        raise ValueError(f"incomplete exact body: {method['methodName']}")
    instructions = list(
        Cs(CS_ARCH_ARM64, CS_MODE_ARM).disasm(raw, int(candidate["moduleOffset"], 16))
    )
    return candidate, raw, instructions


def descriptor(method: dict) -> dict:
    candidate, raw, _ = decode(method)
    return {
        "className": method["className"],
        "methodName": method["methodName"],
        "parameterTypes": method["parameterTypes"],
        "token": method["token"],
        "moduleOffset": candidate["moduleOffset"],
        "nativeSizeBytes": len(raw),
        "bodySha256": hashlib.sha256(raw).hexdigest(),
    }


def require(method: dict, anchors: list[tuple[int, str, str]]) -> None:
    _, _, instructions = decode(method)
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
    flame = find(action_rows, "FlameExplosionCtrl", "Action")
    constants = constant_fields(json.loads(constant_schema_path.read_text()))
    expected_constants = {
        0x338C: "BLOW_DESTRUCTION_POWER_VALUE",
        0x2830: "VENUM_RAIN_POWER_VALUE",
        0x2C98: "CURSE_CHAIN_POWER_VALUE",
        0x2850: "DARK_RIFT_POWER_VALUE",
        0x33BC: "POISON_FANG_POWER_VALUE",
    }
    for offset, name in expected_constants.items():
        field = constants[offset]
        if field["name"] != name or field["type"] != "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt":
            raise ValueError(f"ConstantData field changed at 0x{offset:X}")

    specs = [
        (
            "KMFIIOFLHKC", 0x338C, 0,
            [(0x3409960, "mov", "w9, #0x338c"), (0x3409970, "bl", "#0x245672c"),
             (0x3409988, "fmul", "s0, s0, s1"), (0x340998C, "bl", "#0x245787c"),
             (0x34099B4, "bl", "#0x24567e8"), (0x34099D4, "bl", "#0x245668c"),
             (0x34099F8, "fmul", "s0, s0, s1"), (0x3409A14, "fcvtzs", "x11, s0"),
             (0x3409AA0, "mov", "x4, x22"), (0x3409AA4, "mov", "w6, wzr"),
             (0x3409AAC, "bl", "#0x2c572b0")],
        ),
        (
            "BBOCACECAAO", 0x2830, 1,
            [(0x340D2BC, "ldr", "x0, [x8, #0x2830]"), (0x340D2C4, "bl", "#0x245672c"),
             (0x340D2DC, "fmul", "s0, s0, s1"), (0x340D2E0, "bl", "#0x245787c"),
             (0x340D308, "bl", "#0x24567e8"), (0x340D328, "bl", "#0x245668c"),
             (0x340D34C, "fmul", "s0, s0, s1"), (0x340D370, "fcvtzs", "x12, s0"),
             (0x340D3FC, "mov", "x4, x22"), (0x340D400, "mov", "w6, #1"),
             (0x340D408, "bl", "#0x2c572b0")],
        ),
        (
            "CHKMAHCLJBN", 0x2C98, 2,
            [(0x3421F98, "ldr", "x0, [x8, #0x2c98]"), (0x3421FA0, "bl", "#0x245672c"),
             (0x3421FB8, "fmul", "s0, s0, s1"), (0x3421FBC, "bl", "#0x245787c"),
             (0x3421FE4, "bl", "#0x24567e8"), (0x3422004, "bl", "#0x245668c"),
             (0x3422028, "fmul", "s0, s0, s1"), (0x342204C, "fcvtzs", "x12, s0"),
             (0x34220D8, "mov", "x4, x22"), (0x34220DC, "mov", "w6, #2"),
             (0x34220E4, "bl", "#0x2c572b0")],
        ),
        (
            "BCLCCDFCHFC", 0x2850, 0,
            [(0x34212D8, "ldr", "x0, [x8, #0x2850]"), (0x34212E0, "bl", "#0x245672c"),
             (0x34212F8, "fmul", "s0, s0, s1"), (0x34212FC, "bl", "#0x245787c"),
             (0x3421324, "bl", "#0x24567e8"), (0x3421344, "bl", "#0x245668c"),
             (0x3421368, "fmul", "s0, s0, s1"), (0x342138C, "fcvtzs", "x12, s0"),
             (0x3421418, "mov", "x4, x22"), (0x342141C, "mov", "w6, wzr"),
             (0x3421424, "bl", "#0x2c572b0")],
        ),
        (
            "GHFOIEIIDDF", None, 0,
            [(0x34635A0, "bl", "#0x245672c"), (0x34635B8, "fmul", "s0, s0, s1"),
             (0x34635BC, "bl", "#0x245787c"), (0x34635E4, "bl", "#0x24567e8"),
             (0x3463604, "bl", "#0x245668c"), (0x3463628, "fmul", "s0, s0, s1"),
             (0x3463644, "fcvtzs", "x11, s0"), (0x34636D0, "mov", "x4, x22"),
             (0x34636D4, "mov", "w6, wzr"), (0x34636DC, "bl", "#0x2c572b0")],
        ),
    ]

    members = []
    for method_name, constant_offset, selector, anchors in specs:
        method = find(caller_rows, "HunterCtrl", method_name)
        if method["parameterTypes"] != ["EvilCtrl"]:
            raise ValueError(f"signature changed for {method_name}")
        require(method, anchors)
        coefficient_source = (
            {
                "kind": "ConstantData.ObscuredInt",
                "field": expected_constants[constant_offset],
                "runtimeDataOffset": f"0x{constant_offset:X}",
            }
            if constant_offset is not None
            else {
                "kind": "nested ObscuredInt collection lookup",
                "field": None,
                "semanticStatus": "collection and index path captured; product name unresolved",
            }
        )
        members.append(
            {
                "method": descriptor(method),
                "coefficientSource": coefficient_source,
                "action": "FlameExplosionCtrl.Action",
                "damageParameter": 4,
                "actionSelector": {"parameter6": selector},
            }
        )

    poison = find(caller_rows, "HunterCtrl", "IABOOKJBHHO")
    if poison["parameterTypes"] != ["EvilCtrl"]:
        raise ValueError("IABOOKJBHHO signature changed")
    require(poison, [
        (0x34222E4, "mov", "w9, #0x33bc"), (0x34222F4, "bl", "#0x245672c"),
        (0x342230C, "fmul", "s0, s0, s1"), (0x3422310, "bl", "#0x245787c"),
        (0x3422338, "bl", "#0x24567e8"), (0x3422358, "bl", "#0x245668c"),
        (0x3422370, "fmul", "s0, s0, s1"), (0x342237C, "fcvtzs", "x9, s0"),
        (0x34223BC, "mov", "x2, x21"), (0x34223CC, "mov", "w4, wzr"),
        (0x34223D0, "mov", "w6, wzr"), (0x34223D4, "blr", "x9"),
    ])
    members.append({
        "method": descriptor(poison),
        "coefficientSource": {
            "kind": "ConstantData.ObscuredInt",
            "field": expected_constants[0x33BC],
            "runtimeDataOffset": "0x33BC",
        },
        "action": "EvilCtrl virtual slot +0x2A8",
        "managedActionTarget": None,
        "damageParameter": 2,
        "damagedVectorTail": {"parameter4": False, "parameter5": "runtime value", "parameter6": False},
    })

    return {
        "schemaVersion": 1,
        "contractType": "original-native-hunter-skill-coefficients-pass13",
        "runtimeCompatibility": "evidence-only-disconnected",
        "sources": [source(callers_path), source(actions_path), source(constant_schema_path)],
        "actionTarget": descriptor(flame),
        "family": {
            "id": "internal-obscured-int-percent",
            "equation": "percent = float32(decode(sourceObscuredInt)) * 0.01f; damage = trunc_i64(float32(baseDamage) * percent)",
            "rounding": "all multiplication is float32; final FCVTZS truncates toward zero",
            "obscuredRoundTrip": "percent is wrapped as ObscuredFloat and decoded before the final multiplication",
            "members": members,
        },
        "coverage": {
            "exactGetDamageCallerBodies": 49,
            "coefficientMembersResolvedPass9And11": 9,
            "coefficientMembersResolvedThisPass": 6,
            "remainingCallerBodies": 34,
        },
        "unresolved": [
            "public skill-row mappings for all six obfuscated methods",
            "semantic collection/index identity used by GHFOIEIIDDF",
            "managed identity of the EvilCtrl vtable +0x2A8 target",
            "runtime meaning of IABOOKJBHHO parameter 5 forwarded to the virtual target",
            "coefficient producers for the remaining thirty-four getDamage callers",
        ],
        "integrationStatus": "disconnected_no_public_skill_mapping_or_live_combat_use",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--callers", type=Path, default=CALLERS)
    parser.add_argument("--actions", type=Path, default=ACTIONS)
    parser.add_argument("--constant-schema", type=Path, default=CONSTANT_SCHEMA)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    args.output.write_text(
        json.dumps(build(args.callers, args.actions, args.constant_schema), ensure_ascii=True, indent=2) + "\n"
    )
    print(f"Wrote Hunter skill coefficient Pass13 evidence to {args.output}")


if __name__ == "__main__":
    main()
