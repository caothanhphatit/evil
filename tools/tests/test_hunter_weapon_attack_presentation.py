import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "tools/extract-hunter-weapon-attack-presentation.py"
SPEC = importlib.util.spec_from_file_location("hunter_weapon_attack_presentation", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class HunterWeaponAttackPresentationTest(unittest.TestCase):
    def test_packaged_weapon_slots_and_directional_clips(self) -> None:
        evidence = MODULE.build(
            type(
                "Args",
                (),
                {
                    "hunter": MODULE.DEFAULT_HUNTER,
                    "monster": MODULE.DEFAULT_MONSTER,
                    "gear": MODULE.DEFAULT_GEAR,
                    "schema": MODULE.DEFAULT_SCHEMA,
                    "native": MODULE.DEFAULT_NATIVE,
                },
            )()
        )

        families = {row["family"]: row for row in evidence["hunterSkeleton"]["weaponFamilies"]}
        self.assertEqual(families["h1"]["attachment"], "sword")
        self.assertEqual(families["h2"]["attachment"], "hammer")
        self.assertEqual(families["h3"]["attachment"], "bow")
        self.assertEqual(families["h4"]["attachment"], "wand")
        self.assertEqual(families["h5"]["attachment"], "spear")
        self.assertTrue(all(row["allUseExpectedSlotAndAttachment"] for row in families.values()))

        attacks = {
            row["name"]: row for row in evidence["hunterSkeleton"]["basicAttackAnimations"]
        }
        self.assertEqual(attacks["h3_hit"]["weaponSlotInitialColors"]["h3"], "ffffffff")
        self.assertEqual(attacks["h3_hit"]["weaponSlotInitialColors"]["h2"], "ffffff00")
        self.assertEqual(attacks["h3_hit"]["durationSeconds"], 0.3333)

        monster = evidence["monsterSkeleton"]["directionalAttachmentNames"]
        self.assertEqual(monster["atk"]["body"], "body")
        self.assertEqual(monster["atk_b"]["body"], "body_b")
        self.assertEqual(monster["atk"]["weapon"], "weapon")
        self.assertEqual(monster["atk_b"]["weapon"], "weapon")

        self.assertEqual(evidence["gearCatalog"]["weaponRowCount"], 315)
        self.assertEqual(evidence["gearCatalog"]["visualSkinBinding"], "unresolved")


if __name__ == "__main__":
    unittest.main()
