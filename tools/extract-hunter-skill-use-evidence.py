#!/usr/bin/env python3
"""Extract bounded original Hunter skill-use evidence from packaged data."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_TABLES = ROOT / "reverse-engineering/evidence/hunter-info-tables-v1.json"
DEFAULT_DOMAIN_SCHEMA = (
    ROOT / "reverse-engineering/evidence/hunter-domain-runtime-schema-android-api30-v1.json"
)
DEFAULT_CTRL_SCHEMA = (
    ROOT / "reverse-engineering/evidence/evil-ai-drop-runtime-schema-android-api35-v1.json"
)
DEFAULT_NATIVE = ROOT / "reverse-engineering/evidence/original-native-ai-runtime-v1.json"
DEFAULT_HUNTER = ROOT / "apps/web/public/content/releases/visible-world-v1/actors/hunter/hunter.json"
DEFAULT_SELECTION = ROOT / "game-assets/manifests/original-flow-v1.selection.json"
DEFAULT_RANGER_PROJECTILE = (
    ROOT / "game-assets/extracted/exported/sprites/atk_ranger__3599.png"
)
DEFAULT_SORCERER_PROJECTILE = (
    ROOT / "game-assets/extracted/exported/sprites/atk_sorcerer__4256.png"
)
DEFAULT_OUTPUT = ROOT / "reverse-engineering/evidence/hunter-skill-use-runtime-v1.json"


SKILL_METHODS = {
    "SkillOn",
    "SkillOff",
    "SkillMent",
    "SkillMentEnd",
    "SkillStudy",
    "CheckManaOrb",
    "GetManaOrbPower",
    "RangeFireDamageAction",
    "RangedDualAttack",
    "FireDamageAction",
    "PulverizeAttack",
    "HolyBarrierAction",
    "HolyExplosion",
    "GetMisticArrow",
    "ChangeWalkRange",
    "HuntingAttackSetting",
    "HuntingAttackAction",
    "HuntingAttackEnd",
    "RefreshAnimation",
    "BuffSetting",
    "BuffEndSetting",
    "JobBuffSetting",
    "JobBuffEndSetting",
}

SKILL_FIELDS = {
    "mAttackDelay",
    "AttackAniTime",
    "mRange",
    "mWalkRange",
    "mSkillRange",
    "mNowAnimation",
    "mAttackCheck",
    "mEffectGroup",
    "mEffect",
    "mSkillMent",
    "mTargetEvil",
    "TargetAttackCount",
    "mSkillMentOrder",
    "mEffectOrder",
    "mHolyLightEffect",
    "mFrenzyBuffEffect",
    "mFrostArcherBuffEffect",
    "mAncientPowerBuffEffect",
    "mDeathCoilArmorBuffEffect",
    "mCommandBuffEffect",
    "mGraceOfLightBuffEffect",
    "mLightOfSanctificationBuffEffect",
    "mLightOfLifeBuffEffect",
    "mHolyExplosionEffect",
    "mRoundForceBuffEffect",
}

SPECIAL_ANIMATIONS = {
    "h1_hit_whirlwind",
    "h2_hit_executor",
    "h2_hit_executor_back",
    "h3_hit_arcane",
    "h3_hit_arcane_back",
    "h4_hit_darkload",
    "h4_hit_darkload_back",
    "h5_hit_roundslash",
    "h5_hit_roundslash_back",
    "h5_hit_shadejavelin",
    "h5_hit_dragonbreath_vehicle",
    "h5_hit_dragonbreath_back_vehicle",
}


def load(path: Path) -> Any:
    return json.loads(path.read_text())


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source(path: Path) -> dict[str, str]:
    return {"path": path.relative_to(ROOT).as_posix(), "sha256": digest(path)}


def max_time(value: Any) -> float:
    maximum = 0.0
    if isinstance(value, dict):
        time = value.get("time")
        if isinstance(time, (int, float)):
            maximum = float(time)
        for child in value.values():
            maximum = max(maximum, max_time(child))
    elif isinstance(value, list):
        for child in value:
            maximum = max(maximum, max_time(child))
    return maximum


def type_record(schema: dict[str, Any], name: str) -> dict[str, Any]:
    classes = schema.get("record", {}).get("payload", {}).get("classes", [])
    for item in classes:
        if item.get("name") == name:
            return item
    raise ValueError(f"Missing schema type: {name}")


def localized(row: dict[str, Any], locale: str, field: str) -> str | None:
    return row.get("localized", {}).get(locale, {}).get(field)


def basic_skill(row: dict[str, Any]) -> dict[str, Any]:
    return {
        "index": row["index"],
        "job": row["job"],
        "maxLevel": row["maxLevel"],
        "coolTime": row["coolTime"],
        "keepTimeByLevel": row["keepTimeByLevel"],
        "keepValueByLevel": row["keepValueByLevel"],
        "valueTimeByLevel": row["valueTimeByLevel"],
        "valueCountByLevel": row["valueCountByLevel"],
        "studyLevel": row["studyLevel"],
        "studyMoney": row["studyMoney"],
        "nameEn": localized(row, "en", "name"),
        "nameVi": localized(row, "vi", "name"),
        "descriptionEn": localized(row, "en", "description"),
        "detailDescriptionEn": localized(row, "en", "detailDescription"),
    }


def sub_job_skill(row: dict[str, Any]) -> dict[str, Any]:
    numeric_fields = [
        "index",
        "job",
        "subJob",
        "thirdJob",
        "fourthJob",
        "maxLevel",
        "coolTime",
        "upCoolTime",
        "keepTime",
        "upKeepTime",
        "keepValue",
        "upKeepValue",
        "secondValue",
        "upSecondValue",
        "valueTime",
        "upValueTime",
        "valueCount",
        "upValueCount",
        "firstStudySoul",
        "addStudySoul",
    ]
    return {
        **{field: row[field] for field in numeric_fields},
        "nameEn": localized(row, "en", "name"),
        "nameVi": localized(row, "vi", "name"),
        "descriptionEn": localized(row, "en", "description"),
        "detailDescriptionEn": localized(row, "en", "detailDescription"),
    }


def method_signature(method: dict[str, Any]) -> dict[str, Any]:
    return {
        "name": method["name"],
        "token": method["token"],
        "returnType": method["returnType"],
        "parameters": [
            {"index": parameter["index"], "name": parameter["name"], "type": parameter["type"]}
            for parameter in method.get("parameters", [])
        ],
    }


def build(args: argparse.Namespace) -> dict[str, Any]:
    tables = load(args.tables)
    domain_schema = load(args.domain_schema)
    ctrl_schema = load(args.ctrl_schema)
    native = load(args.native)
    hunter = load(args.hunter)
    selection = load(args.selection)

    skill_data = type_record(domain_schema, "SkillData")
    hunter_ctrl = type_record(ctrl_schema, "HunterCtrl")
    hunting_attack = next(
        method
        for method in native["methods"]
        if method.get("type") == "HunterCtrl" and method.get("method") == "HuntingAttackAction"
    )

    ranger_manifest = next(
        asset for asset in selection["assets"] if asset.get("id") == "field.ranger-arrow"
    )
    special_animations = [
        {"name": name, "durationSeconds": max_time(animation)}
        for name, animation in hunter["animations"].items()
        if name in SPECIAL_ANIMATIONS
    ]

    return {
        "schemaVersion": 1,
        "contractType": "hunter-skill-use-runtime-evidence",
        "runtimeCompatibility": "evidence-only",
        "sources": [
            source(args.tables),
            source(args.domain_schema),
            source(args.ctrl_schema),
            source(args.native),
            source(args.hunter),
            source(args.selection),
            source(args.ranger_projectile),
            source(args.sorcerer_projectile),
        ],
        "catalog": {
            "basicSkills": [basic_skill(row) for row in tables["skills"]],
            "subJobSkills": [sub_job_skill(row) for row in tables["subJobSkills"]],
        },
        "perHunterSkillSnapshot": {
            "type": "SkillData",
            "fields": skill_data["fields"],
            "containerEvidence": "HunterData.skill is Dictionary<String, SkillData> in the captured runtime schema.",
            "dictionaryKeyMeaning": "unresolved",
        },
        "hunterCtrl": {
            "methods": [
                method_signature(method)
                for method in hunter_ctrl["methods"]
                if method["name"] in SKILL_METHODS
            ],
            "fields": [
                field for field in hunter_ctrl["fields"] if field["name"] in SKILL_FIELDS
            ],
        },
        "nativeHuntingAttackAction": {
            key: hunting_attack[key]
            for key in [
                "token",
                "moduleOffset",
                "nativeSizeBytes",
                "bodySha256",
                "knownDirectCalls",
                "schemaFieldReferences",
            ]
        },
        "presentation": {
            "specialHunterAnimations": special_animations,
            "confirmedRangerProjectile": ranger_manifest,
            "sorcererProjectileCandidate": {
                "path": args.sorcerer_projectile.relative_to(ROOT).as_posix(),
                "assetNameFromFilename": "atk_sorcerer",
                "bindingState": "exported-file-only-unresolved",
            },
        },
        "confirmed": [
            "The package defines 10 basic skills and 40 sub-job skills with job paths, cooldown/effect parameters and localized descriptions.",
            "Each Hunter can carry a SkillData snapshot containing index, skillIndex, coolTime and level inside HunterData.skill.",
            "HunterCtrl exposes explicit skill activation, study, cooldown-state, buff/effect and melee/ranged attack boundaries.",
            "HuntingAttackAction directly reaches FireDamageAction, PulverizeAttack, GetMisticArrow, WarcryAction, CurseDamage and multiple trait/familiar handlers.",
            "The Hunter Spine actor contains named advanced attack animations, while atk_ranger is a scene-component-confirmed Ranger projectile asset.",
        ],
        "unresolved": [
            "Exact basic/sub-job skill row to Spine animation, effect-controller index and icon for mappings not separately confirmed.",
            "Exact HunterData.skill dictionary key meaning and live learned-skill values for a captured Hunter.",
            "Exact skill-selection branch inside HuntingAttackAction and exact hit/projectile spawn frames.",
            "Exact mana-orb resource formula and whether each skill consumes it; no generic mana field is proven in HunterData.",
            "Exact binding of atk_sorcerer to a HunterCtrl ranged branch.",
            "Original damage, critical, cooldown validation and effect formulas not already recovered by bounded native evidence.",
        ],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tables", type=Path, default=DEFAULT_TABLES)
    parser.add_argument("--domain-schema", type=Path, default=DEFAULT_DOMAIN_SCHEMA)
    parser.add_argument("--ctrl-schema", type=Path, default=DEFAULT_CTRL_SCHEMA)
    parser.add_argument("--native", type=Path, default=DEFAULT_NATIVE)
    parser.add_argument("--hunter", type=Path, default=DEFAULT_HUNTER)
    parser.add_argument("--selection", type=Path, default=DEFAULT_SELECTION)
    parser.add_argument("--ranger-projectile", type=Path, default=DEFAULT_RANGER_PROJECTILE)
    parser.add_argument("--sorcerer-projectile", type=Path, default=DEFAULT_SORCERER_PROJECTILE)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    evidence = build(args)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote {args.output}")


if __name__ == "__main__":
    main()
