#!/usr/bin/env python3
"""Bind the original PlusExp cap and stage/revive secondary progression branch."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_METHODS = ROOT / "reverse-engineering/evidence/original-reward-progression-methods-api35-v1.json"
DEFAULT_STATIC = ROOT / "reverse-engineering/evidence/original-plus-exp-max-level-static-api35-v1.json"
DEFAULT_SCHEMA = ROOT / "reverse-engineering/evidence/hunter-info-runtime-schema-android-api35-v1.json"
DEFAULT_OUTPUT = ROOT / "reverse-engineering/evidence/original-reward-progression-level-domain-v4.json"


def source(path: Path, source_id: str) -> dict[str, Any]:
    body = path.read_bytes()
    return {"id": source_id, "path": path.resolve().relative_to(ROOT).as_posix(), "bytes": len(body), "sha256": hashlib.sha256(body).hexdigest()}


def plus_exp_body(path: Path) -> bytes:
    document = json.loads(path.read_text())
    method = next(row for row in document["record"]["payload"]["methods"] if row["className"] == "HunterCtrl" and row["methodName"] == "PlusExp")
    candidate = method["candidates"][0]
    body = bytes.fromhex(candidate["codeHex"])
    if candidate["codeTruncated"] or len(body) != 5480:
        raise ValueError("PlusExp exact body changed")
    return body


def assert_words(body: bytes, expected: dict[int, str]) -> None:
    for offset, word in expected.items():
        if body[offset : offset + 4].hex() != word:
            raise ValueError(f"PlusExp instruction changed at +{offset:#x}")


def schema_fields(path: Path) -> dict[str, dict[str, int]]:
    document = json.loads(path.read_text())
    classes = {row["name"]: row for row in document["record"]["payload"]["classes"]}
    wanted = {
        "UserData": {"<mStageLevel>k__BackingField": 0x5D8, "<mBuildingSoulUp>k__BackingField": 0x9B0},
        "HunterData": {"<level>k__BackingField": 0x88, "<revive>k__BackingField": 0xC4},
    }
    for class_name, expected in wanted.items():
        actual = {field["name"]: field["offset"] for field in classes[class_name]["fields"]}
        for name, offset in expected.items():
            if actual.get(name) != offset:
                raise ValueError(f"schema changed: {class_name}.{name}")
    return wanted


def secondary_progression_value(stage_level: int, revive: int, hunter_level: int) -> int:
    if revive == 5 and hunter_level == 99 and stage_level >= 6:
        return 100 if stage_level == 6 else 125
    return 75


def build(methods_path: Path, static_path: Path, schema_path: Path) -> dict[str, Any]:
    body = plus_exp_body(methods_path)
    static = json.loads(static_path.read_text())
    obscured = static["obscuredInt"]
    if (obscured["hiddenValue"] ^ obscured["currentCryptoKey"]) != 99 or obscured["decodedValue"] != 99:
        raise ValueError("captured PlusExp cap no longer decodes to 99")
    assert_words(body, {
        0x0CCC: "280340f9",  # load the initialized class holder
        0x0CE8: "085d40f9",  # class static-fields pointer
        0x0CF0: "00854ea9",  # ObscuredInt pair at static offset 0xe8
        0x0CF4: "9072c097",  # decode ObscuredInt
        0x0D00: "df02006b",  # compare Hunter level with decoded cap
        0x0D04: "aa090054",  # skip EXP mutation when level >= cap
        0x0870: "00ed42f9",  # UserData.mStageLevel obscured pair
        0x0880: "1f140071",  # stageLevel compared with 5
        0x0894: "08404cf8",  # HunterData.revive at 0xc4
        0x08A8: "1f140071",  # revive compared with 5
        0x08BC: "088448a9",  # HunterData.level at 0x88
        0x08CC: "1f8c0171",  # Hunter level compared with 99
        0x0910: "60098052",  # secondary value 75
        0x101C: "a80f8052",  # secondary value 125
        0x1024: "890c8052",  # secondary value 100
        0x1028: "2001881a",  # choose 100 only when stageLevel == 6, else 125
        0x1060: "006dc23d",  # UserData.mBuildingSoulUp
    })
    schema = schema_fields(schema_path)
    vectors = [
        {"stageLevel": 4, "revive": 5, "hunterLevel": 99, "expected": 75},
        {"stageLevel": 5, "revive": 5, "hunterLevel": 99, "expected": 75},
        {"stageLevel": 6, "revive": 4, "hunterLevel": 99, "expected": 75},
        {"stageLevel": 6, "revive": 5, "hunterLevel": 98, "expected": 75},
        {"stageLevel": 6, "revive": 5, "hunterLevel": 99, "expected": 100},
        {"stageLevel": 7, "revive": 5, "hunterLevel": 99, "expected": 125},
    ]
    for vector in vectors:
        actual = secondary_progression_value(vector["stageLevel"], vector["revive"], vector["hunterLevel"])
        if actual != vector["expected"]:
            raise ValueError(f"secondary progression vector failed: {vector}")
    return {
        "schemaVersion": 4,
        "contractType": "original-reward-progression-level-domain-evidence",
        "runtimeCompatibility": "evidence-only",
        "sources": [source(methods_path, "reward-progression-methods"), source(static_path, "plus-exp-max-level-static"), source(schema_path, "hunter-info-runtime-schema-api35")],
        "maxLevel": {
            "decodedValue": 99,
            "storedLevelSemantics": "HunterData.level is displayed as stored level + 1 by the PlusExp level-up presentation path",
            "comparison": "current stored HunterData.level >= 99 discards incoming EXP",
            "maximumDisplayedValueObservedOnThisPath": 100,
            "validGetNeedExpCurrentLevels": "0..98 before the cap branch",
            "packagedRowLookup": "currentLevel + 1, therefore rows 1..99",
            "rowZeroUsage": "not used by this PlusExp/GetNeedExp path",
        },
        "secondaryProgressionBranch": {
            "formula": "revive == 5 && hunterLevel == 99 && stageLevel >= 6 ? (stageLevel == 6 ? 100 : 125) : 75",
            "schemaBindings": schema,
            "vectors": vectors,
            "interpretation": "This 75/100/125 result is separate from the max-level comparison and flows into code that reads mBuildingSoulUp; it is not a Hunter level cap.",
        },
        "remainingOperandBindings": {
            "resolved": ["UserData.mStageLevel", "UserData.mBuildingSoulUp", "HunterData.revive", "HunterData.level", "static max-level ObscuredInt = 99"],
            "unresolved": ["Product-facing semantic name of the static max-level holder", "Remaining unnamed singleton/static EXP additions", "Remaining fairy/pet/event gold operands and exact semantic order"],
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--methods", type=Path, default=DEFAULT_METHODS)
    parser.add_argument("--static", type=Path, default=DEFAULT_STATIC)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    evidence = build(args.methods, args.static, args.schema)
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote pass-4 level-domain evidence to {args.output}")


if __name__ == "__main__":
    main()
