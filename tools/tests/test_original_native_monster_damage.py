import json
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class OriginalNativeMonsterDamageTest(unittest.TestCase):
    def test_common_damage_arithmetic_is_reproducible(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "monster-damage.json"
            subprocess.run(
                [
                    "python3",
                    str(ROOT / "tools/analyze-original-native-monster-damage.py"),
                    "--capture",
                    str(ROOT / "reverse-engineering/evidence/original-native-combat-formula-exact-boundaries-api35-v1.json"),
                    "--evil-schema",
                    str(ROOT / "reverse-engineering/evidence/evil-data-runtime-schema-api35-v1.json"),
                    "--hunter-schema",
                    str(ROOT / "reverse-engineering/evidence/hunter-info-runtime-schema-android-api35-v1.json"),
                    "--status-schema",
                    str(ROOT / "reverse-engineering/evidence/status-data-runtime-schema-android-api35-v1.json"),
                    "--static-factors",
                    str(ROOT / "reverse-engineering/evidence/original-runtime-status-data-static-factors-api35-v1.json"),
                    "--output",
                    str(output),
                ],
                check=True,
                cwd=ROOT,
                capture_output=True,
                text=True,
            )
            payload = json.loads(output.read_text())
            self.assertEqual(payload["method"]["nativeSizeBytes"], 4736)
            self.assertEqual(
                payload["resolvedCommonArithmetic"]["minimumDamage"],
                "max_by_branch(damage - effectiveArmor, 1)",
            )
            self.assertEqual(
                payload["resolvedSelectors"]["preArmorBonusSources"][0],
                "StatusData.GearProperty[51][0] under the recovered <=50% monster HP gate",
            )


if __name__ == "__main__":
    unittest.main()
