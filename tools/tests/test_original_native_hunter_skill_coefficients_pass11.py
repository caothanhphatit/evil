import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "hunter_skill_coefficients_pass11",
    ROOT / "tools/analyze-original-native-hunter-skill-coefficients-pass11.py",
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class HunterSkillCoefficientPass11Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = MODULE.build(MODULE.CALLERS, MODULE.ACTIONS, MODULE.PASS9_TARGETS)

    def test_three_formula_families_cover_eight_exact_callers(self):
        families = self.evidence["families"]
        self.assertEqual([family["id"] for family in families], [
            "plain-single-percent",
            "obscured-float-percent",
            "affine-obscured-float-percent",
        ])
        self.assertEqual(sum(len(family["members"]) for family in families), 8)
        self.assertEqual(self.evidence["coverage"]["remainingCallerBodies"], 40)

    def test_action_targets_and_damage_parameter_positions_are_exact(self):
        members = {
            member["method"]["methodName"]: member
            for family in self.evidence["families"]
            for member in family["members"]
        }
        self.assertEqual(members["HIDAPNPHFCA"]["action"], "BlizzardCtrl.Action")
        self.assertEqual(members["HIDAPNPHFCA"]["damageParameter"], 6)
        self.assertEqual(members["PMFEHNBKEIL"]["damageParameter"], 3)
        self.assertEqual(members["CHOGGFICJPL"]["actionSelector"]["parameter6"], 1)
        self.assertEqual(members["JHAAACFJNPA"]["actionSelector"]["parameter6"], 0)

    def test_virtual_target_and_public_skill_names_remain_unresolved(self):
        target = self.evidence["actionTargets"]["evilVirtualDamageBoundary"]
        self.assertIsNone(target["managedTarget"])
        self.assertTrue(any("public skill-row names" in gap for gap in self.evidence["unresolved"]))


if __name__ == "__main__":
    unittest.main()
