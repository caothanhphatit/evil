import hashlib
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
RAW_PATH = ROOT / "reverse-engineering/evidence/original-native-evilctrl-all-methods-api35-v1.json"
ANALYSIS_PATH = ROOT / "reverse-engineering/evidence/original-native-evil-attack-reduction-analysis-v1.json"
SCHEMA_PATH = ROOT / "reverse-engineering/evidence/evil-ai-drop-runtime-schema-android-api35-v1.json"


class OriginalNativeEvilAttackReductionTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw_bytes = RAW_PATH.read_bytes()
        cls.raw = json.loads(cls.raw_bytes)
        cls.analysis = json.loads(ANALYSIS_PATH.read_text())
        cls.schema = json.loads(SCHEMA_PATH.read_text())

    def test_analysis_is_pinned_to_exact_capture(self):
        source = self.analysis["sources"][0]
        self.assertEqual(source["path"], str(RAW_PATH.relative_to(ROOT)))
        self.assertEqual(hashlib.sha256(self.raw_bytes).hexdigest(), source["sha256"])
        self.assertTrue(self.raw["record"]["payload"]["exactBoundaries"])
        self.assertEqual(self.raw["record"]["payload"]["missing"], [])

    def test_formula_and_writer_methods_have_complete_expected_bodies(self):
        methods = {
            row["methodName"]: row
            for row in self.raw["record"]["payload"]["methods"]
        }
        expected = {
            "GetReduceAttackValue": (88, "dc650c97c453fd9f14901245d118681776e250644a1d965da52c63c85e8e76c2"),
            "BuffSetting": (1296, "b3524b95e4d63ea885736ed2bd76011e419ce5e6b8c00bbb69085228302c3df4"),
            "BuffEndSetting": (1144, "b2960694ee33802cd0fcdf3811aee66b8da9ad69e105a57a772c2faf6aaf0e3c"),
            "BuffAllEndSetting": (392, "9670ea1a6fdfae53fac840408d5e0fb176c888773ac8b5dc5d19e8d7cf87978e"),
            "OFEIPNBMNML": (1712, "140634f14de6f7e1f0d676d04b5fe89dca77c7b28829b8f19a74dfd30737313d"),
        }
        for name, (size, digest) in expected.items():
            candidate = methods[name]["candidates"][0]
            code = bytes.fromhex(candidate["codeHex"])
            self.assertFalse(candidate["codeTruncated"], name)
            self.assertEqual(candidate["nativeSizeBytes"], size, name)
            self.assertEqual(len(code), size, name)
            self.assertEqual(hashlib.sha256(code).hexdigest(), digest, name)

    def test_reduction_fields_match_runtime_schema(self):
        classes = self.schema["record"]["payload"]["classes"]
        evil = next(row for row in classes if row["name"] == "EvilCtrl")
        fields = {row["offset"]: (row["name"], row["type"]) for row in evil["fields"]}
        expected = {
            484: ("PHKJDKANJOA", "System.Single"),
            492: ("DBGLOFBIEBL", "System.Single"),
            500: ("BOKEIJGPICA", "System.Single"),
        }
        self.assertEqual({offset: fields[offset] for offset in expected}, expected)
        self.assertEqual(self.analysis["formula"]["fieldOffsets"], list(expected))

    def test_writer_scaling_and_reset_contract_is_explicit(self):
        writers = self.analysis["writers"]
        self.assertEqual(writers[0]["effectType"], 8)
        self.assertEqual(writers[0]["assignment"], "value * 0.01")
        self.assertEqual(writers[1]["effectType"], 55)
        self.assertEqual(writers[1]["assignment"], "value * 0.01")
        self.assertEqual(
            self.analysis["resetBehavior"]["BuffAllEndSetting"]["clearsReductionFields"],
            [484, 492, 500],
        )


if __name__ == "__main__":
    unittest.main()
