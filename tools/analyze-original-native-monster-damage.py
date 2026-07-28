#!/usr/bin/env python3
"""Verify the common EvilCtrl.Damaged arithmetic applied to Hunter attacks."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs


def disassemble(code_hex: str, address: int) -> list[str]:
    decoder = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    return [
        f"0x{instruction.address:x}: {instruction.mnemonic} {instruction.op_str}".rstrip()
        for instruction in decoder.disasm(bytes.fromhex(code_hex), address)
    ]


def require(lines: list[str], fragments: list[str]) -> None:
    text = "\n".join(lines)
    missing = [fragment for fragment in fragments if fragment not in text]
    if missing:
        raise ValueError(f"EvilCtrl.Damaged missing native patterns: {missing}")


def find_class(payload: dict, class_name: str) -> dict:
    pending = [payload]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            if value.get("name") == class_name and isinstance(value.get("fields"), list):
                return value
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)
    raise ValueError(f"class schema not found: {class_name}")


def field_at(class_schema: dict, offset: int) -> dict:
    return next(field for field in class_schema["fields"] if field["offset"] == offset)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture", type=Path, required=True)
    parser.add_argument("--evil-schema", type=Path, required=True)
    parser.add_argument("--hunter-schema", type=Path, required=True)
    parser.add_argument("--status-schema", type=Path, required=True)
    parser.add_argument("--static-factors", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    capture = json.loads(args.capture.read_text())
    evil_schema_payload = json.loads(args.evil_schema.read_text())
    hunter_schema_payload = json.loads(args.hunter_schema.read_text())
    status_schema_payload = json.loads(args.status_schema.read_text())
    static_payload = json.loads(args.static_factors.read_text())

    method = next(
        item
        for item in capture["record"]["payload"]["methods"]
        if item["className"] == "EvilCtrl" and item["methodName"] == "Damaged"
    )
    candidate = method["candidates"][0]
    if method["token"] != 100675604 or candidate["moduleOffset"] != "0x2f2be20":
        raise ValueError("unexpected EvilCtrl.Damaged identity")
    if candidate["nativeSizeBytes"] != 4736 or candidate["codeTruncated"]:
        raise ValueError("EvilCtrl.Damaged is not a complete 4736-byte body")
    lines = disassemble(candidate["codeHex"], int(candidate["moduleOffset"], 16))
    require(
        lines,
        [
            "bl #0x2706384",
            "ldr s2, [x21, #0x1e0]",
            "ldr s1, [x21, #0x1dc]",
            "ldr x8, [x25, #0x210]",
            "ldr x8, [x25, #0x238]",
            "add x10, x24, #0x2c4",
            "sub x0, x26, x8",
            "sub x9, x28, x0",
            "sub x0, x0, #1",
        ],
    )

    evil_data = find_class(evil_schema_payload, "EvilData")
    hunter_data = find_class(hunter_schema_payload, "HunterData")
    status_data = find_class(status_schema_payload, "StatusData")
    expected_fields = {
        "EvilData.armor": field_at(evil_data, 208)["name"],
        "EvilData.nowHp": field_at(evil_data, 240)["name"],
        "HunterData.feel": field_at(hunter_data, 392)["name"],
        "HunterData.nowFeel": field_at(hunter_data, 412)["name"],
        "HunterData.revivePenetrate": field_at(hunter_data, 708)["name"],
        "StatusData.GearProperty": field_at(status_data, 528)["name"],
        "StatusData.RidingPetGearProperty": field_at(status_data, 568)["name"],
    }
    if expected_fields != {
        "EvilData.armor": "<armor>k__BackingField",
        "EvilData.nowHp": "<nowHp>k__BackingField",
        "HunterData.feel": "<feel>k__BackingField",
        "HunterData.nowFeel": "<nowFeel>k__BackingField",
        "HunterData.revivePenetrate": "<revivePenetrate>k__BackingField",
        "StatusData.GearProperty": "<GearProperty>k__BackingField",
        "StatusData.RidingPetGearProperty": "<RidingPetGearProperty>k__BackingField",
    }:
        raise ValueError(f"combat schema offsets changed: {expected_fields}")

    factors = static_payload["monsterDamageFactors"]
    expected_raw = {
        "feel_ratio_80_percent": "cdcc4c3f",
        "feel_ratio_60_percent": "9a99193f",
        "feel_ratio_40_percent": "cdcccc3e",
        "feel_ratio_20_percent": "cdcc4c3e",
        "integer_percent_scale": "0ad7233c",
    }
    for name, raw_hex in expected_raw.items():
        if factors[name]["rawHex"] != raw_hex:
            raise ValueError(f"monster damage factor changed: {name}")

    output = {
        "schemaVersion": 1,
        "contractType": "original-native-monster-damage-analysis",
        "runtimeCompatibility": "evidence-only",
        "source": {
            "capture": str(args.capture),
            "captureSha256": hashlib.sha256(args.capture.read_bytes()).hexdigest(),
            "schemas": [
                {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
                for path in (args.evil_schema, args.hunter_schema, args.status_schema)
            ],
            "staticFactors": str(args.static_factors),
            "staticFactorsSha256": hashlib.sha256(args.static_factors.read_bytes()).hexdigest(),
        },
        "method": {
            "className": "EvilCtrl",
            "methodName": "Damaged",
            "token": 100675604,
            "moduleOffset": "0x2f2be20",
            "nativeSizeBytes": 4736,
            "bodySha256": hashlib.sha256(bytes.fromhex(candidate["codeHex"])).hexdigest(),
        },
        "resolvedCommonArithmetic": {
            "feelScalar": {
                "source": "HunterData.nowFeel / HunterData.feel",
                "thresholds": [0.8, 0.6, 0.4, 0.2],
                "selectedValues": [1.2, 1.1, 1.0, 0.9, 0.8],
                "comparison": "descending >= using float32 products",
            },
            "initialDamage": "trunc_i64(float32(incomingDamage) * feelScalar * GameManager.RandDamage())",
            "directBonus": "if float32(field_0x1DC + field_0x1E0) > f32::from_bits(1): damage = trunc_i64(float32(damage) * float32(1 + sum))",
            "preArmorBonus": "damage += trunc_i64(float32(damage) * preArmorBonusRate)",
            "effectiveArmor": "parameter6 ? 0 : max(armor - trunc_i64(float32(armor) * armorReductionRate), 0)",
            "minimumDamage": "max_by_branch(damage - effectiveArmor, 1)",
            "hpMutation": "EvilData.nowHp = wrapping_i64(EvilData.nowHp - finalDamage)",
        },
        "resolvedSelectors": {
            "preArmorBonusSources": [
                "StatusData.GearProperty[51][0] under the recovered <=50% monster HP gate",
                "StatusData.RidingPetGearProperty[12] under its captured runtime gate",
                "StatusData.RidingPetGearProperty[16] when parameter6 is true",
                "EvilCtrl.field_0x1F8 when parameter3 equals 2",
            ],
            "armorReductionSources": [
                "StatusData.GearProperty[37][0]",
                "otherwise StatusData.GearProperty[61][0]",
                "HunterData.revivePenetrate rows 1..5 through DataManager revive-property data",
            ],
            "integerPercentScale": factors["integer_percent_scale"]["float32"],
        },
        "unresolvedSemantics": [
            "Public names for EvilCtrl fields 0x1DC, 0x1E0 and 0x1F8.",
            "Public names and caller meaning of parameters 3 through 6.",
            "Product names for GearProperty indices 37, 51 and 61 and RidingPetGearProperty indices 12 and 16.",
            "All alternate presentation, life-steal and death-event side effects after the common HP mutation.",
        ],
        "integrationStatus": "common_arithmetic_core_ready_but_live_integration_blocked_by_outgoing_caller_and_skill_contracts",
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote monster damage analysis to {args.output}")


if __name__ == "__main__":
    main()
