import hashlib
import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
ANALYZER_PATH = ROOT / "tools/analyze-il2cpp-native-hunter-attack-speed.py"
EVIDENCE_PATH = ROOT / "reverse-engineering/evidence/original-native-hunter-attack-speed-chain-v1.json"

SPEC = importlib.util.spec_from_file_location("analyze_hunter_attack_speed", ANALYZER_PATH)
ANALYZER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(ANALYZER)


class HunterAttackSpeedAnalysisTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = json.loads(EVIDENCE_PATH.read_text())

    def test_evidence_rebuild_is_deterministic(self):
        rebuilt = ANALYZER.build(
            ANALYZER.DEFAULT_CAPTURE,
            ANALYZER.DEFAULT_TARGET,
            ANALYZER.DEFAULT_REFRESH,
            ANALYZER.DEFAULT_CADENCE,
            ANALYZER.DEFAULT_HUNTER_SCHEMA,
            ANALYZER.DEFAULT_STATUS_SCHEMA,
        )
        self.assertEqual(rebuilt, self.evidence)

    def test_sources_are_checksum_pinned(self):
        for source in self.evidence["sources"]:
            path = ROOT / source["path"]
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), source["sha256"])

    def test_unresolved_boundary_target_is_exactly_identified(self):
        target = self.evidence["resolvedBoundaryTarget"]
        self.assertEqual(target["moduleOffset"], "0x33f79bc")
        self.assertEqual((target["type"], target["method"]), ("HunterCtrl", "InitHunterHpBar"))
        self.assertEqual(target["token"], "0x06005C11")
        self.assertEqual(target["parameterTypes"], [])
        self.assertEqual(target["returnType"], "System.Void")

    def test_exact_cadence_reader_writer_chain(self):
        chain = self.evidence["fieldChain"]
        inputs = {(row["name"], row["offset"], row["access"]) for row in chain["cadenceInputs"]}
        self.assertEqual(
            inputs,
            {
                ("DANCPPLMKIK", 0x3D8, "read"),
                ("BCEBGLKCDHN", 0x6AC, "read and decode"),
            },
        )
        self.assertEqual(chain["cadenceOutput"]["name"], "AttackAniTime")
        self.assertEqual(chain["cadenceOutput"]["offset"], 0x1AC)
        self.assertEqual(
            chain["equation"],
            "composite = DANCPPLMKIK * decode(BCEBGLKCDHN); AttackAniTime = composite > 1.0 ? 0.333 / composite : 0.7",
        )

    def test_calc_attack_speed_is_copied_as_complete_obscured_float(self):
        copy = self.evidence["fieldChain"]["statusCopy"]
        self.assertEqual(copy["source"], {"owner": "StatusData", "name": "CalcAttackSpeed", "offset": 0x88, "type": "ObscuredFloat"})
        self.assertEqual(copy["destination"], {"owner": "HunterCtrl", "name": "mAttackDelay", "offset": 0x194, "type": "ObscuredFloat"})
        self.assertEqual(copy["copyBytes"], 20)

    def test_negative_writer_findings_remain_scoped(self):
        findings = {row["method"]: row for row in self.evidence["negativeFindings"]}
        self.assertIn("0x698", findings["HunterCtrl.SettingProperty"]["finding"])
        self.assertIn("not BCEBGLKCDHN", findings["HunterCtrl.SettingProperty"]["finding"])
        self.assertIn("does not reference", findings["HunterCtrl.RefreshAnimation"]["finding"])
        self.assertTrue(all(row["scope"] == "captured exact method body" for row in findings.values()))


if __name__ == "__main__":
    unittest.main()
