import hashlib
import json
import math
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "reverse-engineering/evidence/original-native-combat-cadence-stat-chain-v1.json"


class NativeCombatCadenceEvidenceTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = json.loads(EVIDENCE.read_text())

    def test_sources_are_checksum_pinned(self):
        for source in self.evidence["sources"]:
            body = (ROOT / source["path"]).read_bytes()
            self.assertEqual(hashlib.sha256(body).hexdigest(), source["sha256"])

    def test_native_bodies_match_exact_boundary_capture(self):
        captures = {}
        for source in self.evidence["sources"]:
            payload = json.loads((ROOT / source["path"]).read_text())["record"]["payload"]
            if "methods" not in payload:
                continue
            for method in payload["methods"]:
                captures[(method["className"], method["methodName"])] = method
        for expected in self.evidence["methods"]:
            method = captures[(expected["type"], expected["method"])]
            body = bytes.fromhex(method["candidates"][0]["codeHex"])
            self.assertEqual(len(body), expected["nativeSizeBytes"])
            self.assertEqual(hashlib.sha256(body).hexdigest(), expected["bodySha256"])

    def test_cadence_golden_vectors(self):
        vectors = self.evidence["goldenVectors"]
        for row in vectors["evilUnitAttackDelay"]:
            actual = 0.08 * max(row["factor"], 1.0)
            self.assertTrue(math.isclose(actual, row["expected"], rel_tol=1e-6))
        for row in vectors["hunterAttackAnimationTime"]:
            actual = 0.333 / row["composite"] if row["composite"] > 1.0 else 0.7
            self.assertTrue(math.isclose(actual, row["expected"], rel_tol=1e-6))

    def test_status_data_field_offsets_match_runtime_schema(self):
        source = next(
            source for source in self.evidence["sources"]
            if source["path"].endswith("status-data-runtime-schema-android-api35-v1.json")
        )
        payload = json.loads((ROOT / source["path"]).read_text())["record"]["payload"]
        status_data = next(cls for cls in payload["classes"] if cls["name"] == "StatusData")
        runtime_fields = {field["name"]: field for field in status_data["fields"]}
        for expected in self.evidence["statusDataFields"]:
            field = runtime_fields[expected["runtimeName"]]
            self.assertEqual(field["offset"], expected["offset"])
            self.assertTrue(field["type"].endswith(expected["type"]))

    def test_critical_threshold_core_vectors(self):
        for row in self.evidence["goldenVectors"]["criticalThresholdCore"]:
            bonus = row["bonus"] if row["bonusEnabled"] else 0
            threshold = min(100, row["calcCritical"] + bonus)
            self.assertEqual(threshold, row["threshold"])
            self.assertEqual(row["roll"] < threshold, row["critical"])


if __name__ == "__main__":
    unittest.main()
