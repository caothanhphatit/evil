#!/usr/bin/env python3
"""Extract packaged Hunter weapon and directional attack presentation evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_HUNTER = ROOT / "apps/web/public/content/releases/visible-world-v1/actors/hunter/hunter.json"
DEFAULT_MONSTER = ROOT / "apps/web/public/content/releases/visible-world-v1/actors/mon_a_01_1/mon_a_01_1.json"
DEFAULT_GEAR = ROOT / "packages/content/releases/evil-hunter-1.411/gear-catalog.json"
DEFAULT_SCHEMA = ROOT / "reverse-engineering/evidence/evil-ai-drop-runtime-schema-android-api35-v1.json"
DEFAULT_NATIVE = ROOT / "reverse-engineering/evidence/original-native-ai-runtime-v1.json"
DEFAULT_OUTPUT = ROOT / "reverse-engineering/evidence/hunter-weapon-attack-presentation-v1.json"


WEAPON_FAMILIES = {
    "h1": {"prefix": "weapon_h1_", "slot": "weapon_01", "attachment": "sword"},
    "h1a": {"prefix": "weapon_h1a_", "slot": "s_weapon", "attachment": "s_weapon"},
    "h2": {"prefix": "weapon_h2_", "slot": "weapon_02", "attachment": "hammer"},
    "h3": {"prefix": "weapon_h3_", "slot": "weapon_03", "attachment": "bow"},
    "h4": {"prefix": "weapon_h4_", "slot": "weapon_04", "attachment": "wand"},
    "h5": {"prefix": "weapon_h5_", "slot": "weapon_05", "attachment": "spear"},
}

BASIC_ATTACKS = [
    "h1_hit",
    "h1_hit_back",
    "h1_a_hit",
    "h1_a_hit_back",
    "h2_hit",
    "h2_hit_back",
    "h3_hit",
    "h3_hit_back",
    "h4_hit",
    "h4_hit_back",
    "h5_hit",
    "h5_hit_back",
]


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
    payload = schema.get("record", {}).get("payload", {})
    for item in payload.get("classes", []):
        if item.get("name") == name:
            return item
    raise ValueError(f"Missing schema type: {name}")


def method_record(native: dict[str, Any], type_name: str, method_name: str) -> dict[str, Any]:
    for item in native.get("methods", []):
        if item.get("type") == type_name and item.get("method") == method_name:
            return item
    raise ValueError(f"Missing native method: {type_name}.{method_name}")


def attachment_names(animation: dict[str, Any]) -> dict[str, str | None]:
    result: dict[str, str | None] = {}
    for slot, timelines in animation.get("slots", {}).items():
        attachments = timelines.get("attachment", [])
        result[slot] = attachments[0].get("name") if attachments else None
    return result


def weapon_visibility(animation: dict[str, Any]) -> dict[str, str | None]:
    result: dict[str, str | None] = {}
    for family, definition in WEAPON_FAMILIES.items():
        slot = definition["slot"]
        colors = animation.get("slots", {}).get(slot, {}).get("rgba", [])
        result[family] = colors[0].get("color") if colors else None
    return result


def build(args: argparse.Namespace) -> dict[str, Any]:
    hunter = load(args.hunter)
    monster = load(args.monster)
    gear = load(args.gear)
    schema = load(args.schema)
    native = load(args.native)

    hunter_skins = hunter["skins"]
    family_rows = []
    for family, definition in WEAPON_FAMILIES.items():
        skins = [skin for skin in hunter_skins if skin["name"].startswith(definition["prefix"])]
        invalid = []
        atlas_paths = []
        for skin in skins:
            slot_attachments = skin.get("attachments", {}).get(definition["slot"], {})
            if definition["attachment"] not in slot_attachments:
                invalid.append(skin["name"])
                continue
            atlas_paths.append(slot_attachments[definition["attachment"]].get("name"))
        family_rows.append(
            {
                "family": family,
                **definition,
                "skinCount": len(skins),
                "allUseExpectedSlotAndAttachment": not invalid,
                "invalidSkinNames": invalid,
                "sampleSkinNames": [skin["name"] for skin in skins[:4]],
                "sampleAtlasPaths": atlas_paths[:4],
            }
        )

    weapon_rows = [row for row in gear["rows"] if row.get("kind") == "weapon"]
    gear_jobs = []
    for job in sorted({row["job"] for row in weapon_rows}):
        rows = [row for row in weapon_rows if row["job"] == job]
        gear_jobs.append(
            {
                "job": job,
                "rowCount": len(rows),
                "minIndex": min(row["index"] for row in rows),
                "maxIndex": max(row["index"] for row in rows),
                "sampleNames": [row["name"] for row in rows[:4]],
            }
        )

    hunter_ctrl = type_record(schema, "HunterCtrl")
    evil_ctrl = type_record(schema, "EvilCtrl")
    hunter_field_names = {
        "mTransform",
        "mAnimation",
        "mCharacter",
        "AttackAniTime",
        "mNowAnimation",
        "mAttackCheck",
        "mTargetEvil",
        "TargetAttackCount",
    }
    evil_field_names = {
        "mTransform",
        "mTargetUnit",
        "PBOCIECIFIP",
        "CJGMDHPGAPL",
        "mCharacter",
        "mAnimation",
    }

    return {
        "schemaVersion": 1,
        "contractType": "hunter-weapon-attack-presentation-evidence",
        "runtimeCompatibility": "evidence-only",
        "sources": [
            source(args.hunter),
            source(args.monster),
            source(args.gear),
            source(args.schema),
            source(args.native),
        ],
        "hunterSkeleton": {
            "spineVersion": hunter.get("skeleton", {}).get("spine"),
            "slotCount": len(hunter["slots"]),
            "skinCount": len(hunter_skins),
            "animationCount": len(hunter["animations"]),
            "weaponBones": [
                bone for bone in hunter["bones"] if re.search(r"weapon|dummy_wp", bone["name"])
            ],
            "weaponSlots": [
                slot for slot in hunter["slots"] if re.search(r"weapon", slot["name"])
            ],
            "weaponFamilies": family_rows,
            "basicAttackAnimations": [
                {
                    "name": name,
                    "durationSeconds": max_time(hunter["animations"][name]),
                    "weaponSlotInitialColors": weapon_visibility(hunter["animations"][name]),
                }
                for name in BASIC_ATTACKS
            ],
            "directionalFamilies": {
                "walk": ["hunter_walk", "hunter_walk_back"],
                "damage": ["hunter_damage", "hunter_damage_back"],
                "basicAttackPairs": [
                    [name, f"{name}_back"]
                    for name in ["h1_hit", "h1_a_hit", "h2_hit", "h3_hit", "h4_hit", "h5_hit"]
                ],
            },
        },
        "monsterSkeleton": {
            "actor": "mon_a_01_1",
            "animationDurationsSeconds": {
                name: max_time(monster["animations"][name])
                for name in ["walk", "walk_b", "atk", "atk_b", "die", "dying"]
            },
            "directionalAttachmentNames": {
                name: attachment_names(monster["animations"][name])
                for name in ["walk", "walk_b", "atk", "atk_b"]
            },
        },
        "gearCatalog": {
            "weaponRowCount": len(weapon_rows),
            "jobs": gear_jobs,
            "visualSkinBinding": "unresolved",
        },
        "nativeBoundaries": {
            "hunterFields": [
                field for field in hunter_ctrl["fields"] if field["name"] in hunter_field_names
            ],
            "evilFields": [
                field for field in evil_ctrl["fields"] if field["name"] in evil_field_names
            ],
            "methods": [
                {
                    key: record[key]
                    for key in ["type", "method", "token", "moduleOffset", "nativeSizeBytes", "bodySha256"]
                }
                for record in [
                    method_record(native, "HunterCtrl", "HuntingAttackSetting"),
                    method_record(native, "HunterCtrl", "HuntingAttackAction"),
                    method_record(native, "EvilCtrl", "FixedUpdate"),
                    method_record(native, "EvilCtrl", "UnitAttack"),
                ]
            ],
        },
        "confirmed": [
            "Hunter weapons are Spine skin attachments composed into weapon-specific slots, not independent world sprites.",
            "The packaged h1/h2/h3/h4/h5 weapon slots use attachment labels sword/hammer/bow/wand/spear respectively.",
            "Basic Hunter attacks expose explicit front/back animation pairs; the selected job family slot is opaque while other base weapon slots are transparent.",
            "mon_a_01_1 uses explicit walk/walk_b and atk/atk_b clips; back clips switch body attachments to their _b variants while retaining the weapon attachment.",
            "Native Hunter attack state owns target, attack timing, current-animation integer and attack-check fields.",
            "Native Evil state owns the target Hunter, character transform, animation, and two stored Quaternion fields used inside FixedUpdate presentation branches.",
        ],
        "unresolved": [
            "Exact GearDefinition index to Spine weapon skin name mapping.",
            "Exact HunterData job/subjob values to h1/h1a/h2/h3/h4/h5 attack selection branches.",
            "Exact semantic meaning of the two EvilCtrl Quaternion fields and the obfuscated helper that selects/stores facing.",
            "Exact native moment at which damage lands relative to the Spine clip timeline.",
            "Whether every monster prefab uses explicit _b clips or some families mirror scale instead.",
        ],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--hunter", type=Path, default=DEFAULT_HUNTER)
    parser.add_argument("--monster", type=Path, default=DEFAULT_MONSTER)
    parser.add_argument("--gear", type=Path, default=DEFAULT_GEAR)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--native", type=Path, default=DEFAULT_NATIVE)
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
