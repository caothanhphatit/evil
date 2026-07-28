#!/usr/bin/env python3
"""Bind the exact known PlusExp operand order and level-up side effects."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_METHODS = ROOT / "reverse-engineering/evidence/original-reward-progression-methods-api35-v1.json"
DEFAULT_REWARD_SCHEMA = ROOT / "reverse-engineering/evidence/original-reward-progression-schema-api35-v1.json"
DEFAULT_HUNTER_SCHEMA = ROOT / "reverse-engineering/evidence/hunter-info-runtime-schema-android-api35-v1.json"
DEFAULT_PASS4 = ROOT / "reverse-engineering/evidence/original-reward-progression-level-domain-v4.json"
DEFAULT_DAMAGE_TAIL = ROOT / "reverse-engineering/evidence/original-native-hunter-damage-tail-v2.json"
DEFAULT_OUTPUT = ROOT / "reverse-engineering/evidence/original-reward-progression-plus-exp-chain-v5.json"


def source(path: Path, source_id: str) -> dict[str, Any]:
    body = path.read_bytes()
    return {
        "id": source_id,
        "path": path.resolve().relative_to(ROOT).as_posix(),
        "bytes": len(body),
        "sha256": hashlib.sha256(body).hexdigest(),
    }


def plus_exp_body(path: Path) -> bytes:
    document = json.loads(path.read_text())
    method = next(
        row
        for row in document["record"]["payload"]["methods"]
        if row["className"] == "HunterCtrl" and row["methodName"] == "PlusExp"
    )
    candidate = method["candidates"][0]
    body = bytes.fromhex(candidate["codeHex"])
    if candidate["codeTruncated"] or len(body) != 5480:
        raise ValueError("PlusExp exact body changed")
    return body


def assert_words(body: bytes, expected: dict[int, str]) -> None:
    for offset, word in expected.items():
        if body[offset : offset + 4].hex() != word:
            raise ValueError(f"PlusExp instruction changed at +{offset:#x}")


def classes(*paths: Path) -> dict[str, dict[str, Any]]:
    result = {}
    for path in paths:
        document = json.loads(path.read_text())
        result.update({row["name"]: row for row in document["record"]["payload"]["classes"]})
    return result


def validate_schema(reward_schema: Path, hunter_schema: Path) -> dict[str, dict[str, int]]:
    available = classes(reward_schema, hunter_schema)
    expected = {
        "UserData": {
            "timeData": 0x98,
            "<currentMissionIdx>k__BackingField": 0x778,
            "<mBuildingExpUp>k__BackingField": 0x960,
            "ExpGemPack_Active": 0x2248,
        },
        "TimeData": {
            "<expScroll>k__BackingField": 0x1A8,
            "<BoxExp>k__BackingField": 0x218,
        },
        "StatusData": {
            "<GearProperty>k__BackingField": 0x210,
            "CostumeExpUp": 0x428,
            "CollectionExpUp": 0x55C,
        },
        "HunterData": {
            "<job>k__BackingField": 0x20,
            "<subJob>k__BackingField": 0x30,
            "<thirdJob>k__BackingField": 0x40,
            "<fourthJob>k__BackingField": 0x50,
            "<level>k__BackingField": 0x88,
            "<exp>k__BackingField": 0x98,
            "<areaIndex>k__BackingField": 0x104,
            "<reviveWisdom>k__BackingField": 0x314,
            "<costumeIndex>k__BackingField": 0x4C0,
            "<DSoul>k__BackingField": 0x640,
        },
    }
    for class_name, fields in expected.items():
        actual = {field["name"]: field["offset"] for field in available[class_name]["fields"]}
        for name, offset in fields.items():
            if actual.get(name) != offset:
                raise ValueError(f"schema changed: {class_name}.{name}")
    return expected


def build(
    methods_path: Path,
    reward_schema: Path,
    hunter_schema: Path,
    pass4_path: Path,
    damage_tail_path: Path = DEFAULT_DAMAGE_TAIL,
) -> dict[str, Any]:
    body = plus_exp_body(methods_path)
    pass4 = json.loads(pass4_path.read_text())
    damage_tail = json.loads(damage_tail_path.read_text())
    if pass4["maxLevel"]["decodedValue"] != 99:
        raise ValueError("pass-4 stored cap changed")
    reused_literal = damage_tail["armorSelector"]["thresholdFloat32"][3]
    if struct.pack("<f", reused_literal).hex() != "cdcc4c3e":
        raise ValueError("module literal 0xD2AAB8 no longer resolves to float32 0.2")
    schema = validate_schema(reward_schema, hunter_schema)
    assert_words(
        body,
        {
            0x00D4: "0059c23d",  # UserData.mBuildingExpUp
            0x0130: "00102e1e",  # fallback 1.0
            0x016C: "00d540f9",  # TimeData.expScroll
            0x019C: "01102e1e",  # +1.0
            0x01EC: "000d41f9",  # TimeData.BoxExp
            0x021C: "01102c1e",  # +0.5
            0x0264: "002551f9",  # UserData.ExpGemPack_Active
            0x0294: "01102e1e",  # +1.0
            0x032C: "08500c91",  # HunterData.reviveWisdom
            0x033C: "1f040071",  # reviveWisdom >= 1
            0x0364: "1f140071",  # reviveWisdom <= 5
            0x03E8: "c8d2288b",  # indexed revive-wisdom table entry
            0x0430: "080841f9",  # StatusData.GearProperty
            0x043C: "3fa10071",  # outer array requires index 40
            0x0550: "c802004b",  # property[0] - property[1]
            0x05D4: "2505cb97",  # GameManager.IsCostumeExpUp
            0x0608: "08a01091",  # StatusData.CostumeExpUp
            0x0654: "005c45bd",  # StatusData.CollectionExpUp
            0x06BC: "08100491",  # HunterData.areaIndex
            0x07D8: "d773c097",  # decoded area-dependent integer operand
            0x0824: "0820201e",  # accumulator clamp comparison with zero
            0x0B38: "0008211e",  # accumulator * incoming, branch A
            0x0B44: "1500381e",  # truncate toward zero
            0x0BAC: "0008211e",  # accumulator * incoming, branch B
            0x0BB8: "1700381e",  # truncate toward zero
            0x0C38: "0008211e",  # accumulator * incoming, branch C
            0x0C44: "0008211e",  # extra unresolved native literal multiplier
            0x0C50: "1500381e",  # truncate toward zero
            0x0DCC: "b0bac097",  # increment HunterData.level ObscuredInt
            0x0DE8: "e176c097",  # construct zero and reset HunterData.exp
            0x0E28: "0000150b",  # final EXP plus positive/non-leveling remainder
            0x0EC8: "08040011",  # displayed level = stored level + 1
            0x0F88: "08040011",  # mission threshold uses stored level + 1
            0x0F8C: "1f910171",  # compare displayed level with 100
            0x111C: "e00313aa",  # secondary progression continues only if value >=1
            0x1314: "0090c13d",  # HunterData.DSoul obscured-long read
            0x13E0: "c092813d",  # HunterData.DSoul write
        },
    )
    return {
        "schemaVersion": 5,
        "contractType": "original-reward-progression-plus-exp-chain-evidence",
        "runtimeCompatibility": "evidence-only",
        "sources": [
            source(methods_path, "reward-progression-methods"),
            source(reward_schema, "reward-progression-schema"),
            source(hunter_schema, "hunter-runtime-schema-api35"),
            source(pass4_path, "reward-progression-level-domain-v4"),
            source(damage_tail_path, "hunter-damage-tail-v2-package-literals"),
        ],
        "schemaBindings": schema,
        "orderedKnownAccumulatorOperands": [
            {"order": 1, "operand": "UserData.mBuildingExpUp", "rule": "use when > 0; otherwise initialize accumulator to 1.0"},
            {"order": 2, "operand": "TimeData.expScroll", "rule": "when true, add 1.0"},
            {"order": 3, "operand": "TimeData.BoxExp", "rule": "when true, add 0.5"},
            {"order": 4, "operand": "UserData.ExpGemPack_Active", "rule": "when true, add 1.0"},
            {"order": 5, "operand": "unnamed singleton ObscuredBool at object offset 0x2C", "rule": "when true, add float32 0.2 loaded from module address 0xD2AAB8; product meaning unresolved"},
            {"order": 6, "operand": "HunterData.reviveWisdom", "rule": "only values 1..5 index an unnamed five-entry ObscuredInt table; add decoded entry multiplied by the confirmed hundredths scalar"},
            {"order": 7, "operand": "StatusData.GearProperty[40]", "rule": "when row 0 <= 0 and row 1 >= 1, add (row0 - row1) multiplied by the confirmed hundredths scalar"},
            {"order": 8, "operand": "StatusData.CostumeExpUp", "rule": "add only when GameManager.IsCostumeExpUp(HunterData.costumeIndex) returns true"},
            {"order": 9, "operand": "StatusData.CollectionExpUp", "rule": "add only when > 0"},
            {"order": 10, "operand": "area-dependent unnamed static pair", "rule": "after excluding two unnamed static area IDs, an unnamed lookup can add a decoded integer multiplied by the confirmed hundredths scalar"},
            {"order": 11, "operand": "accumulator clamp", "rule": "replace a negative accumulator with 0"},
        ],
        "incomingGrantApplication": {
            "confirmedSites": [
                {"offset": "0x0B2C", "formula": "truncateTowardZero(accumulator * incomingGrant)", "notificationCode": 4},
                {"offset": "0x0BA0", "formula": "truncateTowardZero(accumulator * incomingGrant)", "notificationCode": 4},
                {"offset": "0x0C2C", "formula": "truncateTowardZero(accumulator * incomingGrant * float32(0.2))", "notificationCode": 12},
            ],
            "completeBranchSemanticBinding": False,
            "reason": "Stage/revive/area comparisons and the reused float32 0.2 literal are exact, but three static area IDs and branch product meanings remain unresolved.",
        },
        "storedAndDisplayedLevelDomain": {
            "storedCap": 99,
            "displayExpression": "HunterData.level + 1",
            "maximumDisplayedValueOnThisPath": 100,
            "needExpCurrentStoredLevels": "0..98",
            "catalogRows": "1..99",
        },
        "levelMutationAndSideEffects": {
            "alwaysOnGrantPath": [
                "If positive overflow crosses a threshold, increment the stored ObscuredInt level and reset stored EXP to zero.",
                "Repeat while positive overflow remains and stored level is below the static cap.",
                "After the loop, add the non-leveling or carried remainder to HunterData.exp.",
            ],
            "onlyWhenDidLevelAndBooleanParameterIsTrue": [
                "Invoke presentation/status calls at exact native sites 0x0E40..0x0F00; unresolved callees are not renamed.",
                "Build level text from HunterData.level + 1.",
                "Check UserData.currentMissionIdx against an unnamed static mission ID and compare HunterData.level + 1 with 100 before invoking unresolved mission calls.",
                "Evaluate the separate stage/revive/stored-level branch producing 75/100/125 and mBuildingSoulUp.",
                "When that secondary value is positive, read job/subJob/thirdJob/fourthJob data, mutate HunterData.DSoul, and emit notification code 14; exact table semantics remain unresolved.",
            ],
        },
        "implementationBoundary": {
            "fullGoldenCallerVectorAvailable": False,
            "liveIntegrationAllowed": False,
            "unresolved": [
                "Product meaning of the float32 0.2 branch using module literal 0xD2AAB8.",
                "Identity of the singleton Boolean at offset 0x2C and its getter at 0x2ACF0C4.",
                "Names/content of the revive-wisdom table and area-dependent static IDs/table.",
                "Exact branch product meanings for notification codes 4, 12, and 14.",
                "Semantic names of presentation, mission, and DSoul table helper calls.",
            ],
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--methods", type=Path, default=DEFAULT_METHODS)
    parser.add_argument("--reward-schema", type=Path, default=DEFAULT_REWARD_SCHEMA)
    parser.add_argument("--hunter-schema", type=Path, default=DEFAULT_HUNTER_SCHEMA)
    parser.add_argument("--pass4", type=Path, default=DEFAULT_PASS4)
    parser.add_argument("--damage-tail", type=Path, default=DEFAULT_DAMAGE_TAIL)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    evidence = build(args.methods, args.reward_schema, args.hunter_schema, args.pass4, args.damage_tail)
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote pass-5 PlusExp evidence to {args.output}")


if __name__ == "__main__":
    main()
