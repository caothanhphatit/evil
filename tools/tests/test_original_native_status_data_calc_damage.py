import json
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class OriginalNativeStatusDataCalcDamageTest(unittest.TestCase):
    def test_level_and_revive_producers_are_reproducible(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "analysis.json"
            subprocess.run(
                [
                    "python3",
                    str(ROOT / "tools/analyze-original-native-status-data-calc-damage.py"),
                    "--methods",
                    str(
                        ROOT
                        / "reverse-engineering/evidence/original-native-status-data-level-revive-producers-api35-v1.json"
                    ),
                    "--static-factors",
                    str(
                        ROOT
                        / "reverse-engineering/evidence/original-runtime-status-data-static-factors-api35-v1.json"
                    ),
                    "--calc-damage-producer",
                    str(
                        ROOT
                        / "reverse-engineering/evidence/original-native-status-data-calc-damage-producer-api35-v1.json"
                    ),
                    "--guild-owner-method",
                    str(
                        ROOT
                        / "reverse-engineering/evidence/original-native-status-data-torment-static-owner-api35-v1.json"
                    ),
                    "--guild-schema",
                    str(
                        ROOT
                        / "reverse-engineering/evidence/guild-manager-runtime-schema-api35-v1.json"
                    ),
                    "--output",
                    str(output),
                ],
                check=True,
                cwd=ROOT,
                capture_output=True,
                text=True,
            )
            resolved = json.loads(output.read_text())["resolved"]
            self.assertEqual(
                resolved["formulas"]["CalcLevel"],
                "float32(1.0 + float32(HunterData.level) * float32(0.003))",
            )
            self.assertEqual(
                resolved["formulas"]["CalcRevive"],
                "HunterData.revive < 1 ? 1 : wrapping_i32(HunterData.revive * 3)",
            )
            self.assertEqual(resolved["calcLevelFactor"]["rawHex"], "a69b443b")
            self.assertEqual(
                resolved["tormentGuildLayer"]["formula"],
                "damage *= 1.0 + UserData.mTormentAttackUp + GuildManager.mRankBuffAttack",
            )
            self.assertEqual(resolved["fairyAttackUp"]["sourceField"], "HunterData.fairyIndex")


if __name__ == "__main__":
    unittest.main()
