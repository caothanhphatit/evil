import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PATH = ROOT / "tools/analyze-il2cpp-native-gear-formulas.py"
SPEC = importlib.util.spec_from_file_location("gear_formula", PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class GearFormulaAnalysisTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = MODULE.build(MODULE.DEFAULT_CAPTURE, MODULE.DEFAULT_ANALYSIS, MODULE.DEFAULT_SCHEMA)

    def test_exact_boundaries_and_fields(self):
        self.assertEqual({row["method"]: row["nativeSizeBytes"] for row in self.data["methods"]}, MODULE.GEAR_METHODS)
        self.assertEqual([row["offset"] for row in self.data["gearDataInput"]["fields"]], [16, 32, 64, 92, 108])

    def test_armor_acc_golden_vectors_and_ties_to_even(self):
        self.assertEqual([row["result"] for row in self.data["gearArmorAndAcc"]["goldenVectors"]], [100, 80, 145, 10, 12])

    def test_seal_selector_boundary(self):
        self.assertEqual(self.data["sealAttack"]["acceptedInputIds"][:5], [157, 158, 159, 160, 161])
        self.assertIn(822, self.data["sealAttack"]["acceptedInputIds"])
        self.assertEqual(self.data["sealAttack"]["goldenVectors"][2]["result"], 0.0)

    def test_weapon_costume_remains_bounded(self):
        weapon = self.data["weaponCostumeAttack"]
        self.assertEqual(weapon["nativeSizeBytes"], 296)
        self.assertEqual(len(weapon["bodySha256"]), 64)
        self.assertIn("unresolved", weapon["status"])

    def test_get_first_percent_schedule_and_damage_expression(self):
        self.assertEqual(MODULE.first_percent([2, 3, 4, 5, 6, 7, 8, 9, 10], 12), 33)
        self.assertEqual(MODULE.first_percent([2] * 9, -1), 0)
        damage = self.data["gearDamage"]
        self.assertEqual(damage["helper"]["name"], "GetFirstPercent")
        self.assertEqual(damage["helper"]["moduleOffset"], "0x26bdf7c")
        self.assertEqual([row["result"] for row in damage["goldenVectors"][:2]], [100, 216])


if __name__ == "__main__":
    unittest.main()
