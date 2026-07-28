import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "hunter_damage_contract_pass9",
    ROOT / "tools/analyze-original-native-hunter-damage-contract-pass9.py",
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class HunterDamageContractPass9Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = MODULE.build(MODULE.CALLERS, MODULE.TARGETS)

    def test_caller_vectors_are_complete_and_basic_attack_is_separate(self):
        boundary = self.evidence["getDamageCallerBoundary"]
        self.assertEqual(boundary["directCallerCount"], 49)
        self.assertEqual(
            boundary["argumentVectorCounts"],
            {
                "false,false,false": 2,
                "true,false,false": 10,
                "true,false,true": 36,
                "true,true,false": 1,
            },
        )
        self.assertEqual(boundary["basicAttack"]["method"], "HuntingAttackAction")
        self.assertEqual(boundary["basicAttack"]["arguments"], [0, 0, 0])

    def test_blizzard_coefficient_segment_is_bounded(self):
        segment = self.evidence["provenSkillCoefficientSegment"]
        self.assertEqual(segment["getDamageArguments"], [True, False, True])
        self.assertIn("second Single argument", segment["coefficientInput"])
        self.assertIn("BlizzardCtrl.Action parameter 6", segment["forwarding"])

    def test_evil_parameter_six_is_only_mechanically_named(self):
        roles = self.evidence["evilDamagedParameterRoles"]
        self.assertIsNone(roles["parameter3"]["semanticName"])
        self.assertIsNone(roles["parameter4"]["semanticName"])
        self.assertIsNone(roles["parameter5"]["semanticName"])
        self.assertIn("mechanical role only", roles["parameter6"]["semanticName"])
        self.assertTrue(
            any("bypasses" in use for use in roles["parameter6"]["provenUses"])
        )


if __name__ == "__main__":
    unittest.main()
