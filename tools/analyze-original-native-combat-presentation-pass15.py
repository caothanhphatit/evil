#!/usr/bin/env python3
"""Normalize the proven v1.411 combat text and motion presentation contract."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "reverse-engineering/evidence/original-native-combat-presentation-pass15.json"
SOURCES = {
    "schema": (
        ROOT / "reverse-engineering/evidence/original-native-combat-presentation-schema-api35-pass15.json",
        "eac84be1e9285fdc20a2f9f1e7b900fd39b049886acd59c6cc966e24bd5340d8",
    ),
    "methods": (
        ROOT / "reverse-engineering/evidence/original-native-combat-presentation-methods-api35-pass15.json",
        "7e4e2cafc48e1adbd5fad254457af5461ba658fe2b2e5cf78ff9bd5cb7fd574c",
    ),
    "runtime": (
        ROOT / "reverse-engineering/evidence/original-native-combat-presentation-runtime-api35-pass15.json",
        "4ddfed298cfb6aa45b25cc9a558aee8711066bc2be3898ce971b990fdd641f77",
    ),
    "hitMiss": (
        ROOT / "reverse-engineering/evidence/original-native-hit-miss-pass12.json",
        "d32ab1b91e658d6907314c23f75855708085dd366061d73f012a45c53e09823f",
    ),
    "hunterIntake": (
        ROOT / "reverse-engineering/evidence/original-native-hunter-damage-intake-api35-v1.json",
        "9c48d4c82a61f2549ec7bd1b4cb6b7ae7ab45f98273fe1c03a81d053c28591e8",
    ),
    "evilMethods": (
        ROOT / "reverse-engineering/evidence/original-native-evilctrl-all-methods-api35-v1.json",
        "0095758226878134fccced21c97b7cb0432689ac97e6006108cccd382ffdfa71",
    ),
}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_sources() -> dict[str, dict[str, Any]]:
    loaded = {}
    for name, (path, expected_sha) in SOURCES.items():
        actual_sha = digest(path)
        if actual_sha != expected_sha:
            raise ValueError(f"source changed: {path} ({actual_sha})")
        loaded[name] = json.loads(path.read_text())
    return loaded


def find_method(payload: dict[str, Any], class_name: str, method_name: str) -> dict[str, Any]:
    return next(
        method
        for method in payload["record"]["payload"]["methods"]
        if method["className"] == class_name and method["methodName"] == method_name
    )


def disassemble_method(method: dict[str, Any]) -> tuple[dict[str, Any], list[Any]]:
    candidate = method["candidates"][0]
    body = bytes.fromhex(candidate["codeHex"])
    if candidate.get("codeTruncated") or len(body) != candidate["nativeSizeBytes"]:
        raise ValueError(f"incomplete body: {method['className']}.{method['methodName']}")
    decoder = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    return candidate, list(decoder.disasm(body, int(candidate["moduleOffset"], 16)))


def disassemble_body(record: dict[str, Any]) -> list[Any]:
    body = bytes.fromhex(record["codeHex"])
    if len(body) != record["nativeSizeBytes"]:
        raise ValueError("incomplete iterator body")
    decoder = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    return list(decoder.disasm(body, int(record["moduleOffset"], 16)))


def require(instructions: list[Any], anchors: list[tuple[int, str, str]]) -> None:
    indexed = {(item.address, item.mnemonic, item.op_str) for item in instructions}
    missing = [anchor for anchor in anchors if anchor not in indexed]
    if missing:
        raise ValueError(f"native anchors changed: {missing}")


def class_fields(payload: dict[str, Any], class_name: str) -> dict[str, int]:
    row = next(row for row in payload["record"]["payload"]["classes"] if row["name"] == class_name)
    return {field["name"]: field["offset"] for field in row["fields"]}


def method_descriptor(method: dict[str, Any], candidate: dict[str, Any]) -> dict[str, Any]:
    body = bytes.fromhex(candidate["codeHex"])
    return {
        "className": method["className"],
        "methodName": method["methodName"],
        "token": method["token"],
        "moduleOffset": candidate["moduleOffset"],
        "nativeSizeBytes": len(body),
        "bodySha256": hashlib.sha256(body).hexdigest(),
    }


def build() -> dict[str, Any]:
    data = load_sources()
    schema = data["schema"]
    runtime = data["runtime"]
    show = find_method(data["methods"], "DamageCtrl", "Show")
    manager_show = find_method(data["methods"], "DamageManager", "Show")
    effect_show = find_method(data["methods"], "DamageEffectCtrl", "Show")
    effect_fixed = find_method(data["methods"], "DamageEffectCtrl", "FixedUpdate")
    hunter_damaged = find_method(data["hunterIntake"], "HunterCtrl", "Damaged")
    evil_attack = find_method(data["evilMethods"], "EvilCtrl", "EBNOJHOGEMM")

    show_candidate, show_ins = disassemble_method(show)
    manager_candidate, manager_ins = disassemble_method(manager_show)
    effect_show_candidate, effect_show_ins = disassemble_method(effect_show)
    effect_fixed_candidate, effect_fixed_ins = disassemble_method(effect_fixed)
    hunter_candidate, hunter_ins = disassemble_method(hunter_damaged)
    evil_candidate, evil_ins = disassemble_method(evil_attack)
    iterator_record = runtime["nestedIterator"]["moveNext"]
    iterator_ins = disassemble_body(iterator_record)

    require(
        show_ins,
        [
            (0x31D4014, "cmp", "w8, #0x11"),
            (0x31D4030, "br", "x10"),
            (0x31D4060, "cmp", "w8, #2"),
            (0x31D414C, "ldr", "x19, [x19, #0x20]"),
            (0x31D4410, "ldr", "x19, [x19, #0x20]"),
            (0x31D3F8C, "mov", "w8, #0x42700000"),
            (0x31D3FA0, "fadd", "s1, s1, s8"),
            (0x31D3FB0, "fadd", "s0, s0, s8"),
        ],
    )
    require(
        manager_ins,
        [
            (0x2FCBBC0, "fmov", "s0, #1.00000000"),
            (0x2FCBC18, "bl", "#0x31d3c5c"),
            (0x2FCBC24, "bl", "#0x2fcab04"),
            (0x2FCBC4C, "b", "#0x5a8043c"),
        ],
    )
    require(
        hunter_ins,
        [
            (0x3468EE4, "mov", "w1, wzr"),
            (0x3468EF4, "csel", "x2, x8, x9, eq"),
            (0x3468FFC, "mov", "w2, #1"),
            (0x346900C, "bl", "#0x2fcba7c"),
            (0x3465514, "mov", "w1, #0xf"),
            (0x3465528, "bl", "#0x2fcba7c"),
        ],
    )
    require(
        evil_ins,
        [
            (0x2F29FD8, "mov", "w1, #0x10"),
            (0x2F29FE0, "mov", "x2, xzr"),
            (0x2F29FEC, "mov", "w3, wzr"),
            (0x2F29FF0, "mov", "w4, wzr"),
            (0x2F29FFC, "b", "#0x2fcba7c"),
        ],
    )
    require(
        iterator_ins,
        [
            (0x2FCDB20, "ldr", "s8, [x8, #0x34]"),
            (0x2FCDB30, "fmov", "s0, w8"),
            (0x2FCDCB4, "fmov", "s0, #20.00000000"),
            (0x2FCDDA4, "fmov", "s0, #15.00000000"),
            (0x2FCDE18, "fmov", "s0, #5.00000000"),
            (0x2FCDDF8, "mov", "w8, #0x42a00000"),
            (0x2FCDE6C, "mov", "w8, #0x42f00000"),
            (0x2FCDE74, "fmul", "s0, s0, s1"),
            (0x2FCDEA4, "bl", "#0x5a8c890"),
            (0x2FCDF04, "bl", "#0x5a88d04"),
            (0x2FCDF20, "bl", "#0x5a8c968"),
            (0x2FCDD68, "ldr", "x8, [x21, #0x50]"),
            (0x2FCDC50, "bl", "#0x3298f98"),
        ],
    )
    require(
        effect_show_ins,
        [
            (0x2FC085C, "mov", "w1, #1"),
            (0x2FC0864, "bl", "#0x5a7e170"),
            (0x2FC089C, "b", "#0x5a8b5a8"),
        ],
    )
    require(
        effect_fixed_ins,
        [
            (0x2FC0574, "bl", "#0x5ada8d4"),
            (0x2FC05E0, "b", "#0x3298f98"),
        ],
    )

    damage_fields = class_fields(schema, "DamageCtrl")
    manager_fields = class_fields(schema, "DamageManager")
    if damage_fields != {
        "DamageText": 32,
        "Rect": 40,
        "NowPos": 48,
        "Type": 56,
        "IsPvPShow": 60,
        "IsAdventureShow": 61,
        "IsGuildBattleShow": 62,
        "IsDamageTestShow": 63,
        "IsWorldBossShow": 64,
        "IsFallenPastureShow": 65,
    }:
        raise ValueError("DamageCtrl field contract changed")
    if manager_fields["NJPPNDLJOPC"] != 80:
        raise ValueError("DamageManager WaitForFixedUpdate offset changed")

    strings = {row["slot"]: row["value"] for row in runtime["runtimeStrings"]}
    localized = {row["key"]: row["english"] for row in runtime["localization"]}
    critical = data["hitMiss"]["hunterAttacksEvil"]["getDamageIntegerRng"]
    if "discriminator to 2" not in critical["notHitMiss"]:
        raise ValueError("critical discriminator evidence changed")

    type_map = {
        "0": {"role": "incoming_damage", "text": "<color='#DE3232'>{damage}</color>", "provenBy": "HunterCtrl.Damaged calls Show(type=0) with computed damage or the one-damage clamp."},
        "1": {"role": "outgoing_normal_damage", "text": "<color='#AF70E0'>{damage}</color>", "provenBy": "getDamage baseline discriminator is 1 and DamageCtrl stores/dispatches it unchanged."},
        "2": {"role": "outgoing_critical_damage", "text": "<size=20><color='#FFD228'>CRIT</color></size>\\n<color='#AF70E0'>{damage}</color>", "provenBy": "getDamage critical branch sets discriminator 2; Show type 2 alone prepends status_6/CRIT."},
        "3": {"role": "evade", "text": "<color='#81F7F3'>Evade</color>", "key": "damagectrl_0"},
        "4": {"role": "experience_gain", "textFragments": [strings["0x601fb30"], localized["ruschnlocalize_1"], strings["0x6031d20"]]},
        "5": {"role": "purchase_item_variant", "color": "#ed1c24", "label": localized["damagectrl_4"]},
        "6": {"role": "purchase_item_variant", "color": "#43c552", "label": localized["damagectrl_4"]},
        "7": {"role": "purchase_item_variant", "color": "#b41ced", "label": localized["damagectrl_4"]},
        "8": {"role": "purchase_item_variant", "color": "#1c7bed", "label": localized["damagectrl_4"]},
        "9": {"role": "purchase_item_variant", "color": "#ed891c", "label": localized["damagectrl_4"]},
        "10": {"role": "element_gain", "label": localized["ruschnlocalize_17"], "color": "#A7FAEB"},
        "11": {"role": "numeric_variant", "color": "#997C8A", "publicLabel": None},
        "12": {"role": "penalty", "label": localized["damagectrl_1"], "colors": ["#FFFFFF", "#9F81F7", "#C3C3C3"]},
        "13": {"role": "lifesteal", "label": localized["damagectrl_2"], "colors": ["#DE3232", "#E17366"]},
        "14": {"role": "soul_gain", "label": localized["ruschnlocalize_16"], "colors": ["#FFFFFF", "#a07be2"]},
        "15": {"role": "invulnerable", "text": "<color='#8c82fa'>Invulnerable</color>", "key": "damagectrl_3"},
        "16": {"role": "miss", "text": "<color='#D43D3D'>Miss</color>", "key": "damagectrl_5", "provenBy": "effect-54 abort calls Show(16,0,position,0,false) before HunterCtrl.Damaged."},
        "17": {"role": "recovery_percent", "text": "<color='#c3dbc3'><color='#FFFFFF'>+{damage/100:f2}%</color> Recovery</color>", "key": "damagectrl_6"},
    }

    movement_seconds = 5 / 20 + 10 / 120 + 5 / 80 + 15 / 20
    return {
        "schemaVersion": 1,
        "contractType": "original-native-combat-presentation-pass15",
        "runtimeCompatibility": "evidence-only-disconnected",
        "sources": [
            {"name": name, "path": path.relative_to(ROOT).as_posix(), "sha256": expected_sha}
            for name, (path, expected_sha) in SOURCES.items()
        ],
        "methods": [
            method_descriptor(method, candidate)
            for method, candidate in (
                (manager_show, manager_candidate),
                (show, show_candidate),
                (effect_show, effect_show_candidate),
                (effect_fixed, effect_fixed_candidate),
                (hunter_damaged, hunter_candidate),
                (evil_attack, evil_candidate),
            )
        ],
        "damageCtrl": {
            "fields": damage_fields,
            "typeDispatch": runtime["damageCtrlShowJumpTable"],
            "types": type_map,
            "position": {"inputFlagFalse": "world-to-canvas position unchanged", "inputFlagTrue": "adds exactly 60 to canvas y and stored NowPos.y"},
            "initialLocalScale": [1.0, 1.0, 1.0],
        },
        "damageManager": {
            "instanceModel": "instantiates one DamageCtrl prefab per accepted Show call and starts one independent coroutine; no merge/coalesce path is present",
            "coroutine": {
                "iteratorClass": runtime["nestedIterator"]["className"],
                "yield": "DamageManager.NJPPNDLJOPC@0x50 (WaitForFixedUpdate)",
                "verticalSegments": [
                    {"fromOffsetY": 0.0, "toOffsetY": 5.0, "speedPerSecond": 20.0},
                    {"fromOffsetY": 5.0, "toOffsetY": 15.0, "speedPerSecond": 120.0},
                    {"fromOffsetY": 15.0, "toOffsetY": 20.0, "speedPerSecond": 80.0},
                    {"fromOffsetY": 20.0, "toOffsetY": 35.0, "speedPerSecond": 20.0},
                ],
                "continuousIdealDurationSeconds": movement_seconds,
                "actualDurationBoundary": "frame-quantized by WaitForFixedUpdate; ends after localPosition.y passes NowPos.y + 35",
                "scale": "while localScale.x > 0.4, subtract deltaTime/3 from localScale.x and y, write z=0",
                "completion": "PoolingSystem.DestroyList(gameObject, pool prefab)",
            },
        },
        "damageEffectCtrl": {
            "show": "activates its GameObject and sets transform.localPosition to (0,0.15,0)",
            "lifetime": "FixedUpdate returns while ParticleSystem is active; once ParticleSystem reports stopped it returns the GameObject through PoolingSystem.DestroyList",
        },
        "prefab": runtime["prefabAsset"],
        "dodgeAsset": runtime["dodgeAsset"],
        "semanticBoundary": {
            "evade": "DamageCtrl type 3 localized text; separate DodgeMent sprite clip is an asset-level dodge presentation.",
            "miss": "DamageCtrl type 16, used by the captured effect-54 pre-damage abort.",
            "calcDodgeFormula": "still unresolved; presentation labels do not prove the producer formula.",
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    result = build()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote combat presentation pass 15 evidence to {args.output}")


if __name__ == "__main__":
    main()
