from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PATH = ROOT / "tools/analyze-original-reward-progression-pass4.py"
SPEC = importlib.util.spec_from_file_location("reward_progression_pass4", PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class OriginalRewardProgressionPass4Test(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.evidence = MODULE.build(MODULE.DEFAULT_METHODS, MODULE.DEFAULT_STATIC, MODULE.DEFAULT_SCHEMA)

    def test_checked_in_output_is_deterministic(self) -> None:
        self.assertEqual(self.evidence, json.loads(MODULE.DEFAULT_OUTPUT.read_text()))

    def test_live_static_cap_decodes_to_level_99(self) -> None:
        cap = self.evidence["maxLevel"]
        self.assertEqual(cap["decodedValue"], 99)
        self.assertEqual(cap["maximumDisplayedValueObservedOnThisPath"], 100)
        self.assertEqual(cap["validGetNeedExpCurrentLevels"], "0..98 before the cap branch")
        self.assertEqual(cap["packagedRowLookup"], "currentLevel + 1, therefore rows 1..99")

    def test_secondary_branch_is_not_the_level_cap(self) -> None:
        branch = self.evidence["secondaryProgressionBranch"]
        self.assertIn("separate from the max-level comparison", branch["interpretation"])
        vectors = {
            (row["stageLevel"], row["revive"], row["hunterLevel"]): row["expected"]
            for row in branch["vectors"]
        }
        self.assertEqual(vectors[(6, 5, 99)], 100)
        self.assertEqual(vectors[(7, 5, 99)], 125)
        self.assertEqual(vectors[(6, 4, 99)], 75)
        self.assertEqual(vectors[(6, 5, 98)], 75)

    def test_schema_bindings_are_exact_runtime_offsets(self) -> None:
        bindings = self.evidence["secondaryProgressionBranch"]["schemaBindings"]
        self.assertEqual(bindings["HunterData"]["<level>k__BackingField"], 0x88)
        self.assertEqual(bindings["HunterData"]["<revive>k__BackingField"], 0xC4)
        self.assertEqual(bindings["UserData"]["<mStageLevel>k__BackingField"], 0x5D8)
        self.assertEqual(bindings["UserData"]["<mBuildingSoulUp>k__BackingField"], 0x9B0)


if __name__ == "__main__":
    unittest.main()
