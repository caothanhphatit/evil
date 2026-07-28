from __future__ import annotations

import importlib.util
import json
import unittest
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PATH = ROOT / "tools/analyze-original-reward-progression-pass2.py"
SPEC = importlib.util.spec_from_file_location("reward_progression_pass2", PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class OriginalRewardProgressionPass2Test(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.evidence = MODULE.build(MODULE.DEFAULT_METHODS, MODULE.DEFAULT_HELPERS, MODULE.DEFAULT_MATERIAL, MODULE.DEFAULT_SCHEMA)
        cls.methods = {row["method"]: row for row in cls.evidence["methods"]}

    def test_checked_in_evidence_is_deterministic(self) -> None:
        self.assertEqual(self.evidence, json.loads(MODULE.DEFAULT_OUTPUT.read_text()))

    def test_reward_mutation_order(self) -> None:
        self.assertEqual(
            self.evidence["recoveredExactFacts"]["rewardMutationOrder"],
            ["HunterCtrl.PlusExp/2", "HunterCtrl.CalVillTax/1", "HunterCtrl.PlusGold/1"],
        )

    def test_reward_material_helper_call_counts(self) -> None:
        calls = self.methods["RewardMetrial"]["knownDirectCallCounts"]
        self.assertEqual(calls["HunterCtrl.LDHAEMDJCFF/5"], 17)
        self.assertEqual(calls["HunterCtrl.GHPHHEFFNKN/2"], 6)
        self.assertEqual(calls["UnityEngine.Random.Range(Int32,Int32)"], 50)

    def test_helper_rng_range_families(self) -> None:
        ghph = Counter(
            (row["minimumInclusive"], row["maximumExclusive"])
            for row in self.methods["GHPHHEFFNKN"]["randomRangeSites"]
        )
        self.assertEqual(ghph[(0, 100)], 9)
        self.assertEqual(ghph[(None, 100)], 1)
        self.assertEqual(ghph[(0, 1000)], 2)
        self.assertEqual(ghph[(0, 10000)], 3)
        ldha = Counter(
            (row["minimumInclusive"], row["maximumExclusive"])
            for row in self.methods["LDHAEMDJCFF"]["randomRangeSites"]
        )
        self.assertEqual(ldha, Counter({(0, 20): 1, (0, 3): 1}))

    def test_unique_linkage_remains_fail_closed(self) -> None:
        boundary = " ".join(self.evidence["interpretationBoundary"])
        self.assertIn("uniqueLevel-to-pool", boundary)
        self.assertIn("still unresolved", boundary)


if __name__ == "__main__":
    unittest.main()
