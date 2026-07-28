import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "hunter_fixed_scale_coefficients_pass15",
    ROOT / "tools/analyze-original-native-hunter-fixed-scale-coefficients-pass15.py",
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class HunterFixedScaleCoefficientPass15Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = MODULE.build(MODULE.CALLERS, MODULE.ACTIONS, MODULE.CONSTANT_SCHEMA, MODULE.XAPK)

    def test_exact_package_literals_remain_distinct(self):
        literals = self.evidence["packageLiterals"]
        self.assertEqual(literals["integerPercentScale"]["rawHex"], "0ad7233c")
        self.assertEqual(literals["featherShotFixedScale"]["value"], 1193.0)
        self.assertEqual(literals["darkRiftFixedScale"]["value"], 1597.0)

    def test_feather_shot_siblings_share_field_but_not_scale(self):
        members = self.evidence["families"][0]["members"]
        self.assertEqual([member["method"]["methodName"] for member in members], ["JDONOEEBDCD", "BGIJEDLALGE"])
        self.assertTrue(all(member["coefficientSource"]["name"] == "FEATHER_SHOT_POWER_VALUE" for member in members))
        self.assertNotEqual(members[0]["packageScale"]["value"], members[1]["packageScale"]["value"])
        self.assertTrue(all(member["damageRoute"]["damageArgumentRegister"] == "x4" for member in members))

    def test_dark_rift_fixed_sibling_keeps_roundtrip_and_route(self):
        member = self.evidence["families"][1]["members"][0]
        self.assertEqual(member["method"]["methodName"], "BGHEAJHAICN")
        self.assertEqual(member["coefficientSource"]["name"], "DARK_RIFT_POWER_VALUE")
        self.assertEqual(member["getDamageVector"], [True, True, False])
        self.assertEqual(member["damageRoute"]["target"], "FlameExplosionCtrl.Action")
        self.assertEqual(member["damageRoute"]["selectorParameter6"], 1)

    def test_coverage_is_arithmetic_only(self):
        coverage = self.evidence["coverage"]
        self.assertEqual(coverage["coefficientArithmeticResolvedThisPass"], 3)
        self.assertEqual(coverage["remainingCoefficientArithmeticCallerBodies"], 31)
        self.assertIn("lower than arithmetic coverage", coverage["semanticCallerResolution"])


if __name__ == "__main__":
    unittest.main()
