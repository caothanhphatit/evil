import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
GENERATOR_PATH = ROOT / "tools/generate-monster-material-market-catalog.py"
CATALOG_PATH = ROOT / "packages/content/releases/evil-hunter-1.411/monster-material-market-catalog.json"


def load_generator():
    spec = importlib.util.spec_from_file_location("monster_material_market_catalog", GENERATOR_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class MonsterMaterialMarketCatalogTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.generator = load_generator()
        cls.catalog = json.loads(CATALOG_PATH.read_text(encoding="utf-8"))

    def test_committed_catalog_matches_generator(self):
        generated = self.generator.generate(
            self.generator.DEFAULT_MONSTERS,
            self.generator.DEFAULT_ECONOMY,
            self.generator.DEFAULT_BUILDINGS,
        )
        self.assertEqual(self.catalog, generated)

    def test_every_monster_droppable_material_is_listed_by_trading_post(self):
        materials = self.catalog["materials"]
        self.assertEqual(len(materials), 179)
        self.assertEqual(self.catalog["summary"]["unlistedDroppableMaterials"], [])
        self.assertTrue(
            all(row["tradingPost"]["townPaysHunterGoldPerUnit"] is not None for row in materials)
        )

    def test_drop_slots_reference_normalized_materials(self):
        material_ids = {row["materialId"] for row in self.catalog["materials"]}
        self.assertEqual(len(self.catalog["monsterDropSlots"]), 1617)
        self.assertTrue(
            all(row["materialId"] in material_ids for row in self.catalog["monsterDropSlots"])
        )

    def test_consumers_keep_exact_quantities_and_unresolved_conditions_explicit(self):
        material_ids = {row["materialId"] for row in self.catalog["materials"]}
        self.assertTrue(self.catalog["recipeMaterialInputs"])
        self.assertTrue(self.catalog["buildingMaterialCosts"])
        self.assertTrue(
            all(row["quantity"] > 0 for row in self.catalog["recipeMaterialInputs"])
        )
        self.assertTrue(
            all(row["materialId"] in material_ids for row in self.catalog["recipeMaterialInputs"])
        )
        self.assertTrue(
            all(row["requiredEvidence"] for row in self.catalog["unresolvedRecipeConditions"])
        )


if __name__ == "__main__":
    unittest.main()
