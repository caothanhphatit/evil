#!/usr/bin/env python3
"""Map exact catalog rows into the three ordinary regions and five difficulties."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CATALOG = ROOT / "packages/content/releases/evil-hunter-1.411/monster-runtime-catalog.json"
DEFAULT_OUTPUT = ROOT / "packages/content/releases/evil-hunter-1.411/ordinary-hunting-monster-map.json"


def load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text())


def source(path: Path) -> dict[str, Any]:
    payload = path.read_bytes()
    return {
        "path": path.resolve().relative_to(ROOT).as_posix(),
        "bytes": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }


def monster_row(key: dict[str, int], row: dict[str, Any]) -> dict[str, Any]:
    return {
        "sourceIndex": row["index"],
        "type": key["type"],
        "uniqueLevel": row["uniqueLevel"],
        "race": row["race"],
        "hp": row["hp"],
        "damage": row["damage"],
        "armor": row["armor"],
        "experience": row["experience"],
        "gold": row["gold"],
        "materials": row["materials"],
    }


def generate(catalog_path: Path) -> dict[str, Any]:
    catalog = load(catalog_path)
    lookup = {
        (group["key"]["area"], group["key"]["type"], group["key"]["createLevel"]): group
        for group in catalog["groups"]
    }

    regions = []
    seen_indices: set[int] = set()
    for area in range(3):
        difficulties = []
        for difficulty in range(5):
            pool = []
            for monster_type in range(3):
                key = (area, monster_type, difficulty)
                group = lookup.get(key)
                if group is None or len(group["monsters"]) != 1:
                    raise ValueError(f"ordinary hunting tuple must resolve exactly once: {key}")
                row = group["monsters"][0]
                if row["index"] in seen_indices:
                    raise ValueError(f"ordinary monster index reused: {row['index']}")
                seen_indices.add(row["index"])
                pool.append(monster_row(group["key"], row))
            difficulties.append(
                {
                    "globalDifficulty": difficulty,
                    "createLevel": difficulty,
                    "monsterPool": pool,
                }
            )
        regions.append({"area": area, "difficulties": difficulties})

    if len(seen_indices) != 45:
        raise ValueError("ordinary hunting mapping must contain exactly 45 monster rows")

    return {
        "schemaVersion": 1,
        "mappingId": "evil-hunter-1.411.ordinary-hunting-monsters-v1",
        "runtimeCompatibility": "evidence-only",
        "source": source(catalog_path),
        "dimensions": {
            "areas": [0, 1, 2],
            "globalDifficulties": [0, 1, 2, 3, 4],
            "monsterTypesPerPool": [0, 1, 2],
            "lookup": "(area,type,createLevel) -> exactly one sourceIndex",
        },
        "materialSemantics": {
            "rawPercentDenominator": catalog["rewardSemantics"]["materialPercentDenominator"],
            "rollDenominator": catalog["rewardSemantics"]["materialRollDenominator"],
            "selectionOrder": catalog["rewardSemantics"]["materialSelectionOrder"],
            "threshold": catalog["rewardSemantics"]["materialThreshold"],
        },
        "uniqueGear": {
            "availablePoolIndices": [row["index"] for row in catalog["uniqueGearPools"]],
            "poolDefinitionsSource": "monster-runtime-catalog.json#uniqueGearPools",
            "selectionOrder": catalog["rewardSemantics"]["uniqueGearSelectionOrder"],
            "uniqueLevelToPoolBinding": catalog["rewardSemantics"]["uniqueLevelToPoolBinding"],
            "note": "Do not assign any unique-gear pool to these rows until native linkage and selection order are recovered.",
        },
        "regions": regions,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=DEFAULT_CATALOG)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    mapping = generate(args.catalog)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(mapping, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote 45 ordinary monster rows to {args.output}")


if __name__ == "__main__":
    main()
