import hashlib
import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GENERATOR = ROOT / "tools/generate-core-game-sql.py"
OUTPUT = ROOT / "infra/db/core_game/001_core_game_catalog.sql"


class CoreGameSqlTest(unittest.TestCase):
    def test_bundle_is_fresh_and_has_guard_counts(self):
        spec = importlib.util.spec_from_file_location("core_game_sql", GENERATOR)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.main()
        text = OUTPUT.read_text(encoding="utf-8")
        self.assertIn("core_game", text)
        self.assertIn("core-game catalog count mismatch", text)
        self.assertIn("<> 195", text)

    def test_manifest_sources_match_catalog_bytes(self):
        sql = OUTPUT.read_text(encoding="utf-8")
        for name in ("monster-runtime-catalog.json", "monster-material-market-catalog.json", "gear-catalog.json", "experience-runtime-catalog.json"):
            digest = hashlib.sha256((ROOT / "packages/content/releases/evil-hunter-1.411" / name).read_bytes()).hexdigest()
            self.assertIn(digest, sql)


if __name__ == "__main__":
    unittest.main()
