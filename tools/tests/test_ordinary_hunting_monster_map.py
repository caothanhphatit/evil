from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
GENERATOR_PATH = ROOT / "tools/generate-ordinary-hunting-monster-map.py"
SPEC = importlib.util.spec_from_file_location("ordinary_hunting_monster_map", GENERATOR_PATH)
assert SPEC and SPEC.loader
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)


class OrdinaryHuntingMonsterMapTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.mapping = GENERATOR.generate(GENERATOR.DEFAULT_CATALOG)
        cls.catalog = json.loads(GENERATOR.DEFAULT_CATALOG.read_text())
        cls.catalog_rows = {
            row["index"]: row
            for group in cls.catalog["groups"]
            for row in group["monsters"]
        }

    def test_maps_exactly_three_regions_five_difficulties_three_types(self) -> None:
        self.assertEqual([row["area"] for row in self.mapping["regions"]], [0, 1, 2])
        rows = []
        for region in self.mapping["regions"]:
            self.assertEqual(len(region["difficulties"]), 5)
            for difficulty in region["difficulties"]:
                self.assertEqual(difficulty["globalDifficulty"], difficulty["createLevel"])
                self.assertEqual(
                    [row["type"] for row in difficulty["monsterPool"]], [0, 1, 2]
                )
                rows.extend(difficulty["monsterPool"])
        self.assertEqual(len(rows), 45)
        self.assertEqual(len({row["sourceIndex"] for row in rows}), 45)

    def test_source_indices_follow_exact_packaged_tuples(self) -> None:
        for region in self.mapping["regions"]:
            area = region["area"]
            for difficulty in region["difficulties"]:
                level = difficulty["globalDifficulty"]
                self.assertEqual(
                    [row["sourceIndex"] for row in difficulty["monsterPool"]],
                    [area * 15 + monster_type * 5 + level for monster_type in range(3)],
                )

    def test_preserves_exact_stats_and_material_slots(self) -> None:
        for region in self.mapping["regions"]:
            for difficulty in region["difficulties"]:
                for row in difficulty["monsterPool"]:
                    source = self.catalog_rows[row["sourceIndex"]]
                    for field in (
                        "uniqueLevel",
                        "race",
                        "hp",
                        "damage",
                        "armor",
                        "experience",
                        "gold",
                        "materials",
                    ):
                        self.assertEqual(row[field], source[field])

    def test_keeps_unique_gear_binding_unresolved(self) -> None:
        unique = self.mapping["uniqueGear"]
        self.assertEqual(unique["availablePoolIndices"], list(range(61)))
        self.assertIsNone(unique["selectionOrder"])
        self.assertIsNone(unique["uniqueLevelToPoolBinding"])


if __name__ == "__main__":
    unittest.main()
