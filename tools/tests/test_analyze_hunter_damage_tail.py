import hashlib
import json
import math
import struct
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
ANALYZER = ROOT / "tools/analyze-hunter-damage-tail.py"
EVIDENCE = ROOT / "reverse-engineering/evidence/original-native-hunter-damage-tail-v3.json"


class AnalyzeHunterDamageTailTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = json.loads(EVIDENCE.read_text())

    def test_analyzer_is_deterministic(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "analysis.json"
            subprocess.run(
                ["python3", str(ANALYZER), "--output", str(output)],
                cwd=ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            self.assertEqual(json.loads(output.read_text()), self.evidence)

    def test_sources_are_checksum_pinned(self):
        for source in self.evidence["sources"]:
            body = (ROOT / source["path"]).read_bytes()
            self.assertEqual(hashlib.sha256(body).hexdigest(), source["sha256"])

    def test_actk_obscured_float_decode_vectors(self):
        def decode(raw_hex):
            raw = bytes.fromhex(raw_hex)
            key = int.from_bytes(raw[0:4], "little")
            hidden = bytearray(raw[4:8])
            hidden[1], hidden[2] = hidden[2], hidden[1]
            bits = key ^ int.from_bytes(hidden, "little")
            return struct.unpack("<f", bits.to_bytes(4, "little"))[0]

        for raw_hex in (
            "fba7067efb46a741000000000000000000010000",
            "01c30e4a014ec375000000000000000000010000",
        ):
            self.assertEqual(decode(raw_hex), 0.75)

    def test_pre_armor_stages_are_complete_and_ordered(self):
        accumulator = self.evidence["preArmorAccumulator"]
        stages = accumulator["stages"]
        self.assertEqual(accumulator["stageCount"], 32)
        self.assertEqual(len(stages), 32)
        self.assertEqual([stage["order"] for stage in stages], list(range(1, 33)))
        self.assertEqual(len({stage["windowSha256"] for stage in stages}), 32)
        self.assertEqual(accumulator["nextOperation"], "A33 = A32 - armorScratch")

    def test_modifier_family_vectors(self):
        def trunc(value):
            return math.trunc(value)

        for row in self.evidence["goldenVectors"]["modifierFamilies"]:
            accumulator = row["accumulator"]
            family = row["family"]
            if family == "proportional_add":
                actual = trunc(float(accumulator) + row["factor"] * float(accumulator))
            elif family == "proportional_subtract":
                actual = trunc(float(accumulator) - row["factor"] * float(accumulator))
            elif family == "summed_proportional_subtract":
                actual = trunc(float(accumulator) - (row["factor1"] + row["factor2"]) * float(accumulator))
            elif family == "negative_percent_point_add":
                actual = trunc(float(accumulator) + row["rawValue"] * row["count"] * -0.01)
            elif family == "negative_basis_point_add":
                actual = trunc(float(accumulator) + row["rawValue"] * row["count"] * -0.0001)
            elif family == "fixed_scale":
                actual = trunc(float(accumulator) * row["factor"])
            elif family == "one_minus_percent_scale":
                actual = trunc(float(accumulator) * (1.0 + row["rawPercent"] * -0.01))
            elif family == "direct_product_subtract":
                actual = trunc(float(accumulator) - row["factor"] * float(accumulator))
            elif family == "percent_product_subtract":
                actual = trunc(float(accumulator) - row["composite"] * float(accumulator) * 0.01)
            else:
                self.fail(f"unhandled family {family}")
            self.assertEqual(actual, row["expected"])

    def test_selected_final_factor_writer_is_resolved(self):
        factor = self.evidence["selectedFinalFactor"]
        self.assertEqual(factor["owner"], "ConstantData")
        self.assertEqual(factor["field"], "DEFALUT_DAMAGE_DECREASE_VALUE")
        self.assertEqual(factor["capturedValue"], 0.75)
        self.assertEqual(factor["constructorValue"], 0.75)
        self.assertIn("ConstantData..cctor", factor["writer"])

    def test_armor_selector_boundary_vectors(self):
        bands = self.evidence["armorSelector"]["bands"]

        def select(feel, now_feel):
            thresholds = (0.8, 0.6, 0.4, 0.2)
            for index, threshold in enumerate(thresholds):
                if now_feel >= feel * threshold:
                    return bands[index]["factor"]
            return bands[4]["factor"]

        for row in self.evidence["goldenVectors"]["armorSelector"]:
            self.assertTrue(math.isclose(select(row["feel"], row["nowFeel"]), row["factor"], rel_tol=1e-6))

    def test_shield_routing_vectors(self):
        for row in self.evidence["goldenVectors"]["shieldRouting"]:
            if row["currentShield"] < row["forwardedDamage"]:
                shield = 0
                hp_damage = row["forwardedDamage"] - row["currentShield"]
            else:
                shield = row["currentShield"] - row["forwardedDamage"]
                hp_damage = 0
            self.assertEqual(shield, row["expectedShield"])
            self.assertEqual(hp_damage, row["expectedHpDamage"])


if __name__ == "__main__":
    unittest.main()
