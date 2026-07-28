import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "hunter_skill_coefficients_pass13",
    ROOT / "tools/analyze-original-native-hunter-skill-coefficients-pass13.py",
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class HunterSkillCoefficientPass13Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = MODULE.build(MODULE.CALLERS, MODULE.ACTIONS, MODULE.CONSTANT_SCHEMA)

    def test_internal_percent_family_has_six_exact_members(self):
        family = self.evidence["family"]
        self.assertEqual(family["id"], "internal-obscured-int-percent")
        self.assertEqual(len(family["members"]), 6)
        self.assertIn("FCVTZS", family["rounding"])
        self.assertEqual(self.evidence["coverage"]["remainingCallerBodies"], 34)

    def test_named_constant_sources_are_schema_backed(self):
        members = {
            member["method"]["methodName"]: member
            for member in self.evidence["family"]["members"]
        }
        self.assertEqual(
            members["KMFIIOFLHKC"]["coefficientSource"]["field"],
            "BLOW_DESTRUCTION_POWER_VALUE",
        )
        self.assertEqual(
            members["BBOCACECAAO"]["coefficientSource"]["field"],
            "VENUM_RAIN_POWER_VALUE",
        )
        self.assertEqual(
            members["CHKMAHCLJBN"]["coefficientSource"]["field"],
            "CURSE_CHAIN_POWER_VALUE",
        )
        self.assertEqual(
            members["BCLCCDFCHFC"]["coefficientSource"]["field"],
            "DARK_RIFT_POWER_VALUE",
        )
        self.assertEqual(
            members["IABOOKJBHHO"]["coefficientSource"]["field"],
            "POISON_FANG_POWER_VALUE",
        )

    def test_routes_remain_separate_from_public_skill_names(self):
        members = self.evidence["family"]["members"]
        flame = [member for member in members if member["action"] == "FlameExplosionCtrl.Action"]
        virtual = [member for member in members if member["action"] == "EvilCtrl virtual slot +0x2A8"]
        self.assertEqual(len(flame), 5)
        self.assertEqual(len(virtual), 1)
        self.assertIsNone(virtual[0]["managedActionTarget"])
        self.assertTrue(any("public skill-row mappings" in gap for gap in self.evidence["unresolved"]))


if __name__ == "__main__":
    unittest.main()
