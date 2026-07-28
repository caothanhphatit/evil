#!/usr/bin/env python3
"""Recover reward call order and RNG range families from exact ARM64 bodies."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter
from pathlib import Path
from typing import Any

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_METHODS = ROOT / "reverse-engineering/evidence/original-reward-progression-methods-api35-v1.json"
DEFAULT_HELPERS = ROOT / "reverse-engineering/evidence/original-reward-progression-helpers-api35-v1.json"
DEFAULT_MATERIAL = ROOT / "reverse-engineering/evidence/original-reward-material-full-api35-v1.json"
DEFAULT_SCHEMA = ROOT / "reverse-engineering/evidence/original-reward-progression-schema-api35-v1.json"
DEFAULT_OUTPUT = ROOT / "reverse-engineering/evidence/original-reward-progression-callgraph-v2.json"

RANDOM_RANGE_OFFSET = 0x5A76240
KNOWN_TARGETS = {
    0x342D35C: "HunterCtrl.LDHAEMDJCFF/5",
    0x342DBA4: "HunterCtrl.GHPHHEFFNKN/2",
    0x3438FF8: "HunterCtrl.PlusExp/2",
    0x343A560: "HunterCtrl.CalVillTax/1",
    0x343AB68: "HunterCtrl.PlusGold/1",
    0x26FAA60: "GameManager.IsCostumeExpUp/1",
    0x271D430: "GameManager.GetNeedExp/2",
    RANDOM_RANGE_OFFSET: "UnityEngine.Random.Range(Int32,Int32)",
}


def source(path: Path, source_id: str) -> dict[str, Any]:
    body = path.read_bytes()
    return {"id": source_id, "path": path.resolve().relative_to(ROOT).as_posix(), "bytes": len(body), "sha256": hashlib.sha256(body).hexdigest()}


def sign_extend(value: int, bits: int) -> int:
    sign = 1 << (bits - 1)
    return (value ^ sign) - sign


def direct_calls(body: bytes, start: int) -> list[tuple[int, int]]:
    result = []
    for offset in range(0, len(body), 4):
        word = int.from_bytes(body[offset : offset + 4], "little")
        if word & 0xFC000000 != 0x94000000:
            continue
        target = start + offset + (sign_extend(word & 0x03FFFFFF, 26) << 2)
        result.append((offset, target))
    return result


def documents(*paths: Path) -> list[dict[str, Any]]:
    return [json.loads(path.read_text()) for path in paths]


def exact_method(docs: list[dict[str, Any]], class_name: str, method_name: str) -> tuple[dict[str, Any], bytes, int]:
    matches = []
    for document in docs:
        for method in document["record"]["payload"]["methods"]:
            if method["className"] == class_name and method["methodName"] == method_name:
                candidate = method["candidates"][0]
                matches.append((len(candidate["codeHex"]), method, candidate))
    if not matches:
        raise ValueError(f"missing {class_name}.{method_name}")
    _, method, candidate = max(matches, key=lambda row: row[0])
    body = bytes.fromhex(candidate["codeHex"])
    if candidate.get("codeTruncated") or len(body) != candidate.get("nativeSizeBytes"):
        raise ValueError(f"{class_name}.{method_name} is not a full exact body")
    return method, body, int(candidate["moduleOffset"], 16)


IMMEDIATE_RE = re.compile(r"#(-?0x[0-9a-f]+|-?[0-9]+)")


def immediate(text: str) -> int | None:
    match = IMMEDIATE_RE.search(text)
    return None if match is None else int(match.group(1), 0)


def register_constant(instructions, call_index: int, register: str) -> int | None:
    movk_parts: list[tuple[int, int]] = []
    for instruction in reversed(instructions[max(0, call_index - 8) : call_index]):
        if instruction.mnemonic == "bl":
            break
        operands = [part.strip() for part in instruction.op_str.split(",")]
        if not operands or operands[0] != register:
            continue
        if instruction.mnemonic == "movk":
            value = immediate(operands[1])
            shift = immediate(operands[2]) if len(operands) > 2 else 0
            if value is None:
                return None
            movk_parts.append((shift or 0, value & 0xFFFF))
            continue
        if instruction.mnemonic != "mov":
            return None
        if len(operands) < 2:
            return None
        if operands[1] == "wzr":
            value = 0
        else:
            value = immediate(operands[1])
            if value is None:
                return None
        for shift, part in movk_parts:
            mask = 0xFFFF << shift
            value = (value & ~mask) | (part << shift)
        return value
    return None


def rng_sites(body: bytes, start: int) -> list[dict[str, Any]]:
    decoder = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    instructions = list(decoder.disasm(body, start))
    result = []
    for index, instruction in enumerate(instructions):
        if instruction.mnemonic != "bl" or int(instruction.op_str[1:], 16) != RANDOM_RANGE_OFFSET:
            continue
        minimum = register_constant(instructions, index, "w0")
        maximum = register_constant(instructions, index, "w1")
        result.append({
            "instructionOffset": instruction.address - start,
            "minimumInclusive": minimum,
            "maximumExclusive": maximum,
            "outcomes": None if minimum is None or maximum is None else maximum - minimum,
            "argumentKind": "constant" if minimum is not None and maximum is not None else "dynamic-or-control-flow-merged",
        })
    return result


def method_analysis(docs: list[dict[str, Any]], class_name: str, method_name: str) -> dict[str, Any]:
    method, body, start = exact_method(docs, class_name, method_name)
    calls = direct_calls(body, start)
    known = Counter(KNOWN_TARGETS[target] for _, target in calls if target in KNOWN_TARGETS)
    known_sites = [
        {"instructionOffset": offset, "targetModuleOffset": f"0x{target:x}", "method": KNOWN_TARGETS[target]}
        for offset, target in calls
        if target in KNOWN_TARGETS
    ]
    return {
        "type": class_name,
        "method": method_name,
        "token": method["token"],
        "moduleOffset": f"0x{start:x}",
        "nativeSizeBytes": len(body),
        "bodySha256": hashlib.sha256(body).hexdigest(),
        "directCallCount": len(calls),
        "knownDirectCallCounts": dict(sorted(known.items())),
        "knownDirectCallSites": known_sites,
        "randomRangeSites": rng_sites(body, start),
    }


def validate_schema(document: dict[str, Any]) -> dict[str, Any]:
    classes = {row["name"]: row for row in document["record"]["payload"]["classes"]}
    result = {}
    expected = {
        "AdminEvilData": {"<uniqueLevel>k__BackingField": 0x28, "<metIdx>k__BackingField": 0xC8, "<metCount>k__BackingField": 0xD0, "<metPercent>k__BackingField": 0xD8, "<exp>k__BackingField": 0xE0, "<gold>k__BackingField": 0xF0},
        "AdminDropUniqueGearData": {"<index>k__BackingField": 0x10, "<dropRange>k__BackingField": 0x28, "<dropCut>k__BackingField": 0x38, "<gearType>k__BackingField": 0x40, "<gearIndex>k__BackingField": 0x48, "<gearPercent>k__BackingField": 0x50},
    }
    for class_name, fields in expected.items():
        actual = {field["name"]: field["offset"] for field in classes[class_name]["fields"]}
        for field_name, offset in fields.items():
            if actual.get(field_name) != offset:
                raise ValueError(f"schema changed: {class_name}.{field_name}")
        result[class_name] = fields
    return result


def build(methods_path: Path, helpers_path: Path, material_path: Path, schema_path: Path) -> dict[str, Any]:
    docs = documents(methods_path, helpers_path, material_path)
    analyzed = [
        method_analysis(docs, "HunterCtrl", name)
        for name in ("Reward", "RewardMetrial", "GHPHHEFFNKN", "LDHAEMDJCFF", "PlusExp", "CalVillTax", "PlusGold")
    ]
    by_name = {row["method"]: row for row in analyzed}
    required_counts = {
        "Reward": {"HunterCtrl.PlusExp/2": 1, "HunterCtrl.CalVillTax/1": 1, "HunterCtrl.PlusGold/1": 1, "UnityEngine.Random.Range(Int32,Int32)": 2},
        "RewardMetrial": {"HunterCtrl.LDHAEMDJCFF/5": 17, "HunterCtrl.GHPHHEFFNKN/2": 6, "UnityEngine.Random.Range(Int32,Int32)": 50},
        "GHPHHEFFNKN": {"UnityEngine.Random.Range(Int32,Int32)": 15},
        "LDHAEMDJCFF": {"HunterCtrl.GHPHHEFFNKN/2": 1, "UnityEngine.Random.Range(Int32,Int32)": 2},
        "PlusExp": {"GameManager.IsCostumeExpUp/1": 1, "GameManager.GetNeedExp/2": 1},
    }
    for method_name, counts in required_counts.items():
        actual = by_name[method_name]["knownDirectCallCounts"]
        for target, count in counts.items():
            if actual.get(target) != count:
                raise ValueError(f"call graph changed: {method_name} -> {target}")

    reward_order = [
        (row["instructionOffset"], row["method"])
        for row in by_name["Reward"]["knownDirectCallSites"]
        if row["method"] in {"HunterCtrl.PlusExp/2", "HunterCtrl.CalVillTax/1", "HunterCtrl.PlusGold/1"}
    ]
    if reward_order != [(0xFEC, "HunterCtrl.PlusExp/2"), (0x1478, "HunterCtrl.CalVillTax/1"), (0x16DC, "HunterCtrl.PlusGold/1")]:
        raise ValueError(f"Reward mutation order changed: {reward_order}")

    schema = validate_schema(json.loads(schema_path.read_text()))
    return {
        "schemaVersion": 2,
        "contractType": "original-reward-progression-callgraph-evidence",
        "runtimeCompatibility": "evidence-only",
        "sources": [source(methods_path, "reward-progression-methods"), source(helpers_path, "reward-progression-helpers"), source(material_path, "reward-material-full"), source(schema_path, "reward-progression-schema")],
        "methods": analyzed,
        "recoveredExactFacts": {
            "rewardMutationOrder": ["HunterCtrl.PlusExp/2", "HunterCtrl.CalVillTax/1", "HunterCtrl.PlusGold/1"],
            "rewardMaterialDispatch": {"LDHAEMDJCFFCalls": 17, "GHPHHEFFNKNCalls": 6, "randomRangeCalls": 50, "ordinaryMaterialEmissionCallOffset": "0xeb4"},
            "helperRngFamilies": {
                "GHPHHEFFNKN": {"Range(0,100) constant": 9, "upperBound100WithMergedMinimum": 1, "Range(0,1000)": 2, "Range(0,10000)": 3},
                "LDHAEMDJCFF": {"Range(0,20)": 1, "Range(0,3)": 1},
                "Reward": {"Range(0,101)": 1, "Range(0,100)": 1},
            },
            "uniqueGearSchema": schema,
        },
        "interpretationBoundary": [
            "GHPHHEFFNKN is an exact ObscuredFloat-returning helper with fifteen integer RNG calls, but its obfuscated parameter and branch semantics are not renamed.",
            "LDHAEMDJCFF is an exact reward-emission helper used by the ordinary material loop and also contains one GHPHHEFFNKN call; it is not labeled unique-gear-only.",
            "RewardMetrial contains constant and dynamic RNG ranges beyond the ordinary loop. Constant ranges prove denominators/outcome counts, not product meaning by themselves.",
            "AdminEvilData.uniqueLevel and AdminDropUniqueGearData fields are schema-confirmed, but a direct native uniqueLevel-to-pool lookup and dropCut/gearPercent evaluation order are still unresolved.",
            "The complete semantic EXP/gold/material modifier names and branch conditions remain incomplete even where arithmetic and call order are exact.",
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--methods", type=Path, default=DEFAULT_METHODS)
    parser.add_argument("--helpers", type=Path, default=DEFAULT_HELPERS)
    parser.add_argument("--material", type=Path, default=DEFAULT_MATERIAL)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    evidence = build(args.methods, args.helpers, args.material, args.schema)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote pass-2 reward call graph to {args.output}")


if __name__ == "__main__":
    main()
