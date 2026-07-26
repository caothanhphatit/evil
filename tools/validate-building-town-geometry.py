#!/usr/bin/env python3
"""Validate fail-closed building town geometry evidence."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_EVIDENCE = ROOT / "reverse-engineering/evidence/building-town-geometry-v1.json"


def fail(message: str) -> None:
    raise ValueError(f"building town geometry validation failed: {message}")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    evidence_path = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT_EVIDENCE
    evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
    if evidence.get("schemaVersion") != 1:
        fail("unsupported schemaVersion")
    if evidence.get("runtimeCompatibility") != "evidence-only":
        fail("runtimeCompatibility must remain evidence-only")

    source = evidence["source"]
    for key, hash_key in (
        ("buildingAssetEvidence", "buildingAssetEvidenceSha256"),
        ("sceneEvidence", "sceneEvidenceSha256"),
        ("serializedBuildingTables", "serializedBuildingTablesSha256"),
    ):
        source_path = ROOT / source[key]
        if sha256(source_path) != source[hash_key]:
            fail(f"{key} hash mismatch")

    finding = evidence["placementFinding"]
    if finding["status"] != "runtime-or-save-owned":
        fail("placement status was promoted without evidence")
    if finding["serializedPerBuildingPositions"] != 0:
        fail("serialized per-building positions must remain zero")
    if finding["exactSerializedAnchors"] != 1 or len(evidence["serializedAnchors"]) != 1:
        fail("expected the one exact ReviveBuilding anchor")

    generic = evidence["genericBuildingTemplate"]
    if generic["scope"] != "generic-template-not-bound-to-individual-build-ids":
        fail("generic prefab scope mismatch")
    if generic["controllerClass"] != "BuildCtrl":
        fail("generic controller mismatch")
    if generic["collider"]["type"] != "CapsuleCollider2D":
        fail("generic collider mismatch")
    if generic["bindingToIndividualBuildIds"] is not None:
        fail("generic template must not claim per-ID binding")

    buildings = evidence["buildings"]
    if len(buildings) != 36:
        fail(f"expected 36 exact build visual bindings, got {len(buildings)}")
    ids = [item["sourceBuildIndex"] for item in buildings]
    if len(ids) != len(set(ids)):
        fail("duplicate source build indices")
    for item in buildings:
        expected_key = f"build_{item['sourceBuildIndex']}"
        if item["sourceBuildKey"] != expected_key:
            fail(f"{expected_key} key mismatch")
        if item["resolution"] != "exact-visual-geometry-placement-unresolved":
            fail(f"{expected_key} resolution mismatch")
        for field in ("townPosition", "sorting", "collider"):
            if item[field] is not None:
                fail(f"{expected_key}.{field} must remain unresolved")
        sprite = item["baseSprite"]
        bundle_path = ROOT / "game-assets/source/unity-assets/bin/Data" / sprite["sourceBundle"]
        if sha256(bundle_path) != sprite["sourceBundleSha256"]:
            fail(f"{expected_key} source bundle hash mismatch")
        if sprite["pixelsToUnits"] <= 0:
            fail(f"{expected_key} has invalid pixelsToUnits")
        if sprite["decodedImagePixels"]["width"] <= 0 or sprite["decodedImagePixels"]["height"] <= 0:
            fail(f"{expected_key} has invalid decoded image size")

    print(
        f"Validated building town geometry: buildings={len(buildings)}, "
        f"anchors={len(evidence['serializedAnchors'])}, perIdPositions=0"
    )


if __name__ == "__main__":
    main()
