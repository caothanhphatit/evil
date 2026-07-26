#!/usr/bin/env python3
"""Validate the recovered building Town Hall condition contract."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "reverse-engineering/evidence/building-condition-evidence-v1.json"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def main() -> None:
    evidence = json.loads(EVIDENCE.read_text())
    require(evidence["schemaVersion"] == 1, "unexpected schema version")
    require(evidence["evaluator"]["subjectId"] == "build_1.level", "subject mismatch")
    require(evidence["evaluator"]["operator"] == "greater-than-or-equal", "operator mismatch")

    for source in evidence["sources"].values():
        path = ROOT / source["path"]
        payload = path.read_bytes()
        expected = source.get("assetSha256", source.get("sha256"))
        require(hashlib.sha256(payload).hexdigest() == expected, f"source hash mismatch: {path}")

    localized = {row["key"]: row for row in evidence["localizationRows"]}
    require(set(localized) == {"buildpop_9", "buildtoast_0"}, "condition UI key mismatch")
    require(
        "Town Hall Lv.{0} or higher required." in localized["buildpop_9"]["localized"]["en"],
        "build popup condition text mismatch",
    )
    require(
        "Town Hall Lv.{0} or higher required." in localized["buildtoast_0"]["localized"]["en"],
        "build toast condition text mismatch",
    )

    rows = {row["key"]: row for row in evidence["conditionRows"]}
    require(len(rows) == 227, "condition row count mismatch")
    require(rows["build_7:level:1"]["requiredTownHallLevel"] == 2, "weapon shop level 1 mismatch")
    require(rows["build_7:level:5"]["requiredTownHallLevel"] == 11, "weapon shop level 5 mismatch")
    require(rows["build_1:level:17"]["requiredTownHallLevel"] == 1, "Town Hall level 17 mismatch")
    print("Building Town Hall condition evidence is valid")


if __name__ == "__main__":
    main()
