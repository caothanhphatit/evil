#!/usr/bin/env python3
"""Build the versioned rebuild weapon catalog and PostgreSQL import bundle."""

from __future__ import annotations

import hashlib
import json
from decimal import Decimal, ROUND_CEILING, ROUND_HALF_EVEN
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WEAPON_SOURCE = ROOT / "tools/asset-generation-pipeline/profiles/weapons/catalog.json"
QUICKSHEET_SOURCE = ROOT / "reverse-engineering/evidence/quicksheet-decoded-v1.json"
RELEASE_DIR = ROOT / "packages/content/releases/evil-hunter-rebuild-v1"
CATALOG_OUT = RELEASE_DIR / "weapon-core-catalog.json"
SQL_OUT = ROOT / "infra/db/core_game/002_rebuild_weapon_core.sql"
RELEASE_ID = "evil-hunter-rebuild-v1.weapon-core-v1"

CLASS_CONFIG = {
    "berserker": {"displayName": "Berserker", "weaponFamily": "greatsword-axe", "packageSourceIndex": 0, "legacyIconIndices": [0, 1, 2, 3, 4, 5, 6], "packageSecondValue": 180},
    "paladin": {"displayName": "Paladin", "weaponFamily": "hammer-maul", "packageSourceIndex": 9, "legacyIconIndices": [9, 10, 11, 12, 13, 14, 15], "packageSecondValue": 200},
    "ranger": {"displayName": "Ranger", "weaponFamily": "bow", "packageSourceIndex": 18, "legacyIconIndices": [18, 19, 20, 21, 22, 23, 24], "packageSecondValue": 150},
    "sorcerer": {"displayName": "Sorcerer", "weaponFamily": "staff-scepter", "packageSourceIndex": 27, "legacyIconIndices": [27, 28, 29, 30, 31, 32, 33], "packageSecondValue": 210},
    "dark_knight": {"displayName": "Dark Knight", "weaponFamily": "spear-glaive", "packageSourceIndex": 252, "legacyIconIndices": [252, 253, 254, 255, 256, 257, 258], "packageSecondValue": 200},
}

RARITIES = [
    {"id": "normal", "displayName": "Normal", "color": "white", "prefixSlots": 0, "suffixSlots": 0},
    {"id": "blue", "displayName": "Blue", "color": "blue", "prefixSlots": 1, "suffixSlots": 1},
    {"id": "purple", "displayName": "Purple", "color": "purple", "prefixSlots": 2, "suffixSlots": 2},
    {"id": "gold", "displayName": "Gold", "color": "gold", "prefixSlots": 3, "suffixSlots": 3},
]

WEAPON_AFFIX_POOL = [
    {"modifierId": "rebuild:flat_attack", "slot": "prefix", "family": "flat_attack", "exclusiveGroup": "attack_base", "weight": 100},
    {"modifierId": "gear_property:5", "slot": "prefix", "family": "attack_percent", "exclusiveGroup": "attack_percent", "weight": 100},
    {"modifierId": "gear_property:36", "slot": "prefix", "family": "additional_damage", "exclusiveGroup": "additional_damage", "weight": 80},
    {"modifierId": "gear_property:43", "slot": "prefix", "family": "critical_damage", "exclusiveGroup": "critical_damage", "weight": 80},
    {"modifierId": "gear_property:11", "slot": "prefix", "family": "primate_damage", "exclusiveGroup": "race_damage", "weight": 50},
    {"modifierId": "gear_property:12", "slot": "prefix", "family": "demon_damage", "exclusiveGroup": "race_damage", "weight": 50},
    {"modifierId": "gear_property:13", "slot": "prefix", "family": "undead_damage", "exclusiveGroup": "race_damage", "weight": 50},
    {"modifierId": "gear_property:41", "slot": "prefix", "family": "boss_damage", "exclusiveGroup": "race_damage", "weight": 50},
    {"modifierId": "gear_property:42", "slot": "prefix", "family": "animal_damage", "exclusiveGroup": "race_damage", "weight": 50},
    {"modifierId": "gear_property:6", "slot": "prefix", "family": "attack_speed", "exclusiveGroup": "attack_speed", "weight": 100},
    {"modifierId": "gear_property:7", "slot": "prefix", "family": "critical_chance", "exclusiveGroup": "critical_chance", "weight": 100},
    {"modifierId": "gear_property:8", "slot": "suffix", "family": "movement_speed", "exclusiveGroup": "movement_speed", "weight": 80},
    {"modifierId": "gear_property:34", "slot": "prefix", "family": "lifesteal", "exclusiveGroup": "sustain", "weight": 70},
    {"modifierId": "gear_property:21", "slot": "suffix", "family": "stun_chance", "exclusiveGroup": "control_proc", "weight": 70},
    {"modifierId": "gear_property:9", "slot": "suffix", "family": "double_gold_chance", "exclusiveGroup": "economy_bonus", "weight": 60},
    {"modifierId": "gear_property:10", "slot": "suffix", "family": "extra_material_chance", "exclusiveGroup": "economy_bonus", "weight": 60},
    {"modifierId": "gear_property:40", "slot": "suffix", "family": "experience_gain", "exclusiveGroup": "economy_bonus", "weight": 60},
    {"modifierId": "gear_property:45", "slot": "suffix", "family": "mood_recovery_proc", "exclusiveGroup": "need_recovery", "weight": 40},
    {"modifierId": "gear_property:46", "slot": "suffix", "family": "stamina_recovery_proc", "exclusiveGroup": "need_recovery", "weight": 40},
    {"modifierId": "gear_property:47", "slot": "suffix", "family": "satiety_recovery_proc", "exclusiveGroup": "need_recovery", "weight": 40},
]

