import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GENERATOR = ROOT / "tools/generate-rebuild-weapon-core.py"
CATALOG = ROOT / "packages/content/releases/evil-hunter-rebuild-v1/weapon-core-catalog.json"
SQL = ROOT / "infra/db/core_game/002_rebuild_weapon_core.sql"


def load_generator():
    spec = importlib.util.spec_from_file_location("rebuild_weapon_core", GENERATOR)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class RebuildWeaponCoreTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.generator = load_generator()
        cls.generator.main()
        cls.catalog = json.loads(CATALOG.read_text(encoding="utf-8"))

    def test_catalog_shape_and_localization(self):
        self.assertEqual(len(self.catalog["difficulties"]), 8)
        self.assertEqual(len(self.catalog["rarities"]), 4)
        self.assertEqual(len(self.catalog["weapons"]), 40)
        self.assertEqual(len(self.catalog["modifiers"]), 126)
        self.assertEqual(len([row for row in self.catalog["modifiers"] if row["origin"] == "package"]), 125)
        self.assertEqual(len(self.catalog["affixTiers"]), 160)
        self.assertEqual(len(self.catalog["weaponModifierPool"]), 20)
        self.assertEqual(len(self.catalog["virtues"]), 5)
        self.assertEqual(len(self.catalog["collectionSets"]), 61)
        self.assertEqual({row["classId"] for row in self.catalog["weapons"]}, set(self.generator.CLASS_CONFIG))
        for weapon in self.catalog["weapons"]:
            self.assertEqual(set(weapon["localization"]), {"en", "vi"})

    def test_progression_and_rarity_contract(self):
        self.assertEqual(
            [row["basePowerMin"] for row in self.catalog["difficulties"]],
            [60, 96, 154, 246, 393, 629, 1007, 1611],
        )
        self.assertEqual(self.catalog["difficulties"][-1]["basePowerMax"], 2577)
        self.assertEqual(
            [(row["prefixSlots"], row["suffixSlots"]) for row in self.catalog["rarities"]],
            [(0, 0), (1, 1), (2, 2), (3, 3)],
        )

    def test_unresolved_evidence_stays_fail_closed(self):
        self.assertTrue(all(row["effectState"] == "unresolved" for row in self.catalog["collectionSets"]))
        active_pool = [row for row in self.catalog["weaponModifierPool"] if row["active"]]
        self.assertEqual(len([row for row in active_pool if row["slot"] == "prefix"]), 12)
        self.assertEqual(len([row for row in active_pool if row["slot"] == "suffix"]), 8)
        transforms = {row["sourceId"]: row for row in self.catalog["modifiers"] if row["sourceId"] in (48, 49)}
        self.assertEqual(set(transforms), {48, 49})
        self.assertTrue(all(row["generationState"] == "unresolved" for row in transforms.values()))

    def test_every_pool_affix_has_eight_ordered_tiers(self):
        tiers_by_modifier = {}
        for tier in self.catalog["affixTiers"]:
            tiers_by_modifier.setdefault(tier["modifierId"], []).append(tier)
        self.assertEqual(set(tiers_by_modifier), {row["modifierId"] for row in self.catalog["weaponModifierPool"]})
        for tiers in tiers_by_modifier.values():
            self.assertEqual([row["difficulty"] for row in tiers], list(range(1, 9)))
            self.assertTrue(all(row["minimumValue"] <= row["maximumValue"] for row in tiers))
        flat = tiers_by_modifier["rebuild:flat_attack"]
        self.assertEqual((flat[0]["minimumValue"], flat[0]["maximumValue"]), (8, 19))
        self.assertEqual((flat[-1]["minimumValue"], flat[-1]["maximumValue"]), (194, 515))

    def test_generated_sql_has_count_guard(self):
        sql = SQL.read_text(encoding="utf-8")
        self.assertIn("rebuild weapon core count mismatch", sql)
        for count in (8, 4, 40, 80, 126, 160, 20, 5, 61):
            self.assertIn(f"<> {count}", sql)


if __name__ == "__main__":
    unittest.main()
