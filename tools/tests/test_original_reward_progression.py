from __future__ import annotations

import hashlib
import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


ANALYZER = load_module("original_reward_progression", ROOT / "tools/analyze-original-reward-progression.py")
GENERATOR = load_module("experience_runtime_catalog", ROOT / "tools/generate-experience-runtime-catalog.py")


class OriginalRewardProgressionTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.catalog = GENERATOR.generate(GENERATOR.DEFAULT_TABLES, GENERATOR.DEFAULT_SCHEMA, GENERATOR.DEFAULT_HELPERS)
        cls.evidence = ANALYZER.build(
            ANALYZER.DEFAULT_METHODS,
            ANALYZER.DEFAULT_HELPERS,
            ANALYZER.DEFAULT_MATERIAL,
            ANALYZER.DEFAULT_SCHEMA,
            ANALYZER.DEFAULT_CATALOG,
        )

    def test_catalog_is_deterministic_and_source_hashes_are_pinned(self) -> None:
        checked_in = json.loads(GENERATOR.DEFAULT_OUTPUT.read_text())
        self.assertEqual(self.catalog, checked_in)
        for source in self.catalog["sources"]:
            body = (ROOT / source["path"]).read_bytes()
            self.assertEqual(hashlib.sha256(body).hexdigest(), source["sha256"])

    def test_get_need_exp_uses_revive_and_next_level_row(self) -> None:
        lookup = self.catalog["lookup"]
        self.assertEqual(lookup["inputs"], ["revive", "currentLevel"])
        self.assertEqual(lookup["rowIndexFormula"], "currentLevel + 1")
        self.assertEqual(lookup["columnFormula"], "experienceByDifficulty[revive] for revive 0..5")
        self.assertEqual(len(self.catalog["rows"]), 100)
        self.assertEqual(self.catalog["rows"][1]["experienceByDifficulty"], [240, 960, 5760, 46080, 460800, 5529600])

    def test_level_carry_strict_threshold_and_cap(self) -> None:
        simulate = ANALYZER.simulate_plus_exp
        need = lambda _: 100
        self.assertEqual(simulate(4, 20, 79, 10, need), (4, 99))
        self.assertEqual(simulate(4, 20, 80, 10, need), (4, 100))
        self.assertEqual(simulate(4, 20, 81, 10, need), (5, 1))
        self.assertEqual(simulate(4, 20, 281, 10, need), (7, 1))
        self.assertEqual(simulate(10, 77, 999, 10, need), (10, 77))

    def test_material_roll_core_is_exact_and_modifier_chain_fails_closed(self) -> None:
        material = self.evidence["recoveredExactFacts"]["ordinaryMaterialRoll"]
        self.assertEqual(material["loopOrder"], "ascending array slot")
        self.assertEqual(material["loopBound"], "materialIndices.length")
        self.assertEqual(material["roll"], "UnityEngine.Random.Range(1, 10001)")
        self.assertEqual(material["baseThreshold"], "materialPercentValues[slot] * 10")
        self.assertEqual(material["grantComparison"], "effectiveThreshold >= roll")
        self.assertTrue(any("Unique-level" in row for row in self.evidence["unresolved"]))

    def test_normalized_evidence_is_deterministic(self) -> None:
        checked_in = json.loads(ANALYZER.DEFAULT_OUTPUT.read_text())
        self.assertEqual(self.evidence, checked_in)


if __name__ == "__main__":
    unittest.main()
