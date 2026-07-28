import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "tools/extract-hunter-skill-use-evidence.py"
SPEC = importlib.util.spec_from_file_location("hunter_skill_use_evidence", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class HunterSkillUseEvidenceTest(unittest.TestCase):
    def test_catalog_runtime_and_presentation_boundaries(self) -> None:
        evidence = MODULE.build(
            type(
                "Args",
                (),
                {
                    "tables": MODULE.DEFAULT_TABLES,
                    "domain_schema": MODULE.DEFAULT_DOMAIN_SCHEMA,
                    "ctrl_schema": MODULE.DEFAULT_CTRL_SCHEMA,
                    "native": MODULE.DEFAULT_NATIVE,
                    "hunter": MODULE.DEFAULT_HUNTER,
                    "selection": MODULE.DEFAULT_SELECTION,
                    "ranger_projectile": MODULE.DEFAULT_RANGER_PROJECTILE,
                    "sorcerer_projectile": MODULE.DEFAULT_SORCERER_PROJECTILE,
                },
            )()
        )

        basic = evidence["catalog"]["basicSkills"]
        sub_jobs = evidence["catalog"]["subJobSkills"]
        self.assertEqual(len(basic), 10)
        self.assertEqual(len(sub_jobs), 40)
        self.assertEqual(basic[0]["nameEn"], "Fury")
        self.assertEqual(basic[0]["coolTime"], 15.0)
        self.assertEqual(sub_jobs[0]["nameEn"], "Dual Weapon")
        self.assertEqual(sub_jobs[0]["index"], 100)

        fields = {
            field["name"]: field
            for field in evidence["perHunterSkillSnapshot"]["fields"]
        }
        self.assertEqual(fields["<coolTime>k__BackingField"]["offset"], 48)
        self.assertEqual(fields["<level>k__BackingField"]["offset"], 88)

        methods = {method["name"] for method in evidence["hunterCtrl"]["methods"]}
        self.assertIn("HuntingAttackAction", methods)
        self.assertIn("FireDamageAction", methods)
        self.assertIn("SkillOn", methods)

        calls = {
            call["method"]
            for call in evidence["nativeHuntingAttackAction"]["knownDirectCalls"]
        }
        self.assertIn("HunterCtrl.FireDamageAction", calls)
        self.assertIn("HunterCtrl.PulverizeAttack", calls)

        animations = {
            animation["name"]
            for animation in evidence["presentation"]["specialHunterAnimations"]
        }
        self.assertIn("h3_hit_arcane", animations)
        self.assertIn("h5_hit_roundslash", animations)
        self.assertEqual(
            evidence["presentation"]["confirmedRangerProjectile"]["bindingState"],
            "scene-component-confirmed",
        )


if __name__ == "__main__":
    unittest.main()
