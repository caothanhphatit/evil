import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "tools/analyze-original-native-dodge-consumer-pass18.py"


def load_analyzer():
    spec = importlib.util.spec_from_file_location("dodge_pass18", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class DodgeConsumerPass18Test(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.analyzer = load_analyzer()
        cls.result = cls.analyzer.build()

    def test_normal_consumer_and_producer_are_exact(self):
        normal = self.result["normalHunter"]
        self.assertIn("wrapping_i32", normal["formula"])
        self.assertEqual(normal["comparison"], "signed exclusive less-than")
        self.assertEqual(normal["presentationOnSuccess"]["identity"], "Evade")
        self.assertIn("GUP_Property[8]", self.result["producer"]["rawDodge"])

    def test_callers_abort_damage_or_debuff_path(self):
        callers = self.result["normalHunter"]["callers"]
        self.assertIn("exits the damage routine", callers[0])
        self.assertIn("value-51", callers[1])
        self.assertEqual(self.result["effectType5Writer"]["publicName"], None)

    def test_mode_variants_preserve_distinct_rng_and_subtractors(self):
        modes = self.result["modeVariants"]
        self.assertIn("primaryRoll[0,101)", modes["worldBoss"]["formula"])
        self.assertIn("opponent-owned subtraction", modes["guildBattle"]["formula"])
        self.assertIn("likely dispatch remains unresolved", modes["pvp"]["callerStatus"])

    def test_output_is_deterministic(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "pass18.json"
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
