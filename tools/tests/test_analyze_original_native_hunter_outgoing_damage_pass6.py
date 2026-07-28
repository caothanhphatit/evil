import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class Pass6Test(unittest.TestCase):
    def test_proven_native_anchors(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "evidence.json"
            subprocess.run([sys.executable, str(ROOT / "tools/analyze-original-native-hunter-outgoing-damage-pass6.py"), "--output", str(output)], check=True)
            data = json.loads(output.read_text())
        self.assertTrue(all(data["invariants"].values()))
        self.assertEqual(data["proven"]["criticalMultiplier"]["base"], 1.75)
        self.assertIn("FCVTZS", data["proven"]["rounding"])
        self.assertEqual(data["runtimeCompatibility"], "evidence-only-disconnected")


if __name__ == "__main__":
    unittest.main()
