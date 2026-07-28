#!/usr/bin/env python3
"""Normalize exact native reward/progression captures into auditable evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_METHODS = ROOT / "reverse-engineering/evidence/original-reward-progression-methods-api35-v1.json"
DEFAULT_HELPERS = ROOT / "reverse-engineering/evidence/original-reward-progression-helpers-api35-v1.json"
DEFAULT_MATERIAL = ROOT / "reverse-engineering/evidence/original-reward-material-full-api35-v1.json"
DEFAULT_SCHEMA = ROOT / "reverse-engineering/evidence/original-reward-progression-schema-api35-v1.json"
DEFAULT_CATALOG = ROOT / "packages/content/releases/evil-hunter-1.411/experience-runtime-catalog.json"
DEFAULT_OUTPUT = ROOT / "reverse-engineering/evidence/original-reward-progression-runtime-v1.json"


EXPECTED_METHODS = {
    ("HunterCtrl", "PlusExp"): (100687028, 5480, "caa296d09f86352c7dd1ccb051f36902f78d616b192a7004d31b4a041239db06"),
    ("HunterCtrl", "PlusGold"): (100686864, 444, "fdf0854838a35be1de62f89fcc20628cdd1dcdd728600fad511383eb6694c0ee"),
    ("HunterCtrl", "Reward"): (100686803, 5904, "0b6ea9ee35163b4f4bab3bcaa5108e6495373c242de86d25e9551963f7a0bf18"),
    ("HunterCtrl", "GHPHHEFFNKN"): (100686954, 4236, "250ef7005be0c2c343ac5571aa80fc79ecd4b38393aff53eead9c0336b956139"),
    ("HunterCtrl", "LDHAEMDJCFF"): (100687005, 2120, "9621d392d78324d0c2b3a751c37403873f2544e6011c22401fcb8fb3e13804c7"),
    ("GameManager", "GetNeedExp"): (100690845, 404, "e1c931a52031427215a6050e7f6c10c6e97f8e2d93100c6927c0fb0dd250ca9c"),
}


def source(path: Path, source_id: str) -> dict[str, Any]:
    body = path.read_bytes()
    return {"id": source_id, "path": path.resolve().relative_to(ROOT).as_posix(), "bytes": len(body), "sha256": hashlib.sha256(body).hexdigest()}


def methods(document: dict[str, Any]) -> dict[tuple[str, str], tuple[dict[str, Any], bytes]]:
    result = {}
    for method in document["record"]["payload"]["methods"]:
        candidate = method["candidates"][0]
        body = bytes.fromhex(candidate["codeHex"])
        result[(method["className"], method["methodName"])] = (method, body)
    return result


def assert_words(body: bytes, expected: dict[int, str], label: str) -> None:
    for offset, word in expected.items():
        if body[offset : offset + 4].hex() != word:
            raise ValueError(f"{label} instruction changed at +{offset:#x}")


def simulate_plus_exp(level: int, exp: int, incoming: int, max_level: int, need_exp) -> tuple[int, int]:
    if level >= max_level:
        return level, exp
    remaining_grant = incoming
    while level < max_level:
        remaining = remaining_grant - (need_exp(level) - exp)
        if remaining <= 0:
            return level, exp + remaining_grant
        level += 1
        exp = 0
        remaining_grant = remaining
    return level, exp


def build(methods_path: Path, helpers_path: Path, material_path: Path, schema_path: Path, catalog_path: Path) -> dict[str, Any]:
    method_doc = json.loads(methods_path.read_text())
    helper_doc = json.loads(helpers_path.read_text())
    material_doc = json.loads(material_path.read_text())
    schema_doc = json.loads(schema_path.read_text())
    catalog = json.loads(catalog_path.read_text())
    captured = {**methods(method_doc), **methods(helper_doc)}
    normalized_methods = []
    for key, (token, size, body_hash) in EXPECTED_METHODS.items():
        method, body = captured[key]
        candidate = method["candidates"][0]
        if method["token"] != token or candidate["nativeSizeBytes"] != size or len(body) != size:
            raise ValueError(f"exact boundary changed for {key}")
        if hashlib.sha256(body).hexdigest() != body_hash:
            raise ValueError(f"native body changed for {key}")
        normalized_methods.append({"type": key[0], "method": key[1], "token": token, "nativeSizeBytes": size, "bodySha256": body_hash})

    plus_exp = captured[("HunterCtrl", "PlusExp")][1]
    assert_words(plus_exp, {
        0xD28: "f1cefe97",  # load HunterData before revive read
        0xD30: "08404cf8",  # HunterData.revive obscured pair at 0xc4
        0xD54: "088448a9",  # HunterData.level obscured pair at 0x88
        0xD78: "b08dcb97",  # GetNeedExp(revive, currentLevel)
        0xDA4: "c802004b",  # needExp - currentExp
        0xDAC: "b602086b",  # incoming - (needExp - currentExp)
        0xDB0: "ed020054",  # b.le: exact threshold does not level
        0xDFC: "3a008052",  # level-up marker = 1
        0xE00: "f503162a",  # carry remaining into next loop
        0xE04: "c0f5ffb5",  # repeat while HunterData remains available
        0xE28: "0000150b",  # final currentExp + remaining grant
    }, "HunterCtrl.PlusExp")

    full_material_method, full_material = next(iter(methods(material_doc).values()))
    full_candidate = full_material_method["candidates"][0]
    if full_material_method["token"] != 100686745 or len(full_material) != 30732 or full_candidate["codeTruncated"]:
        raise ValueError("RewardMetrial full exact boundary changed")
    assert_words(full_material, {
        0x494: "081980b9", 0x4A0: "20008052", 0x4A4: "21e28452", 0x4EC: "086d40f9",
        0x514: "0808000b", 0x518: "00791f53", 0xD58: "7f02006b", 0xD5C: "eb0a0054",
        0xD90: "086940f9", 0xE64: "086540f9", 0xEBC: "7b070091",
    }, "HunterCtrl.RewardMetrial")

    classes = {row["name"]: row for row in schema_doc["record"]["payload"]["classes"]}
    schema_fields = {name: {field["name"]: field["offset"] for field in classes[name]["fields"]} for name in ("StatusData", "TimeData")}
    required_fields = {
        "StatusData": {"<CalcHighValueMet>k__BackingField": 0xE4, "CostumeExpUp": 0x428, "CollectionExpUp": 0x55C},
        "TimeData": {"<expScroll>k__BackingField": 0x1A8, "<BoxExp>k__BackingField": 0x218},
    }
    for class_name, fields in required_fields.items():
        for name, offset in fields.items():
            if schema_fields[class_name].get(name) != offset:
                raise ValueError(f"schema field changed: {class_name}.{name}")

    vectors = [
        {"name": "below-threshold", "input": [4, 20, 79, 10, 100], "expected": [4, 99]},
        {"name": "exact-threshold", "input": [4, 20, 80, 10, 100], "expected": [4, 100]},
        {"name": "cross-one-level", "input": [4, 20, 81, 10, 100], "expected": [5, 1]},
        {"name": "carry-multiple-levels", "input": [4, 20, 281, 10, 100], "expected": [7, 1]},
        {"name": "max-level-discard", "input": [10, 77, 999, 10, 100], "expected": [10, 77]},
    ]
    for vector in vectors:
        level, exp, incoming, max_level, need = vector["input"]
        actual = simulate_plus_exp(level, exp, incoming, max_level, lambda _: need)
        if list(actual) != vector["expected"]:
            raise ValueError(f"level golden vector failed: {vector['name']}")

    return {
        "schemaVersion": 1,
        "contractType": "original-reward-progression-runtime-evidence",
        "runtimeCompatibility": "evidence-only",
        "sources": [source(methods_path, "reward-progression-methods"), source(helpers_path, "reward-progression-helpers"), source(material_path, "reward-material-full"), source(schema_path, "reward-progression-schema"), source(catalog_path, "experience-runtime-catalog")],
        "methods": normalized_methods,
        "recoveredExactFacts": {
            "experienceLookup": catalog["lookup"],
            "levelCarry": {
                "pseudoCode": "if level >= maxLevel: discard grant; else repeatedly compute remaining = incoming - (GetNeedExp(revive, level) - exp); level only when remaining > 0; on level reset exp=0 and carry remaining; otherwise add incoming to exp",
                "thresholdComparison": "remaining > 0",
                "exactThresholdBehavior": "EXP becomes exactly needExp and level is unchanged",
                "supportsMultipleLevels": True,
                "maxLevelBehavior": "incoming EXP is discarded",
                "finalConversion": "incoming EXP modifier result uses ARM64 fcvtzs (truncate toward zero)",
                "goldenVectors": vectors,
            },
            "ordinaryMaterialRoll": {
                "loopOrder": "ascending array slot",
                "loopBound": "materialIndices.length",
                "roll": "UnityEngine.Random.Range(1, 10001)",
                "baseThreshold": "materialPercentValues[slot] * 10",
                "grantComparison": "effectiveThreshold >= roll",
                "grantValues": ["materialIndices[slot]", "materialCounts[slot]"],
                "fullBodySha256": hashlib.sha256(full_material).hexdigest(),
            },
        },
        "partiallyResolvedModifierInputs": {
            "experience": ["UserData.mBuildingExpUp (base when positive, otherwise 1.0)", "TimeData.expScroll (+1.0)", "TimeData.BoxExp (+0.5)", "UserData.ExpGemPack_Active (+1.0)", "StatusData.CostumeExpUp", "StatusData.CollectionExpUp", "HunterData.reviveWisdom", "StatusData.GearProperty delta * 0.01"],
            "material": ["StatusData.CalcHighValueMet participates in a branch that truncates (CalcHighValueMet + 1.0) * priorThreshold when the material property condition is >= 3"],
        },
        "unresolved": [
            "Complete ordered EXP modifier chain, unnamed singleton Boolean/static constants, and all event conditions.",
            "Complete gold modifier and village-tax chain despite exact helper captures.",
            "Unique-level to unique-gear pool linkage, RNG order, dropRange/dropCut semantics, and gear selection order.",
            "Semantic names for HunterCtrl.GHPHHEFFNKN and HunterCtrl.LDHAEMDJCFF; their exact bodies are captured but not renamed by guess.",
            "Complete material threshold modifier order before the confirmed CalcHighValueMet branch.",
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--methods", type=Path, default=DEFAULT_METHODS)
    parser.add_argument("--helpers", type=Path, default=DEFAULT_HELPERS)
    parser.add_argument("--material", type=Path, default=DEFAULT_MATERIAL)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--catalog", type=Path, default=DEFAULT_CATALOG)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    evidence = build(args.methods, args.helpers, args.material, args.schema, args.catalog)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote normalized reward/progression evidence to {args.output}")


if __name__ == "__main__":
    main()
