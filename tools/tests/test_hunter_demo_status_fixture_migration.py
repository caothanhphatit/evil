import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MIGRATION = ROOT / "infra/db/migrations/0023_hunter_demo_status_fixture.sql"


class HunterDemoStatusFixtureMigrationTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.sql = MIGRATION.read_text(encoding="utf-8")

    def test_seeds_all_eight_demo_hunters(self):
        rows = re.findall(r"^\s*\(([1-8]),\s+\d+::BIGINT,", self.sql, re.MULTILINE)
        self.assertEqual(rows, [str(hunter_id) for hunter_id in range(1, 9)])
        self.assertIn("00000000-0000-4000-8000-00000000a001", self.sql)

    def test_populates_every_nullable_status_projection(self):
        for column in (
            "xp_to_next_level",
            "dps_milli",
            "critical_rate_bps",
            "attack_speed_milli",
            "evasion_rate_bps",
            "awakening_current",
            "awakening_maximum",
            "reincarnation_current",
            "reincarnation_maximum",
            "is_locked",
            "characteristic_release_id",
            "characteristic_id",
            "secret_points",
        ):
            self.assertRegex(self.sql, rf"\b{column}\s*=")

    def test_does_not_claim_runtime_capture_or_seed_runtime_tables(self):
        self.assertIn("not captured original values", self.sql)
        self.assertIn("Keep runtime_evidence/source_* columns nullable", self.sql)
        self.assertNotIn("INSERT INTO player_hunter_runtime_", self.sql)
        self.assertNotRegex(self.sql, r"\bsource_(?:hp|damage|armor|critical|dodge)\s*=")

    def test_advances_the_disposable_demo_seed_version(self):
        self.assertIn("hunter-lab:20260727-full-fixture", self.sql)
        self.assertIn("GREATEST(seed_version, 3)", self.sql)

    def test_does_not_write_columns_missing_from_player_hunter(self):
        hunter_update = self.sql.split("UPDATE player_profile", 1)[0]
        self.assertNotIn("updated_at", hunter_update)


if __name__ == "__main__":
    unittest.main()
