import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "tools/analyze-original-native-hit-miss-pass12.py"


def load_analyzer():
    spec = importlib.util.spec_from_file_location("hit_miss_pass12", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class HitMissPass12Test(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.analyzer = load_analyzer()
        cls.result = cls.analyzer.build()

    def test_hunter_rng_is_critical_not_accuracy(self):
        rng = self.result["hunterAttacksEvil"]["getDamageIntegerRng"]
        self.assertEqual(rng["range"], "[0,100)")
        self.assertEqual(rng["comparison"], "roll < threshold")
        self.assertIn("CalcCritical", rng["threshold"])
        self.assertIn("critical selection", rng["identity"])
        self.assertEqual(
            self.result["hunterAttacksEvil"]["schemaBoundary"],
            {"statusAccuracyFields": [], "evilEvasionFields": []},
        )

    def test_evil_gate_precedes_damage_without_calc_dodge_link(self):
        incoming = self.result["evilAttacksHunter"]
        self.assertEqual(incoming["preDamageGate"]["range"], "[0,100)")
        self.assertEqual(incoming["preDamageGate"]["comparison"], "roll < field")
        self.assertIn("before HunterCtrl.Damaged", incoming["preDamageGate"]["procOrdering"])
        self.assertFalse(incoming["calcDodge"]["directReadInCapturedChain"])
        self.assertEqual(incoming["calcDodge"]["globalConsumerStatus"], "unresolved")

    def test_generated_output_is_deterministic(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "pass12.json"
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
