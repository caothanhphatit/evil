#!/usr/bin/env python3
"""Normalize the proven pass-2 GearData combat-formula boundaries."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CAPTURE = ROOT / "reverse-engineering/evidence/original-native-combat-formula-exact-boundaries-api35-v1.json"
DEFAULT_ANALYSIS = ROOT / "reverse-engineering/evidence/original-native-combat-formula-analysis-v1.json"
DEFAULT_SCHEMA = ROOT / "reverse-engineering/evidence/hunter-domain-runtime-schema-android-api30-v1.json"
DEFAULT_ADMIN_SCHEMA = ROOT / "reverse-engineering/evidence/admin-gear-formula-runtime-schema-api35-v1.json"
DEFAULT_FIRST_PERCENT = ROOT / "reverse-engineering/evidence/original-native-get-first-percent-api35-v1.json"
DEFAULT_OUTPUT = ROOT / "reverse-engineering/evidence/original-native-gear-formula-analysis-v1.json"

GEAR_METHODS = {"GetGearDamage": 1272, "GetGearAcc": 1124, "GetGearArmor": 1124, "GetSealAttackUp": 492}
GEAR_FIELDS = {16: "index", 32: "gearIndex", 64: "quality", 92: "level", 108: "rating"}
QUALITY_MULTIPLIERS = {0: 0.8, 1: 0.9, 3: 1.1, 4: 1.2, "other": 1.0}
SEAL_ACCEPTED = list(range(157, 162)) + [202, 255, 320, 359, 476, 489, 531, 706, 758, 822]
WEAPON_COSTUME_CAPTURE = {
    "token": "0x06006B36",
    "moduleOffset": "0x271fd38",
    "nativeSizeBytes": 296,
    "bodySha256": "bcc55d3a563208f7d2011d43abc2f43d9602a41971266b2d40071cd535f53839",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def method_map(capture: dict) -> dict[str, dict]:
    return {row["methodName"]: row for row in capture["record"]["payload"]["methods"] if row["className"] == "GameManager"}


def field_schema(schema: dict) -> dict[int, dict]:
    gear = next(row for row in schema["record"]["payload"]["classes"] if row["name"] == "GearData")
    return {row["offset"]: {"runtimeName": row["name"], "type": row["type"]} for row in gear["fields"]}


def ties_to_even(value: float) -> int:
    return round(value)


def common_armor_acc(base: float, rating_percent: float, level_percent: float, quality: int) -> int:
    multiplier = QUALITY_MULTIPLIERS.get(quality, QUALITY_MULTIPLIERS["other"])
    return ties_to_even(base * (rating_percent / 100.0) * (1.0 + level_percent / 100.0) * multiplier)


def first_percent(values: list[int], limit: int) -> int:
    if limit < 0:
        return 0
    total = 0
    for step in range(limit + 1):
        if 1 <= step <= 20:
            index = (step - 1) // 5
        elif 21 <= step <= 25:
            index = step - 17
        else:
            continue
        if index < len(values):
            total += values[index]
    return total


def gear_damage(first_value: float, rating_percent: float, level_percent: float, quality: int, second_value: float) -> int:
    multiplier = QUALITY_MULTIPLIERS.get(quality, QUALITY_MULTIPLIERS["other"])
    return ties_to_even(first_value * (rating_percent / 100.0) * (1.0 + level_percent / 100.0) * multiplier * (second_value / 100.0))


def build(capture_path: Path, analysis_path: Path, schema_path: Path, admin_schema_path: Path = DEFAULT_ADMIN_SCHEMA, first_percent_path: Path = DEFAULT_FIRST_PERCENT) -> dict:
    capture = json.loads(capture_path.read_text())
    analysis = json.loads(analysis_path.read_text())
    schema = json.loads(schema_path.read_text())
    admin_schema = json.loads(admin_schema_path.read_text())
    first_percent_evidence = json.loads(first_percent_path.read_text())
    methods = method_map(capture)
    normalized = {(row["type"], row["method"]): row for row in analysis["methods"]}
    fields = field_schema(schema)

    for name, expected_size in GEAR_METHODS.items():
        candidate = methods[name]["candidates"][0]
        if candidate["nativeSizeBytes"] != expected_size or candidate["codeTruncated"]:
            raise ValueError(f"{name} exact boundary changed")

    mapped_fields = [{"semanticName": name, "offset": offset, **fields[offset]} for offset, name in GEAR_FIELDS.items()]
    golden = [
        {"input": [100, 100, 0, 2], "result": common_armor_acc(100, 100, 0, 2)},
        {"input": [100, 80, 25, 0], "result": common_armor_acc(100, 80, 25, 0)},
        {"input": [100, 110, 10, 4], "result": common_armor_acc(100, 110, 10, 4)},
        {"input": [10.5, 100, 0, 2], "result": common_armor_acc(10.5, 100, 0, 2)},
        {"input": [11.5, 100, 0, 2], "result": common_armor_acc(11.5, 100, 0, 2)},
    ]

    return {
        "schemaVersion": 1,
        "contractType": "original-native-gear-formula-analysis",
        "runtimeCompatibility": "evidence-only",
        "sources": [
            {"path": capture_path.relative_to(ROOT).as_posix(), "sha256": sha256(capture_path)},
            {"path": analysis_path.relative_to(ROOT).as_posix(), "sha256": sha256(analysis_path)},
            {"path": schema_path.relative_to(ROOT).as_posix(), "sha256": sha256(schema_path)},
            {"path": admin_schema_path.relative_to(ROOT).as_posix(), "sha256": sha256(admin_schema_path)},
            {"path": first_percent_path.relative_to(ROOT).as_posix(), "sha256": sha256(first_percent_path)},
        ],
        "gearDataInput": {"argumentRegister": "x1", "stableAliasRegister": "x20", "fields": mapped_fields},
        "methods": [
            {
                "method": name,
                "token": f"0x{methods[name]['token']:08X}",
                "nativeSizeBytes": GEAR_METHODS[name],
                "bodySha256": normalized[("GameManager", name)]["bodySha256"],
                "directCalls": normalized[("GameManager", name)]["directCalls"],
            }
            for name in GEAR_METHODS
        ],
        "gearArmorAndAcc": {
            "sharedControlFlow": "The two bodies have the same field, multiplier, percentage-addition, quality, and rounding structure; their table/helper targets differ.",
            "neutralExpression": "roundToEven(base * ratingPercent/100 * (1 + levelPercent/100) * qualityMultiplier)",
            "qualityMultipliers": QUALITY_MULTIPLIERS,
            "rounding": "nearest integer with midpoint ties to even",
            "goldenVectors": golden,
            "adminFields": {"base": "firstValue", "rating": "ratingValue", "levelPercent": "firstPercent"},
        },
        "gearDamage": {
            "confirmedFields": list(GEAR_FIELDS.values()),
            "confirmedConstants": [0.01, 0.8, 0.9, 1.0, 1.1, 1.2],
            "rounding": "nearest integer with midpoint ties to even",
            "helper": first_percent_evidence["method"],
            "adminFields": {"base": "firstValue", "rating": "ratingValue", "levelPercent": "firstPercent", "finalPercent": "secondValue"},
            "ratingSelection": "ratingValue[min(GearData.rating, ratingValue.length - 1)]",
            "expression": "roundToEven(firstValue * ratingValue[rating]/100 * (1 + GetFirstPercent(index, gearIndex, level + adjustment)/100) * qualityMultiplier * secondValue/100)",
            "getFirstPercent": first_percent_evidence["recoveredBehavior"],
            "goldenVectors": [
                {"input": [100, 100, 0, 2, 100], "result": gear_damage(100, 100, 0, 2, 100)},
                {"input": [120, 80, 25, 4, 150], "result": gear_damage(120, 80, 25, 4, 150)},
                {"firstPercentValues": [2, 3, 4, 5, 6, 7, 8, 9, 10], "limit": 12, "result": first_percent([2, 3, 4, 5, 6, 7, 8, 9, 10], 12)}
            ],
            "status": "exact structural expression recovered; gameplay meaning of the caller adjustment remains unresolved"
        },
        "sealAttack": {
            "acceptedInputIds": SEAL_ACCEPTED,
            "acceptedResult": "first integer from the selected nested table row multiplied by 0.01",
            "rejectedResult": 0.0,
            "goldenVectors": [
                {"inputId": 157, "tableFirstValue": 25, "result": 0.25},
                {"inputId": 161, "tableFirstValue": 125, "result": 1.25},
                {"inputId": 156, "tableFirstValue": 25, "result": 0.0},
            ],
        },
        "weaponCostumeAttack": {
            **WEAPON_COSTUME_CAPTURE,
            "acceptedSelectorEvidence": {
                "vectorRangeLowerBounds": [182, 109, 298, 311, 354, 441, 478, 516, 555, 651, 753, 811],
                "vectorRangeWidths": [12, 8, 4, 4, 4, 8, 4, 8, 4, 4, 4, 5],
                "scalarRangeLowerBounds": [823, 847, 874],
                "scalarRangeWidths": [5, 5, 4],
            },
            "acceptedResult": "first integer from the selected nested table row multiplied by 0.01",
            "rejectedResult": 0.0,
            "status": "selector arithmetic is normalized, but the second SIMD lane uses a bit-cleared input; its final accepted ID expansion remains unresolved",
        },
        "blockers": [
            "Option type meanings and plus/minus/rune participation are not proven by these three Gear methods.",
            "The caller adjustment added to GearData.level before GetFirstPercent remains semantically unresolved.",
            "Weapon-costume selector lanes must be exhaustively emulated before publishing a final accepted ID list.",
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture", type=Path, default=DEFAULT_CAPTURE)
    parser.add_argument("--analysis", type=Path, default=DEFAULT_ANALYSIS)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--admin-schema", type=Path, default=DEFAULT_ADMIN_SCHEMA)
    parser.add_argument("--first-percent", type=Path, default=DEFAULT_FIRST_PERCENT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    result = build(args.capture, args.analysis, args.schema, args.admin_schema, args.first_percent)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote {args.output}")


if __name__ == "__main__":
    main()
