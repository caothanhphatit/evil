#!/usr/bin/env python3
"""Generate the exact packaged EXP table and recovered GetNeedExp lookup contract."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_TABLES = ROOT / "reverse-engineering/evidence/hunter-info-tables-v1.json"
DEFAULT_SCHEMA = ROOT / "reverse-engineering/evidence/original-reward-progression-schema-api35-v1.json"
DEFAULT_HELPERS = ROOT / "reverse-engineering/evidence/original-reward-progression-helpers-api35-v1.json"
DEFAULT_OUTPUT = ROOT / "packages/content/releases/evil-hunter-1.411/experience-runtime-catalog.json"


def source(path: Path, source_id: str) -> dict[str, Any]:
    body = path.read_bytes()
    return {
        "id": source_id,
        "path": path.resolve().relative_to(ROOT).as_posix(),
        "bytes": len(body),
        "sha256": hashlib.sha256(body).hexdigest(),
    }


def exact_body(document: dict[str, Any], class_name: str, method_name: str) -> tuple[dict[str, Any], bytes]:
    method = next(
        row
        for row in document["record"]["payload"]["methods"]
        if row["className"] == class_name and row["methodName"] == method_name
    )
    candidate = method["candidates"][0]
    body = bytes.fromhex(candidate["codeHex"])
    if candidate["codeTruncated"] or len(body) != candidate["nativeSizeBytes"]:
        raise ValueError(f"{class_name}.{method_name} is not an exact complete body")
    return method, body


def validate_schema(document: dict[str, Any]) -> None:
    classes = {row["name"]: row for row in document["record"]["payload"]["classes"]}
    fields = {row["name"]: row for row in classes["AdminExpData"]["fields"]}
    expected = {
        "<index>k__BackingField": 0x10,
        "<exp1>k__BackingField": 0x20,
        "<exp2>k__BackingField": 0x30,
        "<exp3>k__BackingField": 0x40,
        "<exp4>k__BackingField": 0x50,
        "<exp5>k__BackingField": 0x60,
        "<exp6>k__BackingField": 0x70,
    }
    actual = {name: fields[name]["offset"] for name in expected}
    if actual != expected:
        raise ValueError(f"AdminExpData offsets changed: {actual}")


def validate_get_need_exp(document: dict[str, Any]) -> dict[str, Any]:
    method, body = exact_body(document, "GameManager", "GetNeedExp")
    if method["token"] != 100690845 or len(body) != 404:
        raise ValueError("GetNeedExp token or exact native size changed")
    expected_words = {
        0x0C: "f303022a",  # currentLevel is the second managed argument (w2).
        0x10: "f403012a",  # revive/difficulty selector is the first argument (w1).
        0x48: "69060011",  # table index = currentLevel + 1.
        0x60: "28810191",  # selector 4 -> exp5 at row offset 0x60.
        0xA0: "28c10091",  # selector 1 -> exp2 at row offset 0x30.
        0xD4: "28010191",  # selector 2 -> exp3 at row offset 0x40.
        0x108: "28410191",  # selector 3 -> exp4 at row offset 0x50.
        0x13C: "28810091",  # selector 0 -> exp1 at row offset 0x20.
        0x170: "28c10191",  # default selector branch -> exp6 at row offset 0x70.
    }
    for offset, word in expected_words.items():
        if body[offset : offset + 4].hex() != word:
            raise ValueError(f"GetNeedExp instruction changed at +{offset:#x}")
    return {
        "token": method["token"],
        "nativeSizeBytes": len(body),
        "bodySha256": hashlib.sha256(body).hexdigest(),
        "inputs": ["revive", "currentLevel"],
        "rowIndexFormula": "currentLevel + 1",
        "columnFormula": "experienceByDifficulty[revive] for revive 0..5",
        "nativeDefaultBranch": "selectors outside 0..4 use the exp6 branch; valid Hunter revive domain still requires runtime-value confirmation",
    }


def generate(tables_path: Path, schema_path: Path, helpers_path: Path) -> dict[str, Any]:
    tables = json.loads(tables_path.read_text())
    schema = json.loads(schema_path.read_text())
    helpers = json.loads(helpers_path.read_text())
    validate_schema(schema)
    lookup = validate_get_need_exp(helpers)
    rows = tables["experience"]
    if [row["index"] for row in rows] != list(range(100)):
        raise ValueError("packaged EXP rows must remain contiguous 0..99")
    if any(len(row["experienceByDifficulty"]) != 6 for row in rows):
        raise ValueError("every packaged EXP row must retain six difficulty/revive values")
    return {
        "schemaVersion": 1,
        "catalogId": "evil-hunter-1.411.experience-runtime-v1",
        "runtimeCompatibility": "evidence-only",
        "sources": [
            source(tables_path, "hunter-info-tables-v1"),
            source(schema_path, "original-reward-progression-schema-api35-v1"),
            source(helpers_path, "original-reward-progression-helpers-api35-v1"),
        ],
        "lookup": lookup,
        "rows": rows,
        "limitations": [
            "The native selector is HunterData.revive, not Hunter job/class.",
            "The numeric global maximum Hunter level is not claimed by this catalog.",
            "Row zero is packaged data but GetNeedExp(currentLevel) indexes row currentLevel + 1.",
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tables", type=Path, default=DEFAULT_TABLES)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--helpers", type=Path, default=DEFAULT_HELPERS)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    output = generate(args.tables, args.schema, args.helpers)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote {len(output['rows'])} EXP rows to {args.output}")


if __name__ == "__main__":
    main()
