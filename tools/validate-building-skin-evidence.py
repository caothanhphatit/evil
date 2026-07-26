#!/usr/bin/env python3
"""Validate building skin evidence and keep unsupported joins fail-closed."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT = ROOT / "reverse-engineering/evidence/building-skin-evidence-v1.json"


def fail(message: str) -> None:
    raise ValueError(f"building skin evidence validation failed: {message}")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def expected_key(skin_id: int, building_id: int) -> str:
    suffix = building_id - 1 if skin_id == 1 else building_id
    return f"buildSkin_{skin_id}_{suffix}"


def expected_prefix(skin_id: int, building_id: int) -> str:
    family = {1: "cos", 2: "halloween", 3: "christmas"}.get(skin_id)
    if family is None:
        fail(f"unsupported skin family {skin_id}")
    return f"bd_a_{family}_{building_id:03d}_"


def main() -> None:
    evidence_path = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT
    evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
    if evidence.get("schemaVersion") != 1 or evidence.get("runtimeCompatibility") != "evidence-only":
        fail("unsupported or runtime-promoted contract")
    sources = evidence["sources"]
    for path_key, hash_key in (
        ("serializedBuildingTables", "serializedBuildingTablesSha256"),
        ("sharedAssets", "sharedAssetsSha256"),
    ):
        if sha256(ROOT / sources[path_key]) != sources[hash_key]:
            fail(f"{path_key} hash mismatch")

    rows = evidence["rows"]
    if len(rows) != 61:
        fail(f"expected 61 serialized skin rows, got {len(rows)}")
    if [row["sourceRowIndex"] for row in rows] != list(range(61)):
        fail("source row indices are not exact and contiguous")
    serialized_rows = json.loads(
        (ROOT / sources["serializedBuildingTables"]).read_text(encoding="utf-8")
    )["buildingSkins"]
    resolved = 0
    unresolved = 0
    for row, serialized in zip(rows, serialized_rows, strict=True):
        for evidence_key, source_key in (
            ("sourceRowIndex", "index"),
            ("buildingId", "buildingId"),
            ("skinId", "skinId"),
            ("titles", "titles"),
            ("requiredGold", "requiredGold"),
            ("requiredMaterialIds", "requiredMaterialIds"),
            ("requiredMaterialQuantities", "requiredMaterialQuantities"),
            ("requiredLevel", "requiredLevel"),
            ("visibility", "visibility"),
        ):
            if row[evidence_key] != serialized[source_key]:
                fail(f"row {row['sourceRowIndex']} serialized {evidence_key} mismatch")
        key = expected_key(row["skinId"], row["buildingId"])
        prefix = expected_prefix(row["skinId"], row["buildingId"])
        binding = row["visualBinding"]
        if binding["state"] == "resolved":
            resolved += 1
            if binding["bindingConfidence"] != "exact-table-rule-clip-controller-sprite-chain":
                fail(f"row {row['sourceRowIndex']} confidence mismatch")
            if binding["assetKey"] != key:
                fail(f"row {row['sourceRowIndex']} asset key mismatch")
            if binding["animationClip"]["name"] != key or binding["animatorController"]["name"] != key:
                fail(f"row {row['sourceRowIndex']} clip/controller name mismatch")
            if binding["animatorController"]["animationClipPathIds"] != [binding["animationClip"]["pathId"]]:
                fail(f"row {row['sourceRowIndex']} controller link mismatch")
            if binding["spritePrefix"] != prefix or not binding["spriteFrames"]:
                fail(f"row {row['sourceRowIndex']} sprite prefix/frames mismatch")
            for frame in binding["spriteFrames"]:
                if not frame["name"].startswith(prefix):
                    fail(f"row {row['sourceRowIndex']} has cross-family sprite")
                source = ROOT / sources["sharedAssets"] if frame["sourceBundle"] is None else ROOT / "game-assets/source/unity-assets/bin/Data" / frame["sourceBundle"]
                if sha256(source) != frame["sourceSha256"]:
                    fail(f"row {row['sourceRowIndex']} sprite source hash mismatch")
        elif binding["state"] == "unresolved":
            unresolved += 1
            if binding["expectedAssetKey"] != key or binding["expectedSpritePrefix"] != prefix:
                fail(f"row {row['sourceRowIndex']} unresolved expectation mismatch")
            if not binding.get("reason"):
                fail(f"row {row['sourceRowIndex']} lacks unresolved reason")
        else:
            fail(f"row {row['sourceRowIndex']} invalid state")

    coverage = evidence["coverage"]
    if (resolved, unresolved) != (47, 14):
        fail(f"expected 47 resolved and 14 unresolved, got {resolved}/{unresolved}")
    if coverage != {
        "orphanAssetKeys": 1,
        "resolvedVisualBindings": 47,
        "serializedRows": 61,
        "unresolvedVisualBindings": 14,
    }:
        fail("coverage mismatch")
    if [item["assetKey"] for item in evidence["orphanAssets"]] != ["buildSkin_3_29"]:
        fail("unexpected orphan asset set")
    print("Validated building skin evidence: rows=61, resolved=47, unresolved=14, orphans=1")


if __name__ == "__main__":
    main()
