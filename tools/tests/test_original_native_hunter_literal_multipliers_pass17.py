import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "hunter_literal_multipliers_pass17",
    ROOT / "tools/analyze-original-native-hunter-literal-multipliers-pass17.py",
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class HunterLiteralMultiplierPass17Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = MODULE.build(MODULE.CALLERS, MODULE.ACTIONS, MODULE.CONSTANT_SCHEMA)

    def test_three_exact_literal_members(self):
        members = self.evidence["family"]["members"]
        self.assertEqual([member["method"]["methodName"] for member in members], ["MEDDIMPJHDA", "BOKBBDIDLJG", "KKHDNNMAOKA"])
        self.assertEqual([member["literalMultiplier"]["float32"] for member in members], [286.0, 5.0, 3.0])
        self.assertTrue(all(member["getDamageVector"] == [True, False, True] for member in members))

    def test_flame_routes_keep_selector_difference(self):
        members = self.evidence["family"]["members"]
        self.assertEqual(members[0]["action"]["selectorParameter6"], 1)
        self.assertEqual(members[1]["action"]["selectorParameter6"], 0)
        self.assertEqual(members[0]["action"]["objectNameSource"]["name"], "FLAMEEXPLOSION_OBJ_NAME")

    def test_divine_route_stays_unresolved(self):
        member = self.evidence["family"]["members"][2]
        self.assertEqual(member["action"]["targetModuleOffset"], "0x2b2b734")
        self.assertIsNone(member["action"]["managedIdentity"])
        self.assertEqual(member["action"]["objectNameSource"]["name"], "DIVINEATTACK_OBJ_NAME")

    def test_result_discriminator_is_not_semantically_guessed(self):
        for member in self.evidence["family"]["members"]:
            self.assertEqual(member["resultPayload"]["sourceOffset"], "getDamage result +0x30")
            self.assertIn("no normal/critical enum label", member["resultPayload"]["semanticStatus"])
        self.assertEqual(self.evidence["classification"]["fullyClosedCallersThisPass"], 0)


if __name__ == "__main__":
    unittest.main()
