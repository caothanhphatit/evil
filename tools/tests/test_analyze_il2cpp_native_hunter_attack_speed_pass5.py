import hashlib
import importlib.util
import json
import math
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
ANALYZER_PATH = ROOT / "tools/analyze-il2cpp-native-hunter-attack-speed-pass5.py"
EVIDENCE_PATH = ROOT / "reverse-engineering/evidence/original-native-hunter-attack-speed-producer-chain-v2.json"

SPEC = importlib.util.spec_from_file_location("analyze_hunter_attack_speed_pass5", ANALYZER_PATH)
ANALYZER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(ANALYZER)


class HunterAttackSpeedPass5Test(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = json.loads(EVIDENCE_PATH.read_text())

    def test_evidence_rebuild_is_deterministic(self):
        rebuilt = ANALYZER.build(
            ANALYZER.DEFAULT_CAPTURE,
            ANALYZER.DEFAULT_SCAN,
            ANALYZER.DEFAULT_RESOLUTIONS,
            ANALYZER.DEFAULT_STATUS_SCHEMA,
            ANALYZER.DEFAULT_HUNTER_SCHEMA,
            ANALYZER.DEFAULT_MANAGER_SCHEMA,
            ANALYZER.DEFAULT_USER_SCHEMA,
        )
        self.assertEqual(rebuilt, self.evidence)

    def test_sources_are_checksum_pinned(self):
        for source in self.evidence["sources"]:
            path = ROOT / source["path"]
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), source["sha256"])

    def test_status_producer_equations(self):
        producer = self.evidence["statusDataProducer"]
        self.assertIn("mGuildAttackSpeedUp", producer["attackSpeedEquation"])
        self.assertEqual(producer["gupArrayIndex"], 7)
        self.assertEqual(producer["calcAttackSpeedEquation"], "CalcAttackSpeed = max(0.25, AttackSpeed / denominator)")
        for vector in producer["goldenVectors"]:
            values = vector["input"]
            if "attackSpeed" in vector:
                actual = ANALYZER.attack_speed(*values)
                self.assertTrue(math.isclose(actual, vector["attackSpeed"], rel_tol=1e-6))
            else:
                actual = ANALYZER.calc_attack_speed(*values)
                self.assertTrue(math.isclose(actual, vector["calcAttackSpeed"], rel_tol=1e-6))

    def test_bce_writers_and_dan_fail_closed(self):
        chain = self.evidence["furyAndBceChain"]
        self.assertIn("secondArgument * 0.01", chain["buffSettingTypeZero"])
        self.assertIn("resets BCEBGLKCDHN = 1.0", chain["buffEndTypeZero"])
        self.assertEqual(self.evidence["danFactor"]["directManagedWriters"], [])
        self.assertEqual(self.evidence["danFactor"]["classMethodsScanned"], 391)

    def test_fixed_update_is_the_direct_timer_reader(self):
        timer = self.evidence["attackDelayFsm"]
        self.assertEqual(timer["readerMethod"], "HunterCtrl.FixedUpdate()")
        self.assertIn("subtract UnityEngine.Time.deltaTime", timer["readerOperation"])
        self.assertEqual(timer["writerMethods"], ["HuntingAttackAction", "CGAHEABLJMF", "NBOMDKMCGND"])

    def test_method_identity_and_boundaries(self):
        methods = {(row["type"], row["method"], row["parameterCount"]): row for row in self.evidence["methods"]}
        self.assertEqual(methods[("StatusData", "COJNMPDBOOO", 0)]["moduleOffset"], "0x2d5e1f8")
        self.assertEqual(methods[("HunterCtrl", "FixedUpdate", 0)]["moduleOffset"], "0x340fcf8")
        self.assertEqual(methods[("HunterCtrl", "BuffSetting", 4)]["nativeSizeBytes"], 12656)


if __name__ == "__main__":
    unittest.main()
