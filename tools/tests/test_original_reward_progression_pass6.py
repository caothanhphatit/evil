from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PATH = ROOT / "tools/analyze-original-reward-progression-pass6.py"
SPEC = importlib.util.spec_from_file_location("reward_progression_pass6", PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class OriginalRewardProgressionPass6Test(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.evidence = MODULE.build(MODULE.DEFAULT_METHODS, MODULE.DEFAULT_REWARD_SCHEMA, MODULE.DEFAULT_HUNTER_SCHEMA, MODULE.DEFAULT_LITERAL)

    def test_checked_in_output_is_deterministic(self) -> None:
        self.assertEqual(self.evidence, json.loads(MODULE.DEFAULT_OUTPUT.read_text()))

    def test_mutation_order_and_sinks(self) -> None:
        self.assertEqual(len(self.evidence["mutationOrder"]), 3)
        self.assertEqual(self.evidence["plusGold"]["sink"], "if resulting grant >=1, HunterData.money += grant")

    def test_tax_fraction_carry_vector(self) -> None:
        self.assertEqual(MODULE.tax_segment(20, 10, 0.4, 2.75, 100), (18, 13, 0.1499999999999999))

    def test_plus_gold_early_stage_scale_is_exact(self) -> None:
        self.assertEqual(MODULE.plus_gold_segment(10, 5, 4, 3), (3, 8))
        self.assertEqual(MODULE.plus_gold_segment(10, 5, 2, 3), (10, 15))

    def test_full_chain_remains_fail_closed(self) -> None:
        boundary = self.evidence["implementationBoundary"]
        self.assertFalse(boundary["fullGoldenCallerVectorAvailable"])
        self.assertFalse(boundary["liveIntegrationAllowed"])


if __name__ == "__main__":
    unittest.main()
