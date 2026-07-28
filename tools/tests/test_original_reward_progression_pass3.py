from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PATH = ROOT / "tools/analyze-original-reward-progression-pass3.py"
SPEC = importlib.util.spec_from_file_location("reward_progression_pass3", PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class OriginalRewardProgressionPass3Test(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.evidence = MODULE.build(MODULE.DEFAULT_METHODS, MODULE.DEFAULT_HELPERS, MODULE.DEFAULT_MATERIAL, MODULE.DEFAULT_SCHEMA)

    def test_checked_in_output_is_deterministic(self) -> None:
        self.assertEqual(self.evidence, json.loads(MODULE.DEFAULT_OUTPUT.read_text()))

    def test_complete_native_arithmetic_counts(self) -> None:
        traces = {row["method"]: row for row in self.evidence["orderedNativeArithmetic"]}
        self.assertEqual(len(traces["PlusExp"]["operations"]), 32)
        self.assertEqual(len(traces["Reward"]["operations"]), 99)
        self.assertEqual(len(traces["CalVillTax"]["operations"]), 10)
        self.assertEqual(len(traces["PlusGold"]["operations"]), 3)
        self.assertEqual([row["operation"] for row in traces["PlusGold"]["operations"]], ["scvtf", "fmul", "fcvtzs"])

    def test_monster_row_fields_are_bound_without_inventing_unique_pool(self) -> None:
        trace = self.evidence["uniqueDropTrace"]
        fields = {row["field"] for row in trace["confirmedAdminEvilRowAccesses"]}
        self.assertEqual(fields, {"metIdx", "metCount", "metPercent", "type"})
        self.assertIsNone(trace["uniqueLevelDirectRowAccess"])
        self.assertIsNone(trace["adminDropUniqueGearRowAccess"])
        self.assertIsNone(trace["poolLinkage"])

    def test_semantic_chains_fail_closed(self) -> None:
        status = self.evidence["semanticChainStatus"]
        self.assertFalse(status["experience"]["completeSemanticOperandBinding"])
        self.assertFalse(status["gold"]["completeSemanticOperandBinding"])
        self.assertEqual(status["experience"]["finalIntegerConversion"], "fcvtzs")


if __name__ == "__main__":
    unittest.main()
