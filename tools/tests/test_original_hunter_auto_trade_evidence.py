import hashlib
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "reverse-engineering/evidence/original-native-hunter-auto-trade-methods-api35-v1.json"
DECRYPTED = ROOT / "reverse-engineering/evidence/original-native-hunter-auto-trade-decrypted-api35-v1.json"


class OriginalHunterAutoTradeEvidenceTest(unittest.TestCase):
    def test_captured_bodies_are_self_consistent(self):
        payload = json.loads(EVIDENCE.read_text())
        methods = payload["methods"]
        self.assertEqual(len(methods), 7)
        tokens = [method["token"] for method in methods]
        self.assertEqual(len(tokens), len(set(tokens)))
        for method in methods:
            body = bytes.fromhex(method["codeHex"])
            self.assertEqual(len(body), method["size"])
            self.assertEqual(hashlib.sha256(body).hexdigest(), method["sha256"])

    def test_capture_is_explicitly_evidence_only(self):
        payload = json.loads(EVIDENCE.read_text())
        self.assertEqual(payload["capture"]["deviceAbi"], "arm64-v8a")
        self.assertIn("no account values", payload["capture"]["action"])
        self.assertTrue(payload["limitations"])

    def test_external_capture_matches_decrypted_method_identity(self):
        external = json.loads(EVIDENCE.read_text())["methods"]
        decrypted = json.loads(DECRYPTED.read_text())["record"]["payload"]["methods"]
        external_ids = {(item["className"], item["methodName"], item["token"]) for item in external}
        decrypted_ids = {(item["className"], item["methodName"], item["token"]) for item in decrypted}
        self.assertEqual(external_ids, decrypted_ids)
        for item in decrypted:
            candidate = item["candidates"][0]
            matching = next(x for x in external if x["token"] == item["token"])
            self.assertEqual(candidate["moduleOffset"], matching["moduleOffset"])
            self.assertEqual(candidate["nativeSizeBytes"], matching["size"])


if __name__ == "__main__":
    unittest.main()
