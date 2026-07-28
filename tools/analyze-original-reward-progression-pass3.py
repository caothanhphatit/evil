#!/usr/bin/env python3
"""Normalize complete native arithmetic order and unique-drop tracing boundaries."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_METHODS = ROOT / "reverse-engineering/evidence/original-reward-progression-methods-api35-v1.json"
DEFAULT_HELPERS = ROOT / "reverse-engineering/evidence/original-reward-progression-helpers-api35-v1.json"
DEFAULT_MATERIAL = ROOT / "reverse-engineering/evidence/original-reward-material-full-api35-v1.json"
DEFAULT_SCHEMA = ROOT / "reverse-engineering/evidence/original-reward-progression-schema-api35-v1.json"
DEFAULT_OUTPUT = ROOT / "reverse-engineering/evidence/original-reward-progression-arithmetic-v3.json"

ARITHMETIC = {"fadd", "fsub", "fmul", "fdiv", "scvtf", "ucvtf", "fcvtzs"}


def source(path: Path, source_id: str) -> dict[str, Any]:
    body = path.read_bytes()
    return {"id": source_id, "path": path.resolve().relative_to(ROOT).as_posix(), "bytes": len(body), "sha256": hashlib.sha256(body).hexdigest()}


def load_methods(*paths: Path) -> dict[tuple[str, str], tuple[dict[str, Any], bytes, int]]:
    result = {}
    for path in paths:
        document = json.loads(path.read_text())
        for method in document["record"]["payload"]["methods"]:
            candidate = method["candidates"][0]
            body = bytes.fromhex(candidate["codeHex"])
            if candidate.get("codeTruncated"):
                continue
            key = (method["className"], method["methodName"])
            current = result.get(key)
            if current is None or len(body) > len(current[1]):
                result[key] = (method, body, int(candidate["moduleOffset"], 16))
    return result


def arithmetic_trace(methods, class_name: str, method_name: str) -> dict[str, Any]:
    method, body, start = methods[(class_name, method_name)]
    decoder = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    operations = [
        {"instructionOffset": instruction.address - start, "operation": instruction.mnemonic, "operands": instruction.op_str}
        for instruction in decoder.disasm(body, start)
        if instruction.mnemonic in ARITHMETIC
    ]
    return {
        "type": class_name,
        "method": method_name,
        "token": method["token"],
        "nativeSizeBytes": len(body),
        "bodySha256": hashlib.sha256(body).hexdigest(),
        "operations": operations,
        "operationSignature": ",".join(row["operation"] for row in operations),
    }


def validate_unique_schema(schema_path: Path) -> dict[str, Any]:
    document = json.loads(schema_path.read_text())
    classes = {row["name"]: row for row in document["record"]["payload"]["classes"]}
    expected = {
        "AdminEvilData": {"uniqueLevel": 0x28, "metIdx": 0xC8, "metCount": 0xD0, "metPercent": 0xD8, "exp": 0xE0, "gold": 0xF0},
        "AdminDropUniqueGearData": {"index": 0x10, "dropRange": 0x28, "dropCut": 0x38, "gearType": 0x40, "gearIndex": 0x48, "gearPercent": 0x50},
    }
    for class_name, fields in expected.items():
        actual = {field["name"].replace("<", "").split(">")[0]: field["offset"] for field in classes[class_name]["fields"]}
        for name, offset in fields.items():
            if actual.get(name) != offset:
                raise ValueError(f"schema offset changed: {class_name}.{name}")
    return expected


def validate_monster_row_accesses(material_body: bytes) -> list[dict[str, Any]]:
    # These instruction sites follow the exact GameManager AdminEvilData list
    # lookup by monster index and therefore bind the accessed row field.
    expected = {
        0x2B0: (0xC8, "metIdx"),
        0x300: (0xC8, "metIdx"),
        0x48C: (0xC8, "metIdx"),
        0x4EC: (0xD8, "metPercent"),
        0xC70: (0xC8, "metIdx"),
        0xD90: (0xD0, "metCount"),
        0xDEC: (0xD0, "metCount"),
        0xE64: (0xC8, "metIdx"),
        0xFD8: (0x38, "type"),
        0x12C0: (0x38, "type"),
        0x23AC: (0x38, "type"),
        0x4D68: (0x38, "type"),
    }
    decoder = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    instructions = {instruction.address: instruction for instruction in decoder.disasm(material_body, 0)}
    rows = []
    for offset, (field_offset, field_name) in expected.items():
        instruction = instructions[offset]
        if f"#0x{field_offset:x}" not in instruction.op_str:
            raise ValueError(f"RewardMetrial field access changed at +{offset:#x}")
        rows.append({"instructionOffset": offset, "fieldOffset": field_offset, "field": field_name, "instruction": f"{instruction.mnemonic} {instruction.op_str}"})
    return rows


def build(methods_path: Path, helpers_path: Path, material_path: Path, schema_path: Path) -> dict[str, Any]:
    methods = load_methods(methods_path, helpers_path, material_path)
    traces = [
        arithmetic_trace(methods, "HunterCtrl", name)
        for name in ("PlusExp", "Reward", "CalVillTax", "PlusGold")
    ]
    expected_counts = {"PlusExp": 32, "Reward": 99, "CalVillTax": 10, "PlusGold": 3}
    for trace in traces:
        if len(trace["operations"]) != expected_counts[trace["method"]]:
            raise ValueError(f"arithmetic trace changed: {trace['method']}")
    material_body = methods[("HunterCtrl", "RewardMetrial")][1]
    row_accesses = validate_monster_row_accesses(material_body)
    schema = validate_unique_schema(schema_path)
    return {
        "schemaVersion": 3,
        "contractType": "original-reward-progression-arithmetic-evidence",
        "runtimeCompatibility": "evidence-only",
        "sources": [source(methods_path, "reward-progression-methods"), source(helpers_path, "reward-progression-helpers"), source(material_path, "reward-material-full"), source(schema_path, "reward-progression-schema")],
        "orderedNativeArithmetic": traces,
        "uniqueDropTrace": {
            "schema": schema,
            "confirmedAdminEvilRowAccesses": row_accesses,
            "uniqueLevelDirectRowAccess": None,
            "adminDropUniqueGearRowAccess": None,
            "poolLinkage": None,
            "dropCutOrder": None,
            "gearPercentDenominator": None,
            "gearTypeIndexRngOrder": None,
        },
        "semanticChainStatus": {
            "experience": {
                "instructionOrder": "complete for PlusExp",
                "knownInputs": ["mBuildingExpUp or 1.0 base", "expScroll +1.0", "BoxExp +0.5", "ExpGemPack_Active +1.0", "CostumeExpUp", "CollectionExpUp", "reviveWisdom", "GearProperty delta * 0.01"],
                "completeSemanticOperandBinding": False,
                "finalIntegerConversion": "fcvtzs",
            },
            "gold": {
                "instructionOrder": "complete for Reward, CalVillTax and PlusGold",
                "mutationOrder": ["Reward arithmetic", "CalVillTax", "PlusGold"],
                "plusGoldTerminalOperations": ["scvtf", "fmul", "fcvtzs"],
                "completeSemanticOperandBinding": False,
            },
        },
        "blockingEvidence": [
            "RewardMetrial has confirmed AdminEvilData metIdx/metCount/metPercent/type row accesses, but no mechanically bound uniqueLevel row access in the traced sites.",
            "No traced object register is yet proven to be an AdminDropUniqueGearData row, so dropRange/dropCut/gear arrays cannot be ordered safely.",
            "The arithmetic instruction sequence is complete, but several singleton/static operands remain unnamed; a complete semantic multiplier formula is not claimed.",
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
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote pass-3 arithmetic evidence to {args.output}")


if __name__ == "__main__":
    main()
