import hashlib
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "reverse-engineering/evidence/original-native-defense-damage-order-v1.json"


class NativeDefenseDamageOrderEvidenceTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = json.loads(EVIDENCE.read_text())
        cls.sources = {
            source["path"]: json.loads((ROOT / source["path"]).read_text())
            for source in cls.evidence["sources"]
        }

    def test_sources_are_checksum_pinned(self):
        for source in self.evidence["sources"]:
            body = (ROOT / source["path"]).read_bytes()
            self.assertEqual(hashlib.sha256(body).hexdigest(), source["sha256"])

    def test_complete_native_method_bodies_are_pinned(self):
        captures = {}
        for source in self.sources.values():
            for method in source["record"]["payload"].get("methods", []):
                captures[(method["className"], method["methodName"])] = method
        for expected in self.evidence["methods"]:
            method = captures[(expected["type"], expected["method"])]
            candidate = method["candidates"][0]
            body = bytes.fromhex(candidate["codeHex"])
            self.assertFalse(candidate["codeTruncated"])
            self.assertEqual(len(body), expected["nativeSizeBytes"])
            self.assertEqual(hashlib.sha256(body).hexdigest(), expected["bodySha256"])

    def test_status_data_defense_offsets_match_schema(self):
        schema_path = next(
            path for path in self.sources
            if path.endswith("status-data-runtime-schema-android-api35-v1.json")
        )
        classes = self.sources[schema_path]["record"]["payload"]["classes"]
        status_data = next(cls for cls in classes if cls["name"] == "StatusData")
        fields = {field["name"]: field for field in status_data["fields"]}
        for key in ("calcArmor", "calcDodge"):
            expected = self.evidence["statusDataBoundary"][key]
            actual = fields[expected["runtimeName"]]
            self.assertEqual(actual["offset"], expected["offset"])
            self.assertEqual(actual["type"], expected["type"])

    def test_instruction_windows_match_exact_bodies(self):
        bodies = {}
        for source in self.sources.values():
            for method in source["record"]["payload"].get("methods", []):
                bodies[f'{method["className"]}.{method["methodName"]}'] = bytes.fromhex(
                    method["candidates"][0]["codeHex"]
                )
        for window in self.evidence["instructionWindows"]:
            actual = bodies[window["method"]][
                window["startOffset"]:window["endOffsetExclusive"]
            ]
            self.assertEqual(actual.hex(), window["hex"])

    def test_no_status_data_accuracy_field_is_claimed(self):
        schema_path = next(
            path for path in self.sources
            if path.endswith("status-data-runtime-schema-android-api35-v1.json")
        )
        classes = self.sources[schema_path]["record"]["payload"]["classes"]
        status_data = next(cls for cls in classes if cls["name"] == "StatusData")
        names = [field["name"].lower() for field in status_data["fields"]]
        self.assertFalse(any("accuracy" in name or "<acc" in name or "<hit" in name for name in names))


if __name__ == "__main__":
    unittest.main()
