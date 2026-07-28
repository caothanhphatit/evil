import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "tools/analyze-il2cpp-native-combat.py"
SPEC = importlib.util.spec_from_file_location("analyze_il2cpp_native_combat", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class AnalyzeIl2CppNativeCombatTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.evidence = MODULE.build(MODULE.DEFAULT_CAPTURE, MODULE.DEFAULT_SCHEMAS)
        cls.methods = {
            (row["type"], row["method"]): row for row in cls.evidence["methods"]
        }

    def test_all_exact_method_boundaries_are_normalized(self) -> None:
        self.assertEqual(len(self.evidence["methods"]), 16)
        for method in self.evidence["methods"]:
            if method["codeTruncated"]:
                self.assertIsNone(method["bodySha256"])
            else:
                self.assertEqual(len(method["bodySha256"]), 64)
            self.assertEqual(len(method["capturedCodeSha256"]), 64)
            self.assertEqual(len(method["exactBoundaryDescriptorSha256"]), 64)
            self.assertEqual(len(method["callTargetNormalizedSha256"]), 64)
            self.assertEqual(len(method["floatArithmeticSignatureSha256"]), 64)

    def test_reduce_attack_value_exact_formula_signature(self) -> None:
        method = self.methods[("EvilCtrl", "GetReduceAttackValue")]
        self.assertEqual(method["nativeSizeBytes"], 88)
        loads = [
            access
            for access in method["selfFieldAccesses"]
            if access["operation"] == "ldr_s"
        ]
        self.assertEqual([access["offset"] for access in loads], [484, 492, 500])
        self.assertEqual(
            [access["schemaFields"][0]["name"] for access in loads],
            ["PHKJDKANJOA", "DBGLOFBIEBL", "BOKEIJGPICA"],
        )
        self.assertEqual(
            method["floatArithmeticSignature"], "fsub,fsub,fsub,fmul,fmul"
        )
        self.assertIn(
            1.0, [constant["value"] for constant in method["floatImmediateConstants"]]
        )

    def test_costume_modifiers_are_exact_zero_return_bodies(self) -> None:
        facts = self.evidence["recoveredExactFacts"]["costumeModifiers"]
        self.assertEqual(facts["GetCostumeAttackUp"]["nativeSizeBytes"], 8)
        self.assertEqual(facts["GetCostumeAttackUp"]["returns"], 0.0)
        self.assertEqual(facts["GetCostumeArmorUp"]["nativeSizeBytes"], 8)
        self.assertEqual(facts["GetCostumeArmorUp"]["returns"], 0.0)
        self.assertEqual(
            facts["GetCostumeAttackUp"]["bodySha256"],
            facts["GetCostumeArmorUp"]["bodySha256"],
        )

    def test_critical_damage_contains_native_base_multiplier(self) -> None:
        fact = self.evidence["recoveredExactFacts"]["criticalDamage"]
        self.assertEqual(fact["baseMultiplier"], 1.75)
        self.assertEqual(fact["instructionOffsets"], [164, 252])

    def test_direct_call_targets_are_normalized_to_module_offsets(self) -> None:
        method = self.methods[("HunterCtrl", "getDamage")]
        self.assertTrue(method["directCalls"])
        self.assertTrue(
            all(call["targetModuleOffset"].startswith("0x") for call in method["directCalls"])
        )


if __name__ == "__main__":
    unittest.main()
