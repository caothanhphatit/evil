#!/usr/bin/env python3
"""Normalize packaged monster and unique-gear tables without inventing drop rules."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_TABLES = ROOT / "reverse-engineering/evidence/hunter-info-tables-v1.json"
DEFAULT_SCHEMA = ROOT / "reverse-engineering/evidence/evil-ai-drop-runtime-schema-android-api35-v1.json"
DEFAULT_NATIVE = ROOT / "reverse-engineering/evidence/monster-reward-native-method-prefixes-android-api35-v1.json"
DEFAULT_RANDOM = ROOT / "reverse-engineering/evidence/unity-random-range-native-methods-android-api35-v1.json"
DEFAULT_OUTPUT = ROOT / "packages/content/releases/evil-hunter-1.411/monster-runtime-catalog.json"


def source(path: Path, source_id: str) -> dict[str, Any]:
    payload = path.read_bytes()
    return {
        "id": source_id,
        "path": path.resolve().relative_to(ROOT).as_posix(),
        "bytes": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }


def load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text())


def runtime_classes(document: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {row["name"]: row for row in document["record"]["payload"]["classes"]}


def field_types(row: dict[str, Any]) -> dict[str, str]:
    return {field["name"]: field["type"] for field in row["fields"]}


def validate_runtime_schema(document: dict[str, Any]) -> None:
    classes = runtime_classes(document)
    evil = field_types(classes["AdminEvilData"])
    unique = field_types(classes["AdminDropUniqueGearData"])
    expected_evil = {
        "<uniqueLevel>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt",
        "<type>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt",
        "<area>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt",
        "<createLevel>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt",
        "<damage>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredLong",
        "<armor>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredLong",
        "<hp>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredLong",
        "<metIdx>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt[]",
        "<metCount>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt[]",
        "<metPercent>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt[]",
        "<exp>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt",
        "<gold>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt",
    }
    expected_unique = {
        "<dropRange>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt",
        "<dropCut>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt[]",
        "<gearType>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt[]",
        "<gearIndex>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt[]",
        "<gearPercent>k__BackingField": "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt[]",
    }
    if any(evil.get(name) != value for name, value in expected_evil.items()):
        raise ValueError("AdminEvilData runtime schema no longer matches the normalized contract")
    if any(unique.get(name) != value for name, value in expected_unique.items()):
        raise ValueError("AdminDropUniqueGearData runtime schema no longer matches the normalized contract")


def validate_native_methods(document: dict[str, Any]) -> list[dict[str, Any]]:
    expected = {
        ("EvilCtrl", "Dead", 1): 100675559,
        ("HunterCtrl", "RewardMetrial", 5): 100686745,
        ("HunterCtrl", "Reward", 2): 100686803,
        ("HunterCtrl", "PlusGold", 1): 100686864,
    }
    rows = document["record"]["payload"]["methods"]
    found = {(row["className"], row["methodName"], row["parameterCount"]): row for row in rows}
    if document["record"]["payload"]["missing"]:
        raise ValueError("native reward capture is missing requested methods")
    for key, token in expected.items():
        if found.get(key, {}).get("token") != token:
            raise ValueError(f"native reward token mismatch: {key}")
    return [
        {
            "className": key[0],
            "methodName": key[1],
            "parameterCount": key[2],
            "token": token,
            "moduleOffset": found[key]["candidates"][0]["moduleOffset"],
            "capturedPrefixBytes": len(found[key]["candidates"][0]["codeHex"]) // 2,
        }
        for key, token in expected.items()
    ]


def validate_material_roll_evidence(
    native_document: dict[str, Any], random_document: dict[str, Any]
) -> dict[str, Any]:
    random_rows = random_document["record"]["payload"]["methods"]
    integer_range = next(
        (
            row
            for row in random_rows
            if row["className"] == "Random"
            and row["methodName"] == "Range"
            and row["parameterTypes"] == ["System.Int32", "System.Int32"]
        ),
        None,
    )
    if integer_range is None or integer_range["token"] != 100665478:
        raise ValueError("UnityEngine.Random.Range(Int32, Int32) token mismatch")
    random_candidate = integer_range["candidates"][0]
    if (
        random_candidate["address"] != "0x73ad876240"
        or random_candidate["moduleOffset"] != "0x5a76240"
    ):
        raise ValueError("UnityEngine.Random.Range(Int32, Int32) pointer mismatch")

    reward_row = next(
        row
        for row in native_document["record"]["payload"]["methods"]
        if row["className"] == "HunterCtrl" and row["methodName"] == "RewardMetrial"
    )
    reward_candidate = reward_row["candidates"][0]
    base = int(reward_candidate["address"], 16)
    code = bytes.fromhex(reward_candidate["codeHex"])
    expected_words = {
        0x73AB225FE4: "081980b9",  # materialIndices array length
        0x73AB225FF0: "20008052",  # Random.Range min = 1
        0x73AB225FF4: "21e28452",  # Random.Range maxExclusive = 10001
        0x73AB225FFC: "91409994",  # call the captured integer overload
        0x73AB22603C: "086d40f9",  # materialPercent array
        0x73AB226064: "0808000b",  # raw + raw * 4
        0x73AB226068: "00791f53",  # previous value * 2 => raw * 10
        0x73AB2268A8: "7f02006b",  # compare effective threshold with roll
        0x73AB2268AC: "eb0a0054",  # skip grant when threshold < roll
        0x73AB2268E0: "086940f9",  # materialCounts array
        0x73AB2269B4: "086540f9",  # materialIndices array
        0x73AB226A0C: "7b070091",  # increment slot index
    }
    for address, expected_hex in expected_words.items():
        offset = address - base
        if code[offset : offset + 4].hex() != expected_hex:
            raise ValueError(f"RewardMetrial instruction mismatch at {address:#x}")

    return {
        "api": "UnityEngine.Random.Range(System.Int32,System.Int32)",
        "token": integer_range["token"],
        "address": random_candidate["address"],
        "moduleOffset": random_candidate["moduleOffset"],
        "minInclusive": 1,
        "maxExclusive": 10001,
        "outcomes": "1..10000",
    }


def normalize_monster(row: dict[str, Any]) -> dict[str, Any]:
    return {
        "index": row["index"],
        "uniqueLevel": row["uniqueLevel"],
        "race": row["race"],
        "hp": row["hp"],
        "damage": row["damage"],
        "armor": row["armor"],
        "experience": row["experience"],
        "gold": row["gold"],
        "materials": {
            "indices": row["materialIndices"],
            "counts": row["materialCounts"],
            "percentValues": row["materialPercentValues"],
            "arrayLengths": {
                "indices": len(row["materialIndices"]),
                "counts": len(row["materialCounts"]),
                "percentValues": len(row["materialPercentValues"]),
            },
        },
    }


def normalize_unique_pool(row: dict[str, Any]) -> dict[str, Any]:
    lengths = {len(row[name]) for name in ("gearTypes", "gearIndices", "gearPercentValues")}
    if len(lengths) != 1:
        raise ValueError(f"unique gear pool {row['index']} has misaligned gear arrays")
    return {
        "index": row["index"],
        "dropRange": row["dropRange"],
        "dropCut": row["dropCut"],
        "gearPool": {
            "types": row["gearTypes"],
            "indices": row["gearIndices"],
            "percentValues": row["gearPercentValues"],
        },
    }


def generate(
    tables_path: Path, schema_path: Path, native_path: Path, random_path: Path
) -> dict[str, Any]:
    tables = load(tables_path)
    schema = load(schema_path)
    native = load(native_path)
    random = load(random_path)
    validate_runtime_schema(schema)
    native_methods = validate_native_methods(native)
    material_roll = validate_material_roll_evidence(native, random)
    monsters = tables["monsters"]
    unique_pools = tables["uniqueGearDrops"]
    if [row["index"] for row in monsters] != list(range(195)):
        raise ValueError("monster indices must remain the exact contiguous packaged order")
    if [row["index"] for row in unique_pools] != list(range(61)):
        raise ValueError("unique gear pool indices must remain the exact contiguous packaged order")

    grouped: dict[tuple[int, int, int], list[dict[str, Any]]] = {}
    for row in monsters:
        key = (row["area"], row["type"], row["createLevel"])
        grouped.setdefault(key, []).append(normalize_monster(row))

    return {
        "schemaVersion": 1,
        "catalogId": "evil-hunter-1.411.monster-runtime-v1",
        "runtimeCompatibility": "evidence-only",
        "sources": [
            source(tables_path, "hunter-info-tables-v1"),
            source(schema_path, "evil-ai-drop-runtime-schema-android-api35-v1"),
            source(native_path, "monster-reward-native-method-prefixes-android-api35-v1"),
            source(random_path, "unity-random-range-native-methods-android-api35-v1"),
        ],
        "monsterKey": {
            "dimensions": ["area", "type", "createLevel"],
            "cardinality": "one-to-many",
            "note": "The packaged dimensions are not unique; every key retains rows in original monster index order.",
        },
        "rewardSemantics": {
            "staticValues": "decoded-exact",
            "materialPercentDenominator": 1000,
            "materialRollDenominator": 10000,
            "materialRoll": material_roll,
            "materialThreshold": {
                "baseFormula": "materialPercentValues[slot] * 10",
                "grantComparison": "effectiveThreshold >= roll",
                "modifierFormula": None,
                "note": "RewardMetrial applies additional hunter/global modifiers after the exact base scaling; their full formula is not yet ported.",
            },
            "materialSelectionOrder": {
                "order": "ascending-array-slot",
                "startsAt": 0,
                "loopBound": "materialIndices.length",
                "arraysReadAtSameSlot": [
                    "materialIndices",
                    "materialCounts",
                    "materialPercentValues",
                ],
            },
            "uniqueGearSelectionOrder": None,
            "uniqueLevelToPoolBinding": None,
            "requiredEvidence": "Recover unique-gear pool linkage/order and the complete material modifier formula, then verify them with controlled runtime outcomes.",
            "packagedArrayAnomalies": [
                {
                    "monsterIndex": row["index"],
                    "materialArrayLengths": {
                        "indices": len(row["materialIndices"]),
                        "counts": len(row["materialCounts"]),
                        "percentValues": len(row["materialPercentValues"]),
                    },
                }
                for row in monsters
                if len({len(row[name]) for name in ("materialIndices", "materialCounts", "materialPercentValues")}) > 1
            ],
            "nativeMethods": native_methods,
        },
        "groups": [
            {
                "key": {"area": key[0], "type": key[1], "createLevel": key[2]},
                "monsters": grouped[key],
            }
            for key in sorted(grouped)
        ],
        "uniqueGearPools": [normalize_unique_pool(row) for row in unique_pools],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tables", type=Path, default=DEFAULT_TABLES)
    parser.add_argument("--runtime-schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--native-evidence", type=Path, default=DEFAULT_NATIVE)
    parser.add_argument("--random-evidence", type=Path, default=DEFAULT_RANDOM)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    catalog = generate(
        args.tables, args.runtime_schema, args.native_evidence, args.random_evidence
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(catalog, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote {sum(len(group['monsters']) for group in catalog['groups'])} monster rows to {args.output}")


if __name__ == "__main__":
    main()
