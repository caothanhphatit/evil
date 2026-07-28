import hashlib
import json
import math
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "reverse-engineering/evidence/original-native-hunter-damage-tail-v1.json"


class NativeHunterDamageTailEvidenceTest(unittest.TestCase):
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

    def test_method_bodies_and_instruction_windows_are_pinned(self):
        bodies = {}
        records = {}
        for source in self.sources.values():
            for method in source["record"]["payload"].get("methods", []):
                key = f'{method["className"]}.{method["methodName"]}'
                candidate = method["candidates"][0]
                bodies[key] = bytes.fromhex(candidate["codeHex"])
                records[key] = (method, candidate)
        for expected in self.evidence["methods"]:
            key = f'{expected["type"]}.{expected["method"]}'
            body = bodies[key]
            method, candidate = records[key]
            self.assertFalse(candidate["codeTruncated"])
            self.assertEqual(f'0x{method["token"]:08X}', expected["token"])
            self.assertEqual(candidate["moduleOffset"], expected["moduleOffset"])
            self.assertEqual(len(body), expected["nativeSizeBytes"])
            self.assertEqual(hashlib.sha256(body).hexdigest(), expected["bodySha256"])
        for window in self.evidence["instructionWindows"]:
            actual = bodies[window["method"]][window["startOffset"]:window["endOffsetExclusive"]]
            self.assertEqual(actual.hex(), window["hex"])

    def test_runtime_schema_fields_match_offsets(self):
        classes = []
        for source in self.sources.values():
            classes.extend(source["record"]["payload"].get("classes", []))
        by_name = {cls["name"]: cls for cls in classes}
        status_fields = {field["name"]: field for field in by_name["StatusData"]["fields"]}
        self.assertEqual(status_fields["<CalcDodge>k__BackingField"]["offset"], 192)
        self.assertEqual(status_fields["<GearProperty>k__BackingField"]["offset"], 528)
        self.assertEqual(status_fields["<GearSetProperty>k__BackingField"]["offset"], 536)
        self.assertEqual(status_fields["<GearSetPropertyValue>k__BackingField"]["offset"], 544)
        hunter_fields = {field["name"]: field for field in by_name["HunterData"]["fields"]}
        self.assertEqual(hunter_fields["<nowHp>k__BackingField"]["offset"], 360)
        evil_fields = {field["name"]: field for field in by_name["EvilCtrl"]["fields"]}
        self.assertEqual(evil_fields["OCLFGGEJKMI"]["offset"], 488)

    def test_common_tail_golden_vectors(self):
        for row in self.evidence["goldenVectors"]["commonTail"]:
            post_armor = row["accumulator"] - row["armorScratch"]
            forwarded = 1 if post_armor <= 0 else math.trunc(
                post_armor * row["selectedFinalFactor"]
            )
            self.assertEqual(post_armor, row["postArmor"])
            self.assertEqual(forwarded, row["forwardedDamage"])

    def test_default_hp_and_effect_gate_vectors(self):
        for row in self.evidence["goldenVectors"]["defaultHpMutation"]:
            actual = max(row["nowHp"] - row["forwardedDamage"], 0)
            self.assertEqual(actual, row["expectedNowHp"])
        for row in self.evidence["goldenVectors"]["effect54Gate"]:
            enabled = row["value"] >= 1
            proc = enabled and row["roll"] < row["value"]
            self.assertEqual(enabled, row["enabled"])
            self.assertEqual(proc, row["proc"])


if __name__ == "__main__":
    unittest.main()
