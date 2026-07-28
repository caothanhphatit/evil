#!/usr/bin/env python3
"""Classify ConstantData-backed Hunter getDamage caller arithmetic."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs


ROOT = Path(__file__).resolve().parents[1]
CALLERS = ROOT / "reverse-engineering/evidence/original-native-hunter-getdamage-callers-api35-v1.json"
ACTIONS = ROOT / "reverse-engineering/evidence/original-native-hunter-skill-coefficient-action-targets-api35-v1.json"
PASS9_TARGETS = ROOT / "reverse-engineering/evidence/original-native-hunter-damage-contract-targets-api35-v1.json"
CONSTANT_SCHEMA = ROOT / "reverse-engineering/evidence/constant-data-runtime-schema-api35-v1.json"
STATIC_FACTORS = ROOT / "reverse-engineering/evidence/original-runtime-status-data-static-factors-api35-v1.json"
OUTPUT = ROOT / "reverse-engineering/evidence/original-native-hunter-constant-coefficients-pass16.json"


def source(path: Path) -> dict:
    raw = path.read_bytes()
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "bytes": len(raw),
        "sha256": hashlib.sha256(raw).hexdigest(),
    }


def methods(path: Path) -> list[dict]:
    return json.loads(path.read_text())["record"]["payload"]["methods"]


def find(rows: list[dict], class_name: str, method_name: str) -> dict:
    return next(row for row in rows if row["className"] == class_name and row["methodName"] == method_name)


def decode(method: dict) -> tuple[bytes, list]:
    candidate = method["candidates"][0]
    raw = bytes.fromhex(candidate["codeHex"])
    if candidate["codeTruncated"] or candidate["nativeSizeBytes"] != len(raw):
        raise ValueError(f"incomplete exact body: {method['methodName']}")
    instructions = list(Cs(CS_ARCH_ARM64, CS_MODE_ARM).disasm(raw, int(candidate["moduleOffset"], 16)))
    return raw, instructions


def descriptor(method: dict) -> dict:
    raw, _ = decode(method)
    candidate = method["candidates"][0]
    return {
        "className": method["className"],
        "methodName": method["methodName"],
        "parameterTypes": method["parameterTypes"],
        "token": method["token"],
        "moduleOffset": candidate["moduleOffset"],
        "nativeSizeBytes": len(raw),
        "bodySha256": hashlib.sha256(raw).hexdigest(),
    }


def require(method: dict, anchors: list[tuple[int, str, str]]) -> None:
    _, instructions = decode(method)
    observed = {(row.address, row.mnemonic, row.op_str) for row in instructions}
    missing = [anchor for anchor in anchors if anchor not in observed]
    if missing:
        raise ValueError(f"native anchors changed for {method['methodName']}: {missing}")


def constant_fields(document: dict) -> dict[int, dict]:
    pending = [document]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            if value.get("name") == "ConstantData" and isinstance(value.get("fields"), list):
                return {field["offset"]: field for field in value["fields"]}
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)
    raise ValueError("ConstantData schema not found")


def build(callers_path: Path, actions_path: Path, pass9_targets_path: Path, constant_schema_path: Path, static_factors_path: Path) -> dict:
    caller_rows = methods(callers_path)
    action_rows = methods(actions_path)
    pass9_target_rows = methods(pass9_targets_path)
    constants = constant_fields(json.loads(constant_schema_path.read_text()))
    factors = json.loads(static_factors_path.read_text())
    percent = factors["monsterDamageFactors"]["integer_percent_scale"]
    if percent != {
        "moduleOffset": "0xd2ac8c",
        "rawHex": "0ad7233c",
        "float32": 0.009999999776482582,
    }:
        raise ValueError("captured integer percent scale changed")

    expected = {
        0x2820: ("POISON_AURA_POWER_VALUE", "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt"),
        0x2860: ("DARK_LIGHTNING_POWER_VALUE", "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt"),
        0x2B20: ("GEAR_PROP_25_VALUE", "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt"),
        0x2C78: ("UNIQUE_GEAR_THUNDER_DRAGON_FURY_PROPERTY_INDEX", "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt"),
        0x2C88: ("THUNDER_DRAGON_FURY_POWER_VALUE", "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt"),
        0x3000: ("FROST_ARCHER_SNIPING_SKILL_UP_VALUE", "CodeStage.AntiCheat.ObscuredTypes.ObscuredFloat"),
        0x3400: ("CURSEAURA_POWER_VALUE", "CodeStage.AntiCheat.ObscuredTypes.ObscuredFloat"),
        0x3450: ("GEAR_FROZEN_HEART_PROPERTY_INDEX", "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt"),
        0x34AC: ("FROZEN_HEART_SHADOW_STRIKE_SKILL_UP_VALUE", "CodeStage.AntiCheat.ObscuredTypes.ObscuredFloat"),
        0x34C0: ("FROZEN_HEART_SPIN_SPLASH_SKILL_UP_VALUE", "CodeStage.AntiCheat.ObscuredTypes.ObscuredFloat"),
        0x3598: ("UNIQUE_GEAR_TRUTHFUL_THUNDER_DRAGON_FURY_PROPERTY_INDEX", "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt"),
        0x35B0: ("TRUTHFUL_THUNDER_DRAGON_FURY_POWER_VALUE", "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt"),
        0x3750: ("JOB_TRAIT_BUFF_SHADOW_SKIN_INDEX", "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt"),
        0x3B50: ("DARK_KNIGHT_DAMAGE_TYPE_INDEX", "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt"),
    }
    for offset, (name, type_name) in expected.items():
        field = constants[offset]
        if field["name"] != name or field["type"] != type_name:
            raise ValueError(f"ConstantData field changed at 0x{offset:X}")

    specs = {
        "BPOGPFGALFD": [
            (0x3409504, "bl", "#0x33f51c4"), (0x34095B8, "ldr", "x0, [x8, #0x2820]"),
            (0x34095C8, "fadd", "s1, s8, s9"), (0x34095E4, "fmul", "s0, s1, s0"),
            (0x34095E8, "fmul", "s1, s2, s3"), (0x34095EC, "fmul", "s0, s0, s1"),
            (0x34095F8, "fcvtzs", "x9, s0"), (0x340968C, "mov", "w5, #2"),
        ],
        "PCGIDDENIJL": [
            (0x340D5D8, "bl", "#0x33f51c4"), (0x340D628, "ldr", "q0, [x8, #0x3400]"),
            (0x340D690, "fmul", "s0, s9, s0"), (0x340D694, "fmul", "s9, s0, s1"),
            (0x340D698, "fcvtzs", "x8, s9"), (0x340D6D8, "fmul", "s0, s2, s0"),
            (0x340D6DC, "fcvtzs", "x8, s0"), (0x340D774, "ldr", "x0, [x8, #0x3b50]"),
        ],
        "NMAIFPMMBHE": [
            (0x340D9C0, "bl", "#0x33f51c4"), (0x340DAD8, "ldr", "x0, [x8, #0x2b20]"),
            (0x340DB70, "ldr", "x0, [x8, #0x3450]"), (0x340DC08, "ldr", "q0, [x8, #0x34c0]"),
            (0x340DC70, "fmul", "s1, s1, s10"), (0x340DC74, "fmul", "s0, s1, s0"),
            (0x340DC80, "fmul", "s0, s0, s1"), (0x340DC8C, "fcvtzs", "x9, s0"),
            (0x340DE58, "bl", "#0x33e9d84"),
        ],
        "DNPJKKJPHLD": [
            (0x3419804, "bl", "#0x33f51c4"), (0x341991C, "ldr", "x0, [x8, #0x2b20]"),
            (0x34199B4, "ldr", "x0, [x8, #0x3450]"), (0x3419A50, "ldr", "q0, [x8, x9]"),
            (0x3419B9C, "ldr", "x0, [x8, #0x3750]"), (0x3419C60, "fmul", "s1, s1, s8"),
            (0x3419C64, "fmul", "s0, s1, s0"), (0x3419C6C, "fmul", "s0, s0, s1"),
            (0x3419C78, "fcvtzs", "x9, s0"), (0x3419CE8, "blr", "x9"),
        ],
        "NPIAALIFANE": [
            (0x34205BC, "bl", "#0x33f51c4"), (0x34206B4, "ldr", "x0, [x8, #0x2b20]"),
            (0x3420790, "ldr", "q0, [x8, #0x3000]"), (0x3420924, "fmul", "s0, s0, s9"),
            (0x3420930, "fadd", "s9, s0, s9"), (0x3420950, "fmul", "s0, s9, s0"),
            (0x3420954, "fmul", "s0, s0, s1"), (0x3420960, "fcvtzs", "x9, s0"),
            (0x3420B88, "bl", "#0x32ff6cc"),
        ],
        "EHKBOGAOFEN": [
            (0x3421588, "bl", "#0x33f51c4"), (0x34215B8, "ldr", "x22, [x8, #0x2860]"),
            (0x34215D8, "ldr", "x0, [x8, #0x3598]"), (0x3421650, "ldr", "x0, [x8, #0x35b0]"),
            (0x34216FC, "ldr", "x0, [x8, #0x2c78]"), (0x3421774, "ldr", "x0, [x8, #0x2c88]"),
            (0x34217FC, "add", "w8, w23, w22"), (0x3421800, "madd", "w0, w0, w24, w8"),
            (0x342184C, "mul", "x8, x24, x8"), (0x3421868, "fmul", "s0, s0, s1"),
            (0x342187C, "fcvtzs", "x10, s0"), (0x3421940, "bl", "#0x2c572b0"),
        ],
    }
    methods_by_name = {}
    for method_name, anchors in specs.items():
        method = find(caller_rows, "HunterCtrl", method_name)
        require(method, anchors)
        methods_by_name[method_name] = descriptor(method)

    flame = descriptor(find(action_rows, "FlameExplosionCtrl", "Action"))
    blizzard = descriptor(find(pass9_target_rows, "BlizzardCtrl", "Action"))

    def fields(*offsets: int) -> list[dict]:
        return [{**constants[offset], "runtimeDataOffset": f"0x{offset:X}"} for offset in offsets]

    return {
        "schemaVersion": 1,
        "contractType": "original-native-hunter-constant-coefficients-pass16",
        "runtimeCompatibility": "evidence-only-disconnected",
        "sources": [source(callers_path), source(actions_path), source(pass9_targets_path), source(constant_schema_path), source(static_factors_path)],
        "integerPercentScale": percent,
        "methods": [
            {
                "method": methods_by_name["BPOGPFGALFD"],
                "getDamageVector": [True, False, True],
                "constantFields": fields(0x2820),
                "equation": "damage0 = trunc_i64((modifierA + modifierB) * float32(baseDamage) * (float32(decode(POISON_AURA_POWER_VALUE)) * 0.01f))",
                "postBranch": "when a decoded target-side discriminator equals 2, damage = trunc_i64(float32(damage0) * modifierC)",
                "route": {"target": "EvilCtrl virtual slot +0x2A8", "damageParameter": 2, "parameter5": 2},
            },
            {
                "method": methods_by_name["PCGIDDENIJL"],
                "getDamageVector": [True, False, True],
                "constantFields": fields(0x3400, 0x3B50),
                "equation": "scaledBase = trunc_i64(float32(baseDamage) * decode(CURSEAURA_POWER_VALUE) * 0.01f); damage0 = trunc_i64(float32(scaledBase) * (modifierA + modifierB))",
                "postBranch": "when a decoded target-side discriminator equals 2, damage = trunc_i64(float32(damage0) * modifierC)",
                "route": {"target": "EvilCtrl virtual slot +0x2A8", "damageParameter": 2, "parameter5Source": "DARK_KNIGHT_DAMAGE_TYPE_INDEX"},
            },
            {
                "method": methods_by_name["NMAIFPMMBHE"],
                "getDamageVector": [True, False, True],
                "constantFields": fields(0x2B20, 0x3450, 0x34C0),
                "equation": "damage = trunc_i64(float32(baseDamageAfterOptionalIntegerScale) * parameter1 * decode(dynamicCoefficient) * 0.01f)",
                "confirmedCoefficientMutation": "a gated Frozen Heart branch adds FROZEN_HEART_SPIN_SPLASH_SKILL_UP_VALUE to dynamicCoefficient",
                "integerScaleBranch": "a separate gated branch replaces the decoded getDamage snapshot with baseDamage * decode(GEAR_PROP_25_VALUE) before later float32 arithmetic",
                "route": {"target": "BlizzardCtrl.Action", "damageParameter": 6},
            },
            {
                "method": methods_by_name["DNPJKKJPHLD"],
                "getDamageVector": [True, False, False],
                "constantFields": fields(0x2B20, 0x3450, 0x34AC, 0x3750, 0x3B50),
                "equation": "damage = trunc_i64(float32(baseDamageAfterOptionalIntegerScale) * parameter2 * decode(dynamicCoefficient) * 0.01f)",
                "confirmedCoefficientMutation": "a gated Frozen Heart branch adds FROZEN_HEART_SHADOW_STRIKE_SKILL_UP_VALUE; a separate Shadow Skin indexed branch contributes another decoded float through an unresolved helper path",
                "integerScaleBranch": "a separate gated branch replaces the decoded getDamage snapshot with baseDamage * decode(GEAR_PROP_25_VALUE) before later float32 arithmetic",
                "route": {"target": "EvilCtrl virtual slot +0x2A8", "damageParameter": 2, "parameter5Source": "DARK_KNIGHT_DAMAGE_TYPE_INDEX"},
            },
            {
                "method": methods_by_name["NPIAALIFANE"],
                "getDamageVector": [True, False, False],
                "constantFields": fields(0x2B20, 0x3000),
                "equation": "effectiveParameter1 = parameter1 * (1.0f + decode(dynamicCoefficient)); damage = trunc_i64(float32(baseDamage) * effectiveParameter1 * 0.01f)",
                "confirmedCoefficientMutation": "a gated branch adds FROST_ARCHER_SNIPING_SKILL_UP_VALUE to dynamicCoefficient",
                "route": {"targetModuleOffset": "0x32ff6cc", "managedIdentity": None, "damageArgumentRegister": "x4"},
            },
            {
                "method": methods_by_name["EHKBOGAOFEN"],
                "getDamageVector": [True, False, True],
                "constantFields": fields(0x2860, 0x2C78, 0x2C88, 0x3598, 0x35B0),
                "equation": "combinedPower = basePower + selectedPower + selectedPower * selectedPropertyValue; damage = trunc_i64(float32(baseDamage * combinedPower) * 0.01f)",
                "selectionBoundary": "native branches select the regular Thunder Dragon Fury or Truthful Thunder Dragon Fury property/power pair; product-facing branch precedence is not inferred beyond the captured control flow",
                "route": {"target": "FlameExplosionCtrl.Action", "damageParameter": 4, "selectorParameter6": 2},
            },
        ],
        "resolvedActionTargets": {"flameExplosion": flame, "blizzard": blizzard},
        "classification": {
            "reviewedExactCallerBodies": 6,
            "status": "constant bindings and final arithmetic classified; several gate/helper meanings remain unresolved",
            "coveragePolicy": "do not subtract all six from the remaining fully resolved caller count",
        },
        "unresolved": [
            "managed identity and full parameter contract of module target 0x32ff6cc",
            "managed identity of EvilCtrl virtual slot +0x2A8",
            "semantic identities of modifierA/modifierB/modifierC and several branch gates",
            "the helper-fed Shadow Skin coefficient value in DNPJKKJPHLD",
            "public skill-row mappings for all six obfuscated HunterCtrl methods",
        ],
        "integrationStatus": "disconnected_no_live_combat_use",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--callers", type=Path, default=CALLERS)
    parser.add_argument("--actions", type=Path, default=ACTIONS)
    parser.add_argument("--pass9-targets", type=Path, default=PASS9_TARGETS)
    parser.add_argument("--constant-schema", type=Path, default=CONSTANT_SCHEMA)
    parser.add_argument("--static-factors", type=Path, default=STATIC_FACTORS)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    args.output.write_text(json.dumps(build(args.callers, args.actions, args.pass9_targets, args.constant_schema, args.static_factors), ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote Hunter ConstantData coefficient Pass16 evidence to {args.output}")


if __name__ == "__main__":
    main()
