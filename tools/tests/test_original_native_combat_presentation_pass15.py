import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "tools/analyze-original-native-combat-presentation-pass15.py"


def load_analyzer():
    spec = importlib.util.spec_from_file_location("combat_presentation_pass15", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class CombatPresentationPass15Test(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.analyzer = load_analyzer()
        cls.result = cls.analyzer.build()

    def test_damage_discriminators_are_bound(self):
        types = self.result["damageCtrl"]["types"]
        self.assertEqual(types["0"]["role"], "incoming_damage")
        self.assertEqual(types["1"]["role"], "outgoing_normal_damage")
        self.assertEqual(types["2"]["role"], "outgoing_critical_damage")
        self.assertIn("CRIT", types["2"]["text"])
        self.assertEqual(types["3"]["text"], "<color='#81F7F3'>Evade</color>")
        self.assertEqual(types["16"]["text"], "<color='#D43D3D'>Miss</color>")

    def test_primary_coroutine_motion_is_exact(self):
        coroutine = self.result["damageManager"]["coroutine"]
        self.assertEqual(
            [(row["toOffsetY"], row["speedPerSecond"]) for row in coroutine["verticalSegments"]],
            [(5.0, 20.0), (15.0, 120.0), (20.0, 80.0), (35.0, 20.0)],
        )
        self.assertAlmostEqual(coroutine["continuousIdealDurationSeconds"], 1.1458333333333333)
        self.assertIn("WaitForFixedUpdate", coroutine["actualDurationBoundary"])
        self.assertIn("DestroyList", coroutine["completion"])

    def test_prefab_and_dodge_assets_are_retained(self):
        prefab = self.result["prefab"]
        self.assertEqual(prefab["font"]["pathId"], 197)
        self.assertEqual(prefab["fontSize"], 32)
        self.assertEqual(prefab["rectSize"], {"width": 50.0, "height": 20.0})
        dodge = self.result["dodgeAsset"]
        self.assertEqual(dodge["frameSpritePathIds"], [7240, 7744, 7788, 7830, 7788, 7744, 7240])
        self.assertEqual(dodge["englishTextSprite"]["pathId"], 7815)

    def test_generated_output_is_deterministic(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "pass15.json"
            subprocess.run(
                [sys.executable, str(SCRIPT), "--output", str(output)],
                cwd=ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            self.assertEqual(json.loads(output.read_text()), self.result)


if __name__ == "__main__":
    unittest.main()
