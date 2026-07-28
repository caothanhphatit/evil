from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PATH = ROOT / "tools/analyze-original-reward-progression-pass5.py"
SPEC = importlib.util.spec_from_file_location("reward_progression_pass5", PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class OriginalRewardProgressionPass5Test(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.evidence = MODULE.build(
            MODULE.DEFAULT_METHODS,
            MODULE.DEFAULT_REWARD_SCHEMA,
            MODULE.DEFAULT_HUNTER_SCHEMA,
            MODULE.DEFAULT_PASS4,
        )

    def test_checked_in_output_is_deterministic(self) -> None:
        self.assertEqual(self.evidence, json.loads(MODULE.DEFAULT_OUTPUT.read_text()))

    def test_known_accumulator_order_is_fixed(self) -> None:
        rows = self.evidence["orderedKnownAccumulatorOperands"]
        self.assertEqual([row["order"] for row in rows], list(range(1, 12)))
        self.assertEqual(rows[0]["operand"], "UserData.mBuildingExpUp")
        self.assertEqual(rows[7]["operand"], "StatusData.CostumeExpUp")
        self.assertEqual(rows[8]["operand"], "StatusData.CollectionExpUp")
        self.assertIn("float32 0.2", rows[4]["rule"])

    def test_stored_level_is_zero_based_for_presentation(self) -> None:
        domain = self.evidence["storedAndDisplayedLevelDomain"]
        self.assertEqual(domain["storedCap"], 99)
        self.assertEqual(domain["displayExpression"], "HunterData.level + 1")
        self.assertEqual(domain["maximumDisplayedValueOnThisPath"], 100)

    def test_incomplete_semantic_chain_stays_disconnected(self) -> None:
        boundary = self.evidence["implementationBoundary"]
        self.assertFalse(boundary["fullGoldenCallerVectorAvailable"])
        self.assertFalse(boundary["liveIntegrationAllowed"])
        self.assertFalse(self.evidence["incomingGrantApplication"]["completeBranchSemanticBinding"])


if __name__ == "__main__":
    unittest.main()
