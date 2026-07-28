from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("pass7", ROOT / "tools/analyze-original-reward-progression-pass7.py")
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class OriginalRewardProgressionPass7Test(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.evidence = MODULE.build(MODULE.DEFAULT_CALLGRAPH, MODULE.DEFAULT_ARITHMETIC, MODULE.DEFAULT_METHODS, MODULE.DEFAULT_HELPERS, MODULE.DEFAULT_MATERIAL)

    def test_checked_in_output_is_deterministic(self) -> None:
        self.assertEqual(self.evidence, json.loads(MODULE.DEFAULT_OUTPUT.read_text()))

    def test_object_boundary_fails_closed(self) -> None:
        self.assertIsNone(self.evidence["confirmedCallerFacts"]["boundUniqueLevelAccess"])
        self.assertIsNone(self.evidence["confirmedCallerFacts"]["boundAdminDropUniqueGearDataAccess"])
        self.assertFalse(self.evidence["implementationBoundary"]["liveIntegrationAllowed"])
        self.assertFalse(self.evidence["implementationBoundary"]["arrayOrderFallbackAllowed"])

    def test_helper_signatures_carry_no_drop_row_object(self) -> None:
        signatures = self.evidence["capturedMethods"]
        self.assertNotIn("AdminEvilData", signatures["LDHAEMDJCFF"]["parameterTypes"])
        self.assertNotIn("AdminDropUniqueGearData", signatures["GHPHHEFFNKN"]["parameterTypes"])


if __name__ == "__main__":
    unittest.main()
