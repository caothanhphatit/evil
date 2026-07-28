#!/usr/bin/env python3
"""Bind mechanically proven Reward, village-tax, and Hunter money mutations."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_METHODS = ROOT / "reverse-engineering/evidence/original-reward-progression-methods-api35-v1.json"
DEFAULT_REWARD_SCHEMA = ROOT / "reverse-engineering/evidence/original-reward-progression-schema-api35-v1.json"
DEFAULT_HUNTER_SCHEMA = ROOT / "reverse-engineering/evidence/hunter-info-runtime-schema-android-api35-v1.json"
DEFAULT_LITERAL = ROOT / "reverse-engineering/evidence/original-plus-gold-scaling-literal-package-v1.json"
DEFAULT_OUTPUT = ROOT / "reverse-engineering/evidence/original-reward-progression-gold-tax-chain-v6.json"


def source(path: Path, source_id: str) -> dict[str, Any]:
    body = path.read_bytes()
    return {"id": source_id, "path": path.resolve().relative_to(ROOT).as_posix(), "bytes": len(body), "sha256": hashlib.sha256(body).hexdigest()}


def bodies(path: Path) -> dict[str, bytes]:
    document = json.loads(path.read_text())
    result = {}
    for row in document["record"]["payload"]["methods"]:
        if row["className"] != "HunterCtrl" or row["methodName"] not in {"Reward", "CalVillTax", "PlusGold"}:
            continue
        candidate = row["candidates"][0]
        if candidate["codeTruncated"]:
            raise ValueError(f"truncated method: {row['methodName']}")
        result[row["methodName"]] = bytes.fromhex(candidate["codeHex"])
    if {name: len(body) for name, body in result.items()} != {"Reward": 5904, "CalVillTax": 1544, "PlusGold": 444}:
        raise ValueError("gold/tax method boundary changed")
    return result


def assert_words(body: bytes, expected: dict[int, str], label: str) -> None:
    for offset, word in expected.items():
        if body[offset : offset + 4].hex() != word:
            raise ValueError(f"{label} instruction changed at +{offset:#x}")


def validate_schema(reward_schema: Path, hunter_schema: Path) -> dict[str, dict[str, int]]:
    classes = {}
    for path in (reward_schema, hunter_schema):
        document = json.loads(path.read_text())
        classes.update({row["name"]: row for row in document["record"]["payload"]["classes"]})
    expected = {
        "UserData": {"<tax>k__BackingField": 0x100, "<taxRemainder>k__BackingField": 0x120, "<mBuildingGoldUp>k__BackingField": 0x974, "<mStageLevel>k__BackingField": 0x5D8},
        "HunterData": {"<money>k__BackingField": 0x60, "<revive>k__BackingField": 0xC4},
        "StatusData": {"FairyGoldUp": 0x4F0, "RamblePetGoldUp": 0x52C, "RelicCollectionGoldUp": 0x5DC},
    }
    for class_name, fields in expected.items():
        actual = {field["name"]: field["offset"] for field in classes[class_name]["fields"]}
        for name, offset in fields.items():
            if actual.get(name) != offset:
                raise ValueError(f"schema changed: {class_name}.{name}")
    return expected


def tax_segment(reward: int, tax: int, remainder: float, candidate: float, cap: int) -> tuple[int, int, float]:
    if reward < 1 or candidate <= 0 or tax >= cap:
        return reward, tax, remainder
    whole = int(candidate)
    reward -= whole
    tax += whole
    remainder += candidate - whole
    if remainder >= 1:
        carry = int(remainder)
        remainder -= carry
        tax += carry
    return reward, min(tax, cap), remainder


def plus_gold_segment(grant: int, money: int, revive: int, stage_level: int) -> tuple[int, int]:
    if revive > stage_level and stage_level <= 3:
        grant = int(grant * 0.3)
    return grant, money + grant if grant >= 1 else money


def build(methods_path: Path, reward_schema: Path, hunter_schema: Path, literal_path: Path) -> dict[str, Any]:
    method = bodies(methods_path)
    schema = validate_schema(reward_schema, hunter_schema)
    literal = json.loads(literal_path.read_text())["literal"]
    if literal != {"fileOffset": "0xD2B404", "littleEndianBytes": "9a99993e", "type": "float32", "value": 0.3, "consumer": "HunterCtrl.PlusGold"}:
        raise ValueError("PlusGold package literal changed")
    assert_words(method["Reward"], {
        0x0168: "09d12591", 0x01F8: "2909281e",  # building gold multiplier and incoming factor
        0x0B98: "003cc13d", 0x0BCC: "0008281e", 0x0BD0: "0028291e",  # FairyGoldUp hundredths then +1
        0x0C20: "002c45bd", 0x0C30: "0008281e", 0x0C34: "0028291e",  # RamblePetGoldUp
        0x13C0: "00dc45bd", 0x1420: "2008201e",  # RelicCollectionGoldUp * reward
        0x1478: "00060094", 0x16DC: "e9060094",  # CalVillTax then PlusGold
    }, "Reward")
    assert_words(method["CalVillTax"], {
        0x0118: "8102229e", 0x011C: "0029201e", 0x0128: "0008211e",  # float32 candidate
        0x0180: "000548ad", 0x02F8: "000708ad",  # UserData.tax read/write
        0x030C: "804ac03d", 0x0368: "804a803d",  # taxRemainder read/write
        0x0274: "0039201e", 0x0398: "800200cb",  # subtract whole tax from remainder/input
        0x05E0: "010140ad",  # final cap clamp write to UserData.tax
    }, "CalVillTax")
    assert_words(method["PlusGold"], {
        0x0050: "08404cf8", 0x008C: "00ed42f9",  # revive and stage level
        0x009C: "bf02006b", 0x00D8: "1f0c0071",  # revive > stage and stage <=3 gate
        0x0100: "010544bd", 0x0108: "0008211e", 0x0114: "0800381e",  # unresolved scale and truncation
        0x015C: "000443ad", 0x0190: "0000158b", 0x019C: "e00740ad",  # HunterData.money read/add/write
    }, "PlusGold")
    vectors = [
        {"name": "tax-fraction-carry", "input": [20, 10, 0.4, 2.75, 100], "expected": [18, 13, 0.15]},
        {"name": "tax-at-cap", "input": [20, 100, 0.4, 2.75, 100], "expected": [20, 100, 0.4]},
        {"name": "non-positive-candidate", "input": [20, 10, 0.4, 0.0, 100], "expected": [20, 10, 0.4]},
    ]
    for vector in vectors:
        actual = tax_segment(*vector["input"])
        if any(abs(a - b) > 1e-6 for a, b in zip(actual, vector["expected"])):
            raise ValueError(f"tax vector failed: {vector}")
    plus_gold_vectors = [
        {"name": "identity", "input": [10, 5, 2, 3], "expected": [10, 15]},
        {"name": "early-stage-scaling", "input": [10, 5, 4, 3], "expected": [3, 8]},
        {"name": "scaled-below-one", "input": [2, 5, 4, 3], "expected": [0, 5]},
    ]
    for vector in plus_gold_vectors:
        if list(plus_gold_segment(*vector["input"])) != vector["expected"]:
            raise ValueError(f"PlusGold vector failed: {vector}")
    return {
        "schemaVersion": 6,
        "contractType": "original-reward-progression-gold-tax-chain-evidence",
        "runtimeCompatibility": "evidence-only",
        "sources": [source(methods_path, "reward-progression-methods"), source(reward_schema, "reward-progression-schema"), source(hunter_schema, "hunter-runtime-schema-api35"), source(literal_path, "plus-gold-scaling-literal")],
        "schemaBindings": schema,
        "mutationOrder": ["Reward constructs an ObscuredLong gold grant", "CalVillTax mutates the grant by reference and updates UserData tax state", "PlusGold conditionally scales the post-tax grant and adds it to HunterData.money"],
        "confirmedRewardGoldOperands": [
            {"operand": "UserData.mBuildingGoldUp", "rule": "positive value participates before later gold modifiers; its exact surrounding rounding branches are preserved but not reduced to one semantic formula"},
            {"operand": "StatusData.FairyGoldUp", "rule": "grant = truncateTowardZero((1 + FairyGoldUp * 0.01) * priorGrant)"},
            {"operand": "StatusData.RamblePetGoldUp", "rule": "grant = truncateTowardZero((1 + RamblePetGoldUp * 0.01) * priorGrant)"},
            {"operand": "StatusData.RelicCollectionGoldUp", "rule": "when positive, add truncateTowardZero(RelicCollectionGoldUp * priorGrant)"},
        ],
        "taxSegment": {
            "candidate": "float32((unnamed ObscuredFloat at singleton offset 0x554 + unnamed ObscuredFloat at singleton offset 0xE8) * inputGrant)",
            "mutations": ["whole = truncateTowardZero(candidate)", "inputGrant -= whole", "UserData.tax += whole", "UserData.taxRemainder += candidate - whole", "when remainder >=1, move truncateTowardZero(remainder) into UserData.tax", "clamp UserData.tax to an unnamed static cap"],
            "goldenVectors": vectors,
        },
        "plusGold": {
            "identityBranch": "post-tax grant is unchanged unless revive > stageLevel and stageLevel <= 3",
            "scaledBranch": "truncateTowardZero(postTaxGrant * float32(0.3))",
            "sink": "if resulting grant >=1, HunterData.money += grant",
            "goldenVectors": plus_gold_vectors,
        },
        "implementationBoundary": {
            "fullGoldenCallerVectorAvailable": False,
            "liveIntegrationAllowed": False,
            "unresolved": ["Tax-rate identities at singleton offsets 0x554 and 0xE8", "Static village-tax cap value/name", "Product meaning of the revive > stageLevel and stageLevel <= 3 scaling branch", "Event/static branches and table operands between RamblePetGoldUp and RelicCollectionGoldUp", "Whether a separate CollectionGoldUp branch exists elsewhere in Reward; no direct 0x588 access is claimed here"],
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--methods", type=Path, default=DEFAULT_METHODS)
    parser.add_argument("--reward-schema", type=Path, default=DEFAULT_REWARD_SCHEMA)
    parser.add_argument("--hunter-schema", type=Path, default=DEFAULT_HUNTER_SCHEMA)
    parser.add_argument("--literal", type=Path, default=DEFAULT_LITERAL)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    evidence = build(args.methods, args.reward_schema, args.hunter_schema, args.literal)
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote pass-6 gold/tax evidence to {args.output}")


if __name__ == "__main__":
    main()
