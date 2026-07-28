import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "tools/analyze-original-native-hunter-constant-coefficients-pass16.py"
SPEC = importlib.util.spec_from_file_location("hunter_constant_coefficients_pass16", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class HunterConstantCoefficientPass16Test(unittest.TestCase):
    def setUp(self):
        self.document = MODULE.build(MODULE.CALLERS, MODULE.ACTIONS, MODULE.PASS9_TARGETS, MODULE.CONSTANT_SCHEMA, MODULE.STATIC_FACTORS)

    def test_exact_methods_and_vectors(self):
        rows = {row["method"]["methodName"]: row for row in self.document["methods"]}
        self.assertEqual(
            set(rows),
            {"BPOGPFGALFD", "PCGIDDENIJL", "NMAIFPMMBHE", "DNPJKKJPHLD", "NPIAALIFANE", "EHKBOGAOFEN"},
        )
        self.assertEqual(rows["DNPJKKJPHLD"]["getDamageVector"], [True, False, False])
        self.assertEqual(rows["NMAIFPMMBHE"]["route"]["damageParameter"], 6)
        self.assertEqual(rows["EHKBOGAOFEN"]["route"]["selectorParameter6"], 2)

    def test_constant_bindings_are_schema_backed(self):
        rows = {row["method"]["methodName"]: row for row in self.document["methods"]}
        self.assertEqual(rows["BPOGPFGALFD"]["constantFields"][0]["name"], "POISON_AURA_POWER_VALUE")
        self.assertEqual(rows["PCGIDDENIJL"]["constantFields"][0]["name"], "CURSEAURA_POWER_VALUE")
        self.assertIn(
            "FROZEN_HEART_SHADOW_STRIKE_SKILL_UP_VALUE",
            {field["name"] for field in rows["DNPJKKJPHLD"]["constantFields"]},
        )
        self.assertEqual(rows["NPIAALIFANE"]["constantFields"][1]["name"], "FROST_ARCHER_SNIPING_SKILL_UP_VALUE")

    def test_generated_evidence_is_current(self):
        committed = json.loads(MODULE.OUTPUT.read_text())
        self.assertEqual(committed, self.document)

    def test_remains_disconnected(self):
        self.assertEqual(self.document["integrationStatus"], "disconnected_no_live_combat_use")
        self.assertIn("do not subtract", self.document["classification"]["coveragePolicy"])


if __name__ == "__main__":
    unittest.main()