POOL_BY_MODIFIER = {row["modifierId"]: row for row in WEAPON_AFFIX_POOL}


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def integer(value: int | float) -> int:
    result = int(value)
    if result != value:
        raise ValueError(f"expected integer-compatible value, got {value}")
    return result


def base_power(level: int) -> int:
    value = Decimal(60) * (Decimal("1.6") ** Decimal(level // 100))
    return int(value.quantize(Decimal("1"), rounding=ROUND_HALF_EVEN))


def package_tier_range(source_range: list[int | float], difficulty: int) -> tuple[int, int]:
    minimum = Decimal(str(source_range[0]))
    maximum = Decimal(str(source_range[-1]))
    span = maximum - minimum
    low = minimum + span * Decimal(difficulty - 1) / Decimal(9)
    high = minimum + span * Decimal(difficulty + 1) / Decimal(9)
    return (
        int(low.quantize(Decimal("1"), rounding=ROUND_HALF_EVEN)),
        int(min(maximum, high).quantize(Decimal("1"), rounding=ROUND_HALF_EVEN)),
    )


def flat_attack_tier_range(difficulty: int) -> tuple[int, int]:
    minimum = Decimal(base_power((difficulty - 1) * 100)) * Decimal("0.12")
    maximum = Decimal(base_power(difficulty * 100)) * Decimal("0.20")
    return (
        int(minimum.quantize(Decimal("1"), rounding=ROUND_CEILING)),
        int(maximum.quantize(Decimal("1"), rounding=ROUND_HALF_EVEN)),
    )


def legacy_icon_index(class_config: dict, unlock_level: int) -> int:
    # The old package has seven icon rows per class; reuse the final row for
    # the eighth rebuild base until new art is generated.
    return class_config["legacyIconIndices"][min(unlock_level // 100, 6)]


def build_catalog() -> dict:
    visual = read_json(WEAPON_SOURCE)
    decoded = read_json(QUICKSHEET_SOURCE)["decoded"]

    difficulties = []
    for difficulty in range(1, 9):
        min_level = (difficulty - 1) * 100
        max_level = difficulty * 100
        difficulties.append({
            "difficulty": difficulty,
            "minLevel": min_level,
            "maxLevel": max_level,
            "basePowerMin": base_power(min_level),
            "basePowerMax": base_power(max_level),
            "evidenceState": "rebuild-designed",
        })

    package_weapons = {integer(row["index"]): row for row in decoded["gearWeapon"]["rows"]}
    weapons = []
    for row in visual["weapons"]:
        class_config = CLASS_CONFIG[row["class"]]
        source = package_weapons[class_config["packageSourceIndex"]]
        if integer(source["secondValue"]) != class_config["packageSecondValue"]:
            raise ValueError(f"package secondValue drift for {row['class']}")
        unlock = integer(row["unlockLevel"])
        difficulty = unlock // 100 + 1
        level_cap = min(unlock + 100, 800)
        visual_class = visual["classes"][row["class"]]
        legacy_index = legacy_icon_index(class_config, unlock)
        attack_min = base_power(unlock)
        attack_max = base_power(level_cap)
        weapons.append({
            "id": row["id"],
            "classId": row["class"],
            "className": class_config["displayName"],
            "weaponFamily": class_config["weaponFamily"],
            "difficulty": difficulty,
            "unlockLevel": unlock,
            "baseLevelCap": level_cap,
            "basePower": base_power(unlock),
            "capPower": base_power(level_cap),
            "attackDamageMin": attack_min,
            "attackDamageMax": attack_max,
            "attackDamageLine": f"+ {attack_min}-{attack_max} Attack Damage",
            "packageSecondValue": class_config["packageSecondValue"],
            "packageSourceIndex": class_config["packageSourceIndex"],
            "localization": {"en": row["en"], "vi": row["vi"]},
            "visual": {
                "family": visual_class["family"],
                "spineSlot": visual_class["slot"],
                "attachment": visual_class["attachment"],
                "theme": row["theme"],
                "assetState": "legacy-package-icon",
                "expectedInventoryIcon": f"/content/releases/evil-hunter-1.411/gear-icons/weapon-{legacy_index}.png",
                "expectedSpineAttachment": f"game-assets/rebuild/weapons/{row['id']}/spine/attachment.png",
            },
            "evidenceState": "rebuild-designed",
            "active": True,
        })

    modifiers = []
    for row in decoded["gearProperty"]["rows"]:
        source_id = integer(row["idx"])
        property_kind = "ordinary-explicit"
        slot_assignment = "unresolved"
        generation_state = "unresolved"
        pool = POOL_BY_MODIFIER.get(f"gear_property:{source_id}")
        if source_id in (48, 49):
            property_kind = "special-explicit-transformation"
        elif source_id == 50:
            property_kind = "virtue-support"
        elif pool:
            slot_assignment = pool["slot"]
            generation_state = "rebuild-designed"
        modifiers.append({
            "id": f"gear_property:{source_id}",
            "sourceId": source_id,
            "origin": "package",
            "nameEn": row["engdesc"],
            "nameVi": row["vnmdesc"],
            "propertyKind": property_kind,
            "slotAssignment": slot_assignment,
            "generationState": generation_state,
            "family": pool["family"] if pool else None,
            "exclusiveGroup": pool["exclusiveGroup"] if pool else None,
            "positiveRollMode": integer(row["randomPlusYn"]),
            "positiveValues": row["randomPlusValue"],
            "negativeRollMode": integer(row["randomMinusYn"]),
            "negativeValues": row["randomMinusValue"],
            "upgradePercent": row["randomUpgadePercent"],
            "powerValue": row["powerValue"],
            "uniqueOption": bool(integer(row["uniqueOptionYn"])),
            "searchable": bool(integer(row["searchPassible"])),
            "gearSkillId": integer(row["gearSkillidx"]) or None,
            "evidenceState": "package-confirmed",
            "raw": row,
        })

    modifiers.append({
        "id": "rebuild:flat_attack",
        "sourceId": None,
        "origin": "rebuild",
        "nameEn": "Adds value Attack Damage",
        "nameVi": "Thêm value Sát Thương Tấn Công",
        "propertyKind": "ordinary-explicit",
        "slotAssignment": "prefix",
        "generationState": "rebuild-designed",
        "family": "flat_attack",
        "exclusiveGroup": "attack_base",
        "positiveRollMode": 1,
        "positiveValues": [],
        "negativeRollMode": 0,
        "negativeValues": [],
        "upgradePercent": 0,
        "powerValue": 0,
        "uniqueOption": False,
        "searchable": True,
        "gearSkillId": None,
        "evidenceState": "rebuild-designed",
        "raw": None,
    })

    modifiers_by_id = {row["id"]: row for row in modifiers}
    affix_tiers = []
    for pool in WEAPON_AFFIX_POOL:
        modifier = modifiers_by_id[pool["modifierId"]]
        for difficulty in range(1, 9):
            if pool["modifierId"] == "rebuild:flat_attack":
                minimum, maximum = flat_attack_tier_range(difficulty)
                value_basis = "base-power-percentage"
            else:
                minimum, maximum = package_tier_range(modifier["positiveValues"], difficulty)
                value_basis = "package-range-partition"
            affix_tiers.append({
                "id": f"{pool['modifierId']}:d{difficulty}",
                "modifierId": pool["modifierId"],
                "difficulty": difficulty,
                "minimumItemLevel": (difficulty - 1) * 100,
                "maximumItemLevel": difficulty * 100,
                "minimumValue": minimum,
                "maximumValue": maximum,
                "valueBasis": value_basis,
                "evidenceState": "rebuild-designed",
            })

    virtues = []
    for row in decoded["gearSetProperty"]["rows"]:
        source_id = integer(row["idx"])
        if source_id == 0:
            continue
        virtues.append({
            "id": f"virtue:{source_id}",
            "sourceId": source_id,
            "nameEn": row["engname"],
            "nameVi": row["vnmname"],
            "descriptionEn": row["engdesc"],
            "descriptionVi": row["vnmdesc"],
            "thresholdValues": row["firstValue"],
            "secondaryValue": row["secondValue"],
            "tertiaryValue": row["thirdValue"],
            "evidenceState": "package-confirmed",
            "raw": row,
        })

    collection_sets = []
    for row in decoded["collectset"]["rows"]:
        source_id = integer(row["idx"])
        collection_sets.append({
            "id": f"collection_set:{source_id}",
            "sourceId": source_id,
            "nameEn": row["engname"],
            "nameVi": row["vnmname"],
            "specialItemIds": row["specialidx"],
            "optionType": integer(row["optionType"]),
            "optionValue": row["optionValue"],
            "visible": integer(row["visible"]),
            "effectState": "unresolved",
            "evidenceState": "package-confirmed",
            "raw": row,
        })

    return {
        "schemaVersion": 2,
        "releaseId": RELEASE_ID,
        "status": "draft-affix-pools-complete-special-explicit-unresolved",
        "locales": ["en", "vi"],
        "formula": {
            "effectiveLevel": "min(rolledLevel,difficulty*100,baseLevelCap)",
            "basePower": "roundHalfEven(60*1.6^(level/100)) at 100-level thresholds",
            "packageDamageFormula": "roundToEven(firstValue*ratingValue/100*(1+firstPercent/100)*qualityMultiplier*secondValue/100)",
            "affixTierSelection": "use the highest eligible modifier tier whose minimum item level is <= item level",
            "affixRoll": "uniform integer in [minimumValue,maximumValue] using the server-owned RNG stream",
            "duplicateRule": "an item cannot contain two affixes with the same exclusiveGroup",
            "evidenceState": "rebuild-designed-informed-by-package-range",
        },
        "difficulties": difficulties,
        "rarities": RARITIES,
        "weapons": weapons,
        "modifiers": modifiers,
        "affixTiers": affix_tiers,
        "weaponModifierPool": [{
            **row,
            "weaponClass": "*",
            "minimumDifficulty": 1,
            "maximumDifficulty": 8,
            "active": True,
            "evidenceState": "rebuild-designed",
        } for row in WEAPON_AFFIX_POOL],
        "virtues": virtues,
        "collectionSets": collection_sets,
        "unresolved": [
            "Archangel and Demon Lord acquisition pools are not bound by package gear rows.",
            "Collection-set optionType and optionValue runtime semantics remain unresolved.",
            "Generated weapon art is not present until the asset pipeline outputs validated derivatives.",
        ],
        "sources": [
            {"id": "weapon-visual-catalog", "path": str(WEAPON_SOURCE.relative_to(ROOT)), "sha256": sha256(WEAPON_SOURCE)},
            {"id": "quicksheet-decoded", "path": str(QUICKSHEET_SOURCE.relative_to(ROOT)), "sha256": sha256(QUICKSHEET_SOURCE)},
        ],
    }


def sql_text(value: str | None) -> str:
    if value is None:
        return "NULL"
    return "'" + value.replace("'", "''") + "'"


def sql_json(value: object) -> str:
    return sql_text(json.dumps(value, ensure_ascii=False, separators=(",", ":"))) + "::jsonb"


def sql_bool(value: bool) -> str:
    return "TRUE" if value else "FALSE"


def build_sql(catalog: dict) -> str:
    lines = [
        "-- Generated by tools/generate-rebuild-weapon-core.py; do not hand-edit.",
        "-- Rebuild-designed weapon progression plus package-confirmed modifier/set evidence.",
        "CREATE SCHEMA IF NOT EXISTS core_game;",
        "DROP TABLE IF EXISTS core_game.rebuild_weapon_affix_pool, core_game.rebuild_affix_tier, core_game.rebuild_weapon_visual_binding, core_game.rebuild_weapon_localization, core_game.rebuild_weapon_base, core_game.rebuild_collection_set, core_game.rebuild_virtue_effect, core_game.rebuild_affix, core_game.rebuild_item_rarity, core_game.rebuild_difficulty CASCADE;",
        "CREATE TABLE core_game.rebuild_difficulty (release_id TEXT NOT NULL REFERENCES core_game.catalog_release(release_id) ON DELETE CASCADE, difficulty INTEGER NOT NULL CHECK (difficulty BETWEEN 1 AND 8), min_level INTEGER NOT NULL, max_level INTEGER NOT NULL, base_power_min INTEGER NOT NULL, base_power_max INTEGER NOT NULL, evidence_state TEXT NOT NULL, payload JSONB NOT NULL, PRIMARY KEY (release_id, difficulty));",
        "CREATE TABLE core_game.rebuild_item_rarity (release_id TEXT NOT NULL REFERENCES core_game.catalog_release(release_id) ON DELETE CASCADE, rarity_id TEXT NOT NULL, display_name TEXT NOT NULL, color TEXT NOT NULL, prefix_slots INTEGER NOT NULL CHECK (prefix_slots BETWEEN 0 AND 3), suffix_slots INTEGER NOT NULL CHECK (suffix_slots BETWEEN 0 AND 3), payload JSONB NOT NULL, PRIMARY KEY (release_id, rarity_id));",
        "CREATE TABLE core_game.rebuild_affix (release_id TEXT NOT NULL REFERENCES core_game.catalog_release(release_id) ON DELETE CASCADE, affix_id TEXT NOT NULL, source_id INTEGER, origin TEXT NOT NULL CHECK (origin IN ('package', 'rebuild')), name_en TEXT NOT NULL, name_vi TEXT NOT NULL, property_kind TEXT NOT NULL, slot_assignment TEXT NOT NULL, generation_state TEXT NOT NULL, family TEXT, exclusive_group TEXT, positive_values JSONB NOT NULL, negative_values JSONB NOT NULL, gear_skill_id INTEGER, evidence_state TEXT NOT NULL, payload JSONB NOT NULL, PRIMARY KEY (release_id, affix_id), UNIQUE (release_id, source_id));",
        "CREATE TABLE core_game.rebuild_affix_tier (release_id TEXT NOT NULL, tier_id TEXT NOT NULL, affix_id TEXT NOT NULL, difficulty INTEGER NOT NULL CHECK (difficulty BETWEEN 1 AND 8), minimum_item_level INTEGER NOT NULL, maximum_item_level INTEGER NOT NULL, minimum_value INTEGER NOT NULL, maximum_value INTEGER NOT NULL CHECK (maximum_value >= minimum_value), value_basis TEXT NOT NULL, evidence_state TEXT NOT NULL, payload JSONB NOT NULL, PRIMARY KEY (release_id, tier_id), UNIQUE (release_id, affix_id, difficulty), FOREIGN KEY (release_id, affix_id) REFERENCES core_game.rebuild_affix(release_id, affix_id) ON DELETE CASCADE);",
        "CREATE TABLE core_game.rebuild_virtue_effect (release_id TEXT NOT NULL REFERENCES core_game.catalog_release(release_id) ON DELETE CASCADE, virtue_id TEXT NOT NULL, source_id INTEGER NOT NULL, name_en TEXT NOT NULL, name_vi TEXT NOT NULL, description_en TEXT NOT NULL, description_vi TEXT NOT NULL, threshold_values JSONB NOT NULL, secondary_value DOUBLE PRECISION NOT NULL, tertiary_value DOUBLE PRECISION NOT NULL, evidence_state TEXT NOT NULL, payload JSONB NOT NULL, PRIMARY KEY (release_id, virtue_id), UNIQUE (release_id, source_id));",
        "CREATE TABLE core_game.rebuild_collection_set (release_id TEXT NOT NULL REFERENCES core_game.catalog_release(release_id) ON DELETE CASCADE, set_id TEXT NOT NULL, source_id INTEGER NOT NULL, name_en TEXT NOT NULL, name_vi TEXT NOT NULL, special_item_ids JSONB NOT NULL, option_type INTEGER NOT NULL, option_value DOUBLE PRECISION NOT NULL, visible INTEGER NOT NULL, effect_state TEXT NOT NULL, evidence_state TEXT NOT NULL, payload JSONB NOT NULL, PRIMARY KEY (release_id, set_id), UNIQUE (release_id, source_id));",
        "CREATE TABLE core_game.rebuild_weapon_base (release_id TEXT NOT NULL REFERENCES core_game.catalog_release(release_id) ON DELETE CASCADE, weapon_id TEXT NOT NULL, class_id TEXT NOT NULL, class_name TEXT NOT NULL, weapon_family TEXT NOT NULL, difficulty INTEGER NOT NULL, unlock_level INTEGER NOT NULL, base_level_cap INTEGER NOT NULL, base_power INTEGER NOT NULL, cap_power INTEGER NOT NULL, package_second_value INTEGER NOT NULL, package_source_index INTEGER NOT NULL, evidence_state TEXT NOT NULL, active BOOLEAN NOT NULL, payload JSONB NOT NULL, PRIMARY KEY (release_id, weapon_id), FOREIGN KEY (release_id, difficulty) REFERENCES core_game.rebuild_difficulty(release_id, difficulty));",
        "CREATE TABLE core_game.rebuild_weapon_localization (release_id TEXT NOT NULL, weapon_id TEXT NOT NULL, locale TEXT NOT NULL CHECK (locale IN ('en', 'vi')), display_name TEXT NOT NULL, PRIMARY KEY (release_id, weapon_id, locale), FOREIGN KEY (release_id, weapon_id) REFERENCES core_game.rebuild_weapon_base(release_id, weapon_id) ON DELETE CASCADE);",
        "CREATE TABLE core_game.rebuild_weapon_visual_binding (release_id TEXT NOT NULL, weapon_id TEXT NOT NULL, family TEXT NOT NULL, spine_slot TEXT NOT NULL, attachment TEXT NOT NULL, theme TEXT NOT NULL, asset_state TEXT NOT NULL, inventory_icon_path TEXT NOT NULL, spine_attachment_path TEXT NOT NULL, payload JSONB NOT NULL, PRIMARY KEY (release_id, weapon_id), FOREIGN KEY (release_id, weapon_id) REFERENCES core_game.rebuild_weapon_base(release_id, weapon_id) ON DELETE CASCADE);",
        "CREATE TABLE core_game.rebuild_weapon_affix_pool (release_id TEXT NOT NULL, weapon_class TEXT NOT NULL, affix_id TEXT NOT NULL, slot TEXT NOT NULL CHECK (slot IN ('prefix', 'suffix', 'special-explicit')), family TEXT NOT NULL, exclusive_group TEXT NOT NULL, weight INTEGER NOT NULL CHECK (weight > 0), minimum_difficulty INTEGER NOT NULL CHECK (minimum_difficulty BETWEEN 1 AND 8), maximum_difficulty INTEGER NOT NULL CHECK (maximum_difficulty BETWEEN minimum_difficulty AND 8), active BOOLEAN NOT NULL, evidence_state TEXT NOT NULL, PRIMARY KEY (release_id, weapon_class, affix_id, slot), FOREIGN KEY (release_id, affix_id) REFERENCES core_game.rebuild_affix(release_id, affix_id) ON DELETE CASCADE);",
        f"DELETE FROM core_game.catalog_release WHERE release_id = {sql_text(RELEASE_ID)};",
        f"INSERT INTO core_game.catalog_release VALUES ({sql_text(RELEASE_ID)}, 'rebuild-v1', {sql_text(catalog['status'])}, 1, {sql_json({'catalogs': ['weapon-core-catalog.json'], 'counts': {'weapons': len(catalog['weapons']), 'modifiers': len(catalog['modifiers']), 'affixTiers': len(catalog['affixTiers']), 'weaponModifierPool': len(catalog['weaponModifierPool']), 'virtues': len(catalog['virtues']), 'collectionSets': len(catalog['collectionSets'])}, 'unresolved': catalog['unresolved']})});",
    ]

    for source in catalog["sources"]:
        path = ROOT / source["path"]
        lines.append(f"INSERT INTO core_game.catalog_source VALUES ({sql_text(RELEASE_ID)}, {sql_text(source['id'])}, {sql_text(source['path'])}, {path.stat().st_size}, {sql_text(source['sha256'])});")
    for row in catalog["difficulties"]:
        lines.append(f"INSERT INTO core_game.rebuild_difficulty VALUES ({sql_text(RELEASE_ID)}, {row['difficulty']}, {row['minLevel']}, {row['maxLevel']}, {row['basePowerMin']}, {row['basePowerMax']}, {sql_text(row['evidenceState'])}, {sql_json(row)});")
    for row in catalog["rarities"]:
        lines.append(f"INSERT INTO core_game.rebuild_item_rarity VALUES ({sql_text(RELEASE_ID)}, {sql_text(row['id'])}, {sql_text(row['displayName'])}, {sql_text(row['color'])}, {row['prefixSlots']}, {row['suffixSlots']}, {sql_json(row)});")
    for row in catalog["modifiers"]:
        lines.append(f"INSERT INTO core_game.rebuild_affix VALUES ({sql_text(RELEASE_ID)}, {sql_text(row['id'])}, {row['sourceId'] if row['sourceId'] is not None else 'NULL'}, {sql_text(row['origin'])}, {sql_text(row['nameEn'])}, {sql_text(row['nameVi'])}, {sql_text(row['propertyKind'])}, {sql_text(row['slotAssignment'])}, {sql_text(row['generationState'])}, {sql_text(row['family'])}, {sql_text(row['exclusiveGroup'])}, {sql_json(row['positiveValues'])}, {sql_json(row['negativeValues'])}, {row['gearSkillId'] if row['gearSkillId'] is not None else 'NULL'}, {sql_text(row['evidenceState'])}, {sql_json(row)});")
    for row in catalog["affixTiers"]:
        lines.append(f"INSERT INTO core_game.rebuild_affix_tier VALUES ({sql_text(RELEASE_ID)}, {sql_text(row['id'])}, {sql_text(row['modifierId'])}, {row['difficulty']}, {row['minimumItemLevel']}, {row['maximumItemLevel']}, {row['minimumValue']}, {row['maximumValue']}, {sql_text(row['valueBasis'])}, {sql_text(row['evidenceState'])}, {sql_json(row)});")
    for row in catalog["virtues"]:
        lines.append(f"INSERT INTO core_game.rebuild_virtue_effect VALUES ({sql_text(RELEASE_ID)}, {sql_text(row['id'])}, {row['sourceId']}, {sql_text(row['nameEn'])}, {sql_text(row['nameVi'])}, {sql_text(row['descriptionEn'])}, {sql_text(row['descriptionVi'])}, {sql_json(row['thresholdValues'])}, {row['secondaryValue']}, {row['tertiaryValue']}, {sql_text(row['evidenceState'])}, {sql_json(row)});")
    for row in catalog["collectionSets"]:
        lines.append(f"INSERT INTO core_game.rebuild_collection_set VALUES ({sql_text(RELEASE_ID)}, {sql_text(row['id'])}, {row['sourceId']}, {sql_text(row['nameEn'])}, {sql_text(row['nameVi'])}, {sql_json(row['specialItemIds'])}, {row['optionType']}, {row['optionValue']}, {row['visible']}, {sql_text(row['effectState'])}, {sql_text(row['evidenceState'])}, {sql_json(row)});")
    for row in catalog["weapons"]:
        lines.append(f"INSERT INTO core_game.rebuild_weapon_base VALUES ({sql_text(RELEASE_ID)}, {sql_text(row['id'])}, {sql_text(row['classId'])}, {sql_text(row['className'])}, {sql_text(row['weaponFamily'])}, {row['difficulty']}, {row['unlockLevel']}, {row['baseLevelCap']}, {row['basePower']}, {row['capPower']}, {row['packageSecondValue']}, {row['packageSourceIndex']}, {sql_text(row['evidenceState'])}, {sql_bool(row['active'])}, {sql_json(row)});")
        for locale, display_name in row["localization"].items():
            lines.append(f"INSERT INTO core_game.rebuild_weapon_localization VALUES ({sql_text(RELEASE_ID)}, {sql_text(row['id'])}, {sql_text(locale)}, {sql_text(display_name)});")
        visual = row["visual"]
        lines.append(f"INSERT INTO core_game.rebuild_weapon_visual_binding VALUES ({sql_text(RELEASE_ID)}, {sql_text(row['id'])}, {sql_text(visual['family'])}, {sql_text(visual['spineSlot'])}, {sql_text(visual['attachment'])}, {sql_text(visual['theme'])}, {sql_text(visual['assetState'])}, {sql_text(visual['expectedInventoryIcon'])}, {sql_text(visual['expectedSpineAttachment'])}, {sql_json(visual)});")
    for row in catalog["weaponModifierPool"]:
        lines.append(f"INSERT INTO core_game.rebuild_weapon_affix_pool VALUES ({sql_text(RELEASE_ID)}, {sql_text(row['weaponClass'])}, {sql_text(row['modifierId'])}, {sql_text(row['slot'])}, {sql_text(row['family'])}, {sql_text(row['exclusiveGroup'])}, {row['weight']}, {row['minimumDifficulty']}, {row['maximumDifficulty']}, {sql_bool(row['active'])}, {sql_text(row['evidenceState'])});")

    lines += [
        "CREATE INDEX rebuild_weapon_base_lookup_idx ON core_game.rebuild_weapon_base(class_id, difficulty, unlock_level);",
        "CREATE INDEX rebuild_affix_lookup_idx ON core_game.rebuild_affix(slot_assignment, generation_state, source_id);",
        "CREATE INDEX rebuild_affix_tier_lookup_idx ON core_game.rebuild_affix_tier(affix_id, difficulty);",
        "CREATE INDEX rebuild_affix_pool_lookup_idx ON core_game.rebuild_weapon_affix_pool(weapon_class, slot, minimum_difficulty, maximum_difficulty);",
        "CREATE INDEX rebuild_collection_set_lookup_idx ON core_game.rebuild_collection_set(effect_state, source_id);",
        "DO $$ BEGIN IF (SELECT count(*) FROM core_game.rebuild_difficulty) <> 8 OR (SELECT count(*) FROM core_game.rebuild_item_rarity) <> 4 OR (SELECT count(*) FROM core_game.rebuild_weapon_base) <> 40 OR (SELECT count(*) FROM core_game.rebuild_weapon_localization) <> 80 OR (SELECT count(*) FROM core_game.rebuild_weapon_visual_binding) <> 40 OR (SELECT count(*) FROM core_game.rebuild_affix) <> 126 OR (SELECT count(*) FROM core_game.rebuild_affix_tier) <> 160 OR (SELECT count(*) FROM core_game.rebuild_weapon_affix_pool) <> 20 OR (SELECT count(*) FROM core_game.rebuild_virtue_effect) <> 5 OR (SELECT count(*) FROM core_game.rebuild_collection_set) <> 61 THEN RAISE EXCEPTION 'rebuild weapon core count mismatch'; END IF; END $$;",
    ]
    return "\n".join(lines) + "\n"


def validate(catalog: dict) -> None:
    if len(catalog["weapons"]) != 40:
        raise ValueError("weapon catalog must contain 40 bases")
    if len(catalog["modifiers"]) != 126 or len(catalog["virtues"]) != 5 or len(catalog["collectionSets"]) != 61:
        raise ValueError("mined modifier/set counts changed")
    weapon_ids = {row["id"] for row in catalog["weapons"]}
    if len(weapon_ids) != 40:
        raise ValueError("weapon IDs must be unique")
    for row in catalog["weapons"]:
        if set(row["localization"]) != {"en", "vi"}:
            raise ValueError(f"{row['id']} must have exactly en and vi localization")
        if row["baseLevelCap"] != min(row["unlockLevel"] + 100, 800):
            raise ValueError(f"invalid level cap for {row['id']}")
    if any(row["effectState"] != "unresolved" for row in catalog["collectionSets"]):
        raise ValueError("collection-set semantics cannot be enabled without evidence")
    active_pool = [row for row in catalog["weaponModifierPool"] if row["active"]]
    if len(active_pool) != 20:
        raise ValueError("weapon v1 requires 20 active affix families")
    if {row["slot"] for row in active_pool} != {"prefix", "suffix"}:
        raise ValueError("weapon pool must include prefix and suffix rows")
    tiers_by_modifier: dict[str, list[dict]] = {}
    for row in catalog["affixTiers"]:
        tiers_by_modifier.setdefault(row["modifierId"], []).append(row)
        if row["minimumValue"] > row["maximumValue"]:
            raise ValueError(f"invalid tier range for {row['id']}")
    if set(tiers_by_modifier) != {row["modifierId"] for row in active_pool}:
        raise ValueError("every active pool modifier must have tiers")
    if any([row["difficulty"] for row in tiers] != list(range(1, 9)) for tiers in tiers_by_modifier.values()):
        raise ValueError("every active modifier must have exactly one ordered tier per difficulty")


def main() -> None:
    catalog = build_catalog()
    validate(catalog)
    RELEASE_DIR.mkdir(parents=True, exist_ok=True)
    CATALOG_OUT.write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    SQL_OUT.write_text(build_sql(catalog), encoding="utf-8")
    print(f"wrote {CATALOG_OUT} ({CATALOG_OUT.stat().st_size} bytes)")
    print(f"wrote {SQL_OUT} ({SQL_OUT.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
