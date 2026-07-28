from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
GENERATOR_PATH = ROOT / "tools/generate-monster-runtime-catalog.py"
SPEC = importlib.util.spec_from_file_location("monster_runtime_catalog", GENERATOR_PATH)
assert SPEC and SPEC.loader
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)


class MonsterRuntimeCatalogTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.catalog = GENERATOR.generate(
            GENERATOR.DEFAULT_TABLES,
            GENERATOR.DEFAULT_SCHEMA,
            GENERATOR.DEFAULT_NATIVE,
            GENERATOR.DEFAULT_RANDOM,
        )
        cls.tables = json.loads(GENERATOR.DEFAULT_TABLES.read_text())

    def test_preserves_every_monster_and_non_unique_composite_key(self) -> None:
        rows = [row for group in self.catalog["groups"] for row in group["monsters"]]
        self.assertEqual(len(rows), 195)
        self.assertEqual(sorted(row["index"] for row in rows), list(range(195)))
        self.assertEqual(self.catalog["monsterKey"]["cardinality"], "one-to-many")
        self.assertTrue(any(len(group["monsters"]) > 1 for group in self.catalog["groups"]))

    def test_exact_combat_reward_and_material_arrays(self) -> None:
        normalized = {
            row["index"]: row
            for group in self.catalog["groups"]
            for row in group["monsters"]
        }
        for source in self.tables["monsters"]:
            row = normalized[source["index"]]
            self.assertEqual(
                [row[name] for name in ("hp", "damage", "armor", "experience", "gold")],
                [source[name] for name in ("hp", "damage", "armor", "experience", "gold")],
            )
            self.assertEqual(row["materials"]["indices"], source["materialIndices"])
            self.assertEqual(row["materials"]["counts"], source["materialCounts"])
            self.assertEqual(row["materials"]["percentValues"], source["materialPercentValues"])

    def test_exact_unique_gear_range_cut_and_pool_arrays(self) -> None:
        self.assertEqual(len(self.catalog["uniqueGearPools"]), 61)
        for source, row in zip(self.tables["uniqueGearDrops"], self.catalog["uniqueGearPools"]):
            self.assertEqual(row["dropRange"], source["dropRange"])
            self.assertEqual(row["dropCut"], source["dropCut"])
            self.assertEqual(row["gearPool"]["types"], source["gearTypes"])
            self.assertEqual(row["gearPool"]["indices"], source["gearIndices"])
            self.assertEqual(row["gearPool"]["percentValues"], source["gearPercentValues"])

    def test_material_roll_and_order_match_native_control_flow(self) -> None:
        semantics = self.catalog["rewardSemantics"]
        self.assertEqual(semantics["materialPercentDenominator"], 1000)
        self.assertEqual(semantics["materialRollDenominator"], 10000)
        self.assertEqual(
            semantics["materialRoll"],
            {
                "api": "UnityEngine.Random.Range(System.Int32,System.Int32)",
                "token": 100665478,
                "address": "0x73ad876240",
                "moduleOffset": "0x5a76240",
                "minInclusive": 1,
                "maxExclusive": 10001,
                "outcomes": "1..10000",
            },
        )
        self.assertEqual(
            semantics["materialThreshold"]["baseFormula"],
            "materialPercentValues[slot] * 10",
        )
        self.assertEqual(
            semantics["materialThreshold"]["grantComparison"],
            "effectiveThreshold >= roll",
        )
        self.assertIsNone(semantics["materialThreshold"]["modifierFormula"])
        self.assertEqual(
            semantics["materialSelectionOrder"]["order"], "ascending-array-slot"
        )
        self.assertEqual(
            semantics["materialSelectionOrder"]["loopBound"],
            "materialIndices.length",
        )

    def test_misaligned_trailing_percent_is_not_in_primary_loop(self) -> None:
        anomaly = self.catalog["rewardSemantics"]["packagedArrayAnomalies"]
        self.assertEqual(
            anomaly,
            [
                {
                    "monsterIndex": 34,
                    "materialArrayLengths": {
                        "indices": 13,
                        "counts": 13,
                        "percentValues": 14,
                    },
                }
            ],
        )
        self.assertEqual(anomaly[0]["materialArrayLengths"]["indices"], 13)

    def test_unique_gear_semantics_fail_closed(self) -> None:
        semantics = self.catalog["rewardSemantics"]
        self.assertIsNone(semantics["uniqueGearSelectionOrder"])
        self.assertIsNone(semantics["uniqueLevelToPoolBinding"])
        self.assertEqual(
            {(row["className"], row["methodName"], row["token"]) for row in semantics["nativeMethods"]},
            {
                ("EvilCtrl", "Dead", 100675559),
                ("HunterCtrl", "RewardMetrial", 100686745),
                ("HunterCtrl", "Reward", 100686803),
                ("HunterCtrl", "PlusGold", 100686864),
            },
        )


if __name__ == "__main__":
    unittest.main()
