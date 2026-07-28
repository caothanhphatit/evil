#!/usr/bin/env python3
"""Normalize the proven native hit/miss boundary without assigning guessed labels."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "reverse-engineering/evidence/original-native-hit-miss-pass12.json"
SOURCES = {
    "hunterCallers": (
        ROOT / "reverse-engineering/evidence/original-native-hunter-getdamage-callers-api35-v1.json",
        "0ed043c4aa358d50ef237133c700abe10935e23bf0cb159690bea26957a67170",
    ),
    "combatMethods": (
        ROOT / "reverse-engineering/evidence/original-native-combat-formula-exact-boundaries-api35-v1.json",
        "0bdf88cfc1874aea8c45b7adcfbf789c602d577b79cf9b62ecd85c67346f5c80",
    ),
    "hunterIntake": (
        ROOT / "reverse-engineering/evidence/original-native-hunter-damage-intake-api35-v1.json",
        "9c48d4c82a61f2549ec7bd1b4cb6b7ae7ab45f98273fe1c03a81d053c28591e8",
    ),
    "evilMethods": (
        ROOT / "reverse-engineering/evidence/original-native-evilctrl-all-methods-api35-v1.json",
        "0095758226878134fccced21c97b7cb0432689ac97e6006108cccd382ffdfa71",
    ),
    "statusSchema": (
        ROOT / "reverse-engineering/evidence/status-data-runtime-schema-android-api35-v1.json",
        "6f6c2394ffaffb5a85fca0239c459434ad98ac7e70a54d1b27f54fe6ae0adda0",
    ),
    "evilSchema": (
        ROOT / "reverse-engineering/evidence/evil-data-runtime-schema-api35-v1.json",
        "ed7895541e98bc2665321ca2c894f8198d76765d5accd3a0d3479bdb6e1560fc",
    ),
}

RANGE_INT = "#0x5a76240"
GET_DAMAGE = "#0x33f51c4"
GET_STATUS = "#0x33f7900"
GET_CRITICAL_DAMAGE = "#0x33f97a8"
HUNTER_DAMAGED = "#0x346514c"


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


def disassemble(method: dict[str, Any]) -> tuple[dict[str, Any], list[Any]]:
    candidate = method["candidates"][0]
    body = bytes.fromhex(candidate["codeHex"])
    if candidate.get("codeTruncated") or len(body) != candidate["nativeSizeBytes"]:
        raise ValueError(f"incomplete body: {method['className']}.{method['methodName']}")
    decoder = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    return candidate, list(decoder.disasm(body, int(candidate["moduleOffset"], 16)))


def require(instructions: list[Any], anchors: list[tuple[int, str, str]]) -> None:
    indexed = {(item.address, item.mnemonic, item.op_str) for item in instructions}
    missing = [anchor for anchor in anchors if anchor not in indexed]
    if missing:
        raise ValueError(f"native anchors changed: {missing}")


def call_sites(instructions: list[Any], target: str) -> list[int]:
    return [
        item.address
        for item in instructions
        if item.mnemonic == "bl" and item.op_str == target
    ]


def class_fields(payload: dict[str, Any], class_name: str) -> list[dict[str, Any]]:
    return next(
        row["fields"]
        for row in payload["record"]["payload"]["classes"]
        if row["name"] == class_name
    )


def descriptor(method: dict[str, Any], candidate: dict[str, Any]) -> dict[str, Any]:
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
    hunting = find_method(data["hunterCallers"], "HunterCtrl", "HuntingAttackAction")
    get_damage = find_method(data["combatMethods"], "HunterCtrl", "getDamage")
    hunter_damaged = find_method(data["hunterIntake"], "HunterCtrl", "Damaged")
    fixed_update = find_method(data["evilMethods"], "EvilCtrl", "FixedUpdate")
    evil_attack = find_method(data["evilMethods"], "EvilCtrl", "EBNOJHOGEMM")
    evil_alt_attack = find_method(data["evilMethods"], "EvilCtrl", "OFEIPNBMNML")

    hunting_candidate, hunting_ins = disassemble(hunting)
    get_damage_candidate, get_damage_ins = disassemble(get_damage)
    hunter_damaged_candidate, hunter_damaged_ins = disassemble(hunter_damaged)
    fixed_candidate, fixed_ins = disassemble(fixed_update)
    evil_attack_candidate, evil_attack_ins = disassemble(evil_attack)
    evil_alt_candidate, evil_alt_ins = disassemble(evil_alt_attack)

    require(
        hunting_ins,
        [
            (0x34173C8, "mov", "w1, wzr"),
            (0x34173CC, "mov", "w2, wzr"),
            (0x34173D0, "mov", "w3, wzr"),
            (0x34173D4, "bl", GET_DAMAGE),
        ],
    )
    hunting_rng = call_sites(hunting_ins, RANGE_INT)
    if hunting_rng != [0x3417044, 0x3417154, 0x3417600, 0x3417758, 0x3418540]:
        raise ValueError(f"HuntingAttackAction RNG inventory changed: {hunting_rng}")

    require(
        get_damage_ins,
        [
            (0x33F6BB8, "bl", GET_STATUS),
            (0x33F6BC0, "ldp", "x8, x1, [x0, #0xb0]"),
            (0x33F6BDC, "mov", "w22, w0"),
            (0x33F6C10, "add", "w9, w0, w22"),
            (0x33F6C18, "cmp", "w9, #0x64"),
            (0x33F6C1C, "csel", "w22, w9, w8, lt"),
            (0x33F6C20, "mov", "w0, wzr"),
            (0x33F6C24, "mov", "w1, #0x64"),
            (0x33F6C2C, "bl", RANGE_INT),
            (0x33F6C30, "cmp", "w0, w22"),
            (0x33F6C34, "b.lt", "#0x33f6c7c"),
            (0x33F6D60, "bl", GET_CRITICAL_DAMAGE),
            (0x33F6D84, "mov", "w0, #2"),
        ],
    )
    if call_sites(get_damage_ins, RANGE_INT) != [0x33F6C2C]:
        raise ValueError("getDamage integer RNG inventory changed")

    require(
        fixed_ins,
        [(0x2F29B7C, "bl", "#0x2f29c74")],
    )
    require(
        evil_attack_ins,
        [
            (0x2F29F68, "ldr", "w8, [x19, #0x1e8]"),
            (0x2F29F70, "cmp", "w8, #1"),
            (0x2F29F7C, "b.lt", "#0x2f2a000"),
            (0x2F29F80, "mov", "w0, wzr"),
            (0x2F29F84, "mov", "w1, #0x64"),
            (0x2F29F8C, "bl", RANGE_INT),
            (0x2F29F90, "ldr", "w8, [x19, #0x1e8]"),
            (0x2F29F94, "cmp", "w0, w8"),
            (0x2F29F98, "b.ge", "#0x2f2a038"),
            (0x2F29FFC, "b", "#0x2fcba7c"),
            (0x2F2A0A8, "bl", HUNTER_DAMAGED),
        ],
    )
    if call_sites(evil_attack_ins, RANGE_INT) != [0x2F29F8C]:
        raise ValueError("Evil attack RNG inventory changed")
    if call_sites(evil_alt_ins, RANGE_INT):
        raise ValueError("alternate Evil direct damage caller gained integer RNG")
    if call_sites(evil_alt_ins, HUNTER_DAMAGED) != [0x2F1AD7C, 0x2F1AE10]:
        raise ValueError("alternate Evil direct damage call sites changed")

    hunter_damaged_rng = call_sites(hunter_damaged_ins, RANGE_INT)
    if hunter_damaged_rng != [0x34653C8, 0x34667B4, 0x3469508, 0x346959C]:
        raise ValueError(f"HunterCtrl.Damaged RNG inventory changed: {hunter_damaged_rng}")

    status_fields = class_fields(data["statusSchema"], "StatusData")
    by_name = {field["name"]: field for field in status_fields}
    if by_name["<CalcCritical>k__BackingField"]["offset"] != 176:
        raise ValueError("CalcCritical offset changed")
    if by_name["<CalcDodge>k__BackingField"]["offset"] != 192:
        raise ValueError("CalcDodge offset changed")
    status_accuracy_fields = [
        field["name"]
        for field in status_fields
        if any(term in field["name"].lower() for term in ("accuracy", "<acc>", "<hit>"))
    ]
    evil_fields = class_fields(data["evilSchema"], "EvilData")
    evil_evasion_fields = [
        field["name"]
        for field in evil_fields
        if any(term in field["name"].lower() for term in ("dodge", "evasion", "accuracy", "<acc>", "<hit>"))
    ]
    if status_accuracy_fields or evil_evasion_fields:
        raise ValueError("accuracy/evasion schema boundary changed")

    return {
        "schemaVersion": 1,
        "contractType": "original-native-hit-miss-pass12",
        "runtimeCompatibility": "evidence-only-disconnected",
        "sources": [
            {
                "name": name,
                "path": path.relative_to(ROOT).as_posix(),
                "sha256": expected_sha,
            }
            for name, (path, expected_sha) in SOURCES.items()
        ],
        "methods": [
            descriptor(method, candidate)
            for method, candidate in (
                (hunting, hunting_candidate),
                (get_damage, get_damage_candidate),
                (fixed_update, fixed_candidate),
                (evil_attack, evil_attack_candidate),
                (evil_alt_attack, evil_alt_candidate),
                (hunter_damaged, hunter_damaged_candidate),
            )
        ],
        "hunterAttacksEvil": {
            "basicAttackEntry": "HuntingAttackAction calls getDamage(false,false,false) at 0x34173D4.",
            "getDamageIntegerRng": {
                "callSite": "0x33f6c2c",
                "range": "[0,100)",
                "threshold": "min(100, StatusData.CalcCritical + gated HunterCtrl field 0x90C)",
                "comparison": "roll < threshold",
                "identity": "critical selection, proven by the CalcCritical@0xB0 load and getCriticalDamage call",
                "notHitMiss": "the selected path changes the result discriminator to 2; it does not abort outgoing damage construction",
            },
            "basicActionRngInventory": [
                {"callSite": "0x3417044", "range": "[0,101)", "sourceRoot": "HunterCtrl fields", "classification": "attacker-owned proc"},
                {"callSite": "0x3417154", "range": "[0,101)", "sourceRoot": "HunterData+0x2A4 and DataManager table", "classification": "attacker-owned proc"},
                {"callSite": "0x3417600", "range": "[0,100)", "sourceRoot": "StatusData.GearProperty row 25", "classification": "attacker-owned gear proc"},
                {"callSite": "0x3417758", "range": "[0,100)", "sourceRoot": "StatusData.GearSetPropertyValue row 64", "classification": "attacker-owned gear-set proc"},
                {"callSite": "0x3418540", "range": "[0,101)", "sourceRoot": "HunterData+0x294 and DataManager table", "classification": "attacker-owned proc"},
            ],
            "schemaBoundary": {
                "statusAccuracyFields": status_accuracy_fields,
                "evilEvasionFields": evil_evasion_fields,
            },
            "conclusion": "No accuracy-versus-Evil-evasion RNG is present in the captured basic-action/getDamage construction path. Target delivery remains separately unresolved, so this is not proof that every possible projectile or skill always hits.",
        },
        "evilAttacksHunter": {
            "directChain": "EvilCtrl.FixedUpdate -> EvilCtrl.EBNOJHOGEMM -> HunterCtrl.Damaged",
            "preDamageGate": {
                "owner": "EvilCtrl",
                "field": "OCLFGGEJKMI@0x1E8",
                "enable": "field >= 1",
                "range": "[0,100)",
                "comparison": "roll < field",
                "procOrdering": "proc branches to DamageManager.Show and returns before HunterCtrl.Damaged; non-proc continues to HunterCtrl.Damaged",
                "semanticBoundary": "BuffSetting effect type 54 writes this field, but native evidence does not name the gameplay effect accuracy, blind, miss, dodge, or evasion.",
            },
            "alternateDirectCaller": "EvilCtrl.OFEIPNBMNML contains two HunterCtrl.Damaged calls and no integer Random.Range call.",
            "hunterDamagedRng": {
                "callSites": [f"0x{site:x}" for site in hunter_damaged_rng],
                "classification": "post-entry Hunter gear/effect procs; none is a direct CalcDodge read",
            },
            "calcDodge": {
                "field": "StatusData.CalcDodge@0xC0",
                "directReadInCapturedChain": False,
                "globalConsumerStatus": "unresolved",
            },
            "conclusion": "The captured direct Evil attack chain has an attacker-owned effect-54 abort gate, not a proven Hunter CalcDodge roll. A product-facing miss/dodge label would be semantic guessing.",
        },
        "integrationStatus": "do_not_add_accuracy_or_dodge_formula_until_a_target_delivery_or_CalcDodge_consumer_is_proven",
        "unresolved": [
            "global consumer and exact formula for StatusData.CalcDodge",
            "product-facing name of Evil effect type 54",
            "all projectile, effect and skill target-delivery paths after getDamage",
            "whether any indirect or target-specific path outside the captured direct chains performs a hit test",
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    result = build()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote hit/miss pass 12 evidence to {args.output}")


if __name__ == "__main__":
    main()
