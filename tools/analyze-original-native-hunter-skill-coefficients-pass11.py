#!/usr/bin/env python3
"""Normalize exact Hunter skill coefficient families without public-name guesses."""

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
OUTPUT = ROOT / "reverse-engineering/evidence/original-native-hunter-skill-coefficients-pass11.json"


def load_methods(path: Path) -> list[dict]:
    return json.loads(path.read_text())["record"]["payload"]["methods"]


def source(path: Path) -> dict:
    raw = path.read_bytes()
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "bytes": len(raw),
        "sha256": hashlib.sha256(raw).hexdigest(),
    }


def find(rows: list[dict], class_name: str, method_name: str) -> dict:
    return next(
        row
        for row in rows
        if row["className"] == class_name and row["methodName"] == method_name
    )


def decode(method: dict) -> tuple[dict, bytes, list]:
    candidate = method["candidates"][0]
    raw = bytes.fromhex(candidate["codeHex"])
    if candidate["codeTruncated"] or candidate["nativeSizeBytes"] != len(raw):
        raise ValueError(f"incomplete body: {method['className']}.{method['methodName']}")
    instructions = list(
        Cs(CS_ARCH_ARM64, CS_MODE_ARM).disasm(raw, int(candidate["moduleOffset"], 16))
    )
    return candidate, raw, instructions


def descriptor(method: dict) -> dict:
    candidate, raw, _ = decode(method)
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
    _, _, instructions = decode(method)
    observed = {(row.address, row.mnemonic, row.op_str) for row in instructions}
    missing = [anchor for anchor in anchors if anchor not in observed]
    if missing:
        raise ValueError(f"native anchors changed for {method['methodName']}: {missing}")


def caller(rows: list[dict], name: str, parameter_types: list[str]) -> dict:
    method = find(rows, "HunterCtrl", name)
    if method["parameterTypes"] != parameter_types:
        raise ValueError(f"signature changed for HunterCtrl.{name}")
    return method


def build(callers_path: Path, actions_path: Path, pass9_targets_path: Path) -> dict:
    callers = load_methods(callers_path)
    actions = load_methods(actions_path)
    pass9_targets = load_methods(pass9_targets_path)

    flame = find(actions, "FlameExplosionCtrl", "Action")
    area = find(actions, "AreaCheckDistanceEffectCtrl", "Action")
    blizzard = find(pass9_targets, "BlizzardCtrl", "Action")
    if flame["parameterTypes"] != [
        "System.String", "System.Int32", "System.String", "System.Int64", "System.Int32", "System.Int32"
    ]:
        raise ValueError("FlameExplosionCtrl.Action signature changed")
    if area["parameterTypes"] != [
        "System.String", "System.Int32", "System.Int64", "System.Int32",
        "System.Int32", "System.String", "System.Int32", "System.Int32",
    ]:
        raise ValueError("AreaCheckDistanceEffectCtrl.Action signature changed")

    pik = caller(callers, "PIKOCNCIHNO", ["EvilCtrl", "System.Single", "System.Single"])
    require(pik, [
        (0x33F4C78, "fmov", "s8, s1"), (0x33F4C7C, "fmov", "s9, s0"),
        (0x33F4E1C, "fmul", "s0, s0, s9"), (0x33F4E2C, "fmul", "s0, s0, s1"),
        (0x33F4E3C, "fcvtzs", "x10, s0"), (0x33F4EE8, "mov", "x4, x22"),
        (0x33F4EEC, "mov", "w6, #2"), (0x33F4EF4, "bl", "#0x2c572b0"),
    ])

    hid = caller(callers, "HIDAPNPHFCA", ["System.Int32", "System.Single", "System.Single", "System.Single"])
    require(hid, [
        (0x3436510, "fmov", "s8, s2"), (0x3436514, "fmov", "s10, s1"),
        (0x343651C, "fmov", "s9, s0"), (0x3436748, "fmul", "s0, s10, s0"),
        (0x3436758, "fmul", "s0, s0, s1"), (0x3436768, "fcvtzs", "x10, s0"),
        (0x3436818, "mov", "x4, x22"), (0x3436820, "bl", "#0x33e9d84"),
    ])

    pmfe = caller(callers, "PMFEHNBKEIL", ["EvilCtrl", "System.Single", "System.Int32"])
    require(pmfe, [
        (0x34236B8, "fmov", "s8, s0"), (0x34239A8, "fmul", "s0, s0, s8"),
        (0x34239AC, "fmul", "s0, s0, s1"), (0x34239B8, "fcvtzs", "x9, s0"),
        (0x3423EBC, "mov", "x3, x21"), (0x3423EDC, "bl", "#0x3024e00"),
    ])

    bojf = caller(callers, "BOJFAPCOBCE", ["EvilCtrl", "System.Single"])
    require(bojf, [
        (0x34444F4, "fmov", "s8, s0"), (0x34447E4, "fmul", "s0, s0, s8"),
        (0x34447F4, "fmul", "s0, s0, s1"), (0x3444804, "fcvtzs", "x10, s0"),
        (0x3444914, "mov", "x3, x22"), (0x3444930, "bl", "#0x3024e00"),
    ])

    chog = caller(callers, "CHOGGFICJPL", ["CodeStage.AntiCheat.ObscuredTypes.ObscuredFloat"])
    require(chog, [
        (0x3420ED4, "mov", "x21, x1"), (0x3421008, "bl", "#0x245668c"),
        (0x3421010, "scvtf", "s2, x22"), (0x3421020, "fmul", "s0, s0, s1"),
        (0x342102C, "fmul", "s0, s0, s2"), (0x3421030, "fcvtzs", "x9, s0"),
        (0x3421130, "mov", "x4, x20"), (0x3421134, "mov", "w6, #1"),
        (0x342113C, "bl", "#0x2c572b0"),
    ])

    omie = caller(callers, "OMIEHJOENAE", ["EvilCtrl", "CodeStage.AntiCheat.ObscuredTypes.ObscuredFloat"])
    require(omie, [
        (0x3422C30, "mov", "x21, x2"), (0x3422F70, "bl", "#0x245668c"),
        (0x3422F78, "scvtf", "s2, x23"), (0x3422F8C, "fmul", "s0, s0, s1"),
        (0x3422F98, "fmul", "s0, s0, s2"), (0x3422F9C, "fcvtzs", "x9, s0"),
        (0x3422FDC, "mov", "x2, x21"), (0x3422FEC, "mov", "w5, #4"),
        (0x3422FF0, "mov", "w6, wzr"), (0x3422FF4, "blr", "x9"),
    ])

    jhaa = caller(callers, "JHAAACFJNPA", ["EvilCtrl", "CodeStage.AntiCheat.ObscuredTypes.ObscuredFloat"])
    require(jhaa, [
        (0x33FA03C, "mov", "x21, x2"), (0x33FA308, "bl", "#0x245668c"),
        (0x33FA32C, "fmul", "s0, s9, s0"), (0x33FA350, "fadd", "s0, s8, s0"),
        (0x33FA360, "fmul", "s0, s0, s1"), (0x33FA368, "fmul", "s0, s0, s1"),
        (0x33FA374, "fcvtzs", "x10, s0"), (0x33FA428, "mov", "x4, x21"),
        (0x33FA42C, "mov", "w6, wzr"), (0x33FA434, "bl", "#0x2c572b0"),
    ])

    mll = caller(callers, "MLLCFGJDLDA", ["EvilCtrl", "CodeStage.AntiCheat.ObscuredTypes.ObscuredFloat"])
    require(mll, [
        (0x3423138, "mov", "x21, x2"), (0x34234E8, "bl", "#0x245668c"),
        (0x34234EC, "fmul", "s0, s9, s0"), (0x3423504, "fadd", "s0, s8, s0"),
        (0x3423508, "fmul", "s0, s0, s1"), (0x3423514, "fmul", "s0, s0, s2"),
        (0x3423518, "fcvtzs", "x9, s0"), (0x3423558, "mov", "x2, x21"),
        (0x3423568, "mov", "w5, #6"), (0x3423570, "blr", "x9"),
    ])

    return {
        "schemaVersion": 1,
        "contractType": "original-native-hunter-skill-coefficients-pass11",
        "runtimeCompatibility": "evidence-only-disconnected",
        "sources": [source(callers_path), source(actions_path), source(pass9_targets_path)],
        "actionTargets": {
            "flameExplosion": descriptor(flame),
            "areaCheckDistance": descriptor(area),
            "blizzard": descriptor(blizzard),
            "evilVirtualDamageBoundary": {
                "dispatch": "target EvilCtrl vtable slot +0x2A8",
                "managedTarget": None,
                "reason": "The native call is BLR through the target object's vtable; matching argument shape alone is not used as identity proof.",
            },
        },
        "families": [
            {
                "id": "plain-single-percent",
                "equation": "trunc_i64(baseDamage * coefficientPercent * 0.01)",
                "members": [
                    {"method": descriptor(pik), "coefficient": "first Single argument", "action": "FlameExplosionCtrl.Action", "damageParameter": 4, "actionSelector": {"parameter6": 2}},
                    {"method": descriptor(hid), "coefficient": "second Single argument", "action": "BlizzardCtrl.Action", "damageParameter": 6},
                    {"method": descriptor(pmfe), "coefficient": "only Single argument", "action": "AreaCheckDistanceEffectCtrl.Action", "damageParameter": 3, "routingInput": "trailing Int32 selects an internal action resource branch"},
                    {"method": descriptor(bojf), "coefficient": "only Single argument", "action": "AreaCheckDistanceEffectCtrl.Action", "damageParameter": 3},
                ],
            },
            {
                "id": "obscured-float-percent",
                "equation": "trunc_i64(baseDamage * decode(coefficientPercent) * 0.01)",
                "members": [
                    {"method": descriptor(chog), "coefficient": "first ObscuredFloat argument", "action": "FlameExplosionCtrl.Action", "damageParameter": 4, "actionSelector": {"parameter6": 1}},
                    {"method": descriptor(omie), "coefficient": "second argument (ObscuredFloat)", "action": "EvilCtrl virtual slot +0x2A8", "damageParameter": 2, "damagedVectorTail": {"parameter5": 4, "parameter6": False}},
                ],
            },
            {
                "id": "affine-obscured-float-percent",
                "equation": "trunc_i64(baseDamage * (basePercent + decode(coefficientPercent) * internalMultiplier) * 0.01)",
                "members": [
                    {"method": descriptor(jhaa), "coefficient": "second argument (ObscuredFloat)", "action": "FlameExplosionCtrl.Action", "damageParameter": 4, "actionSelector": {"parameter6": 0}},
                    {"method": descriptor(mll), "coefficient": "second argument (ObscuredFloat)", "action": "EvilCtrl virtual slot +0x2A8", "damageParameter": 2, "damagedVectorTail": {"parameter5": 6, "parameter6": False}},
                ],
                "semanticBoundary": "basePercent and internalMultiplier sources remain native operands without product-facing names.",
            },
        ],
        "coverage": {
            "exactGetDamageCallerBodies": 49,
            "coefficientMembersResolvedThisPass": 8,
            "coefficientMembersPreviouslyResolved": ["HunterCtrl.GDBMICDJBOK"],
            "remainingCallerBodies": 40,
        },
        "unresolved": [
            "public skill-row names for all eight obfuscated HunterCtrl methods",
            "managed identity of the EvilCtrl vtable +0x2A8 target",
            "product meanings of FlameExplosion and area-action selector integers",
            "coefficient producers for the remaining forty getDamage callers",
            "internal modifier names in the affine coefficient family",
        ],
        "integrationStatus": "disconnected_no_public_skill_mapping_or_live_combat_use",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--callers", type=Path, default=CALLERS)
    parser.add_argument("--actions", type=Path, default=ACTIONS)
    parser.add_argument("--pass9-targets", type=Path, default=PASS9_TARGETS)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    args.output.write_text(
        json.dumps(build(args.callers, args.actions, args.pass9_targets), ensure_ascii=True, indent=2) + "\n"
    )
    print(f"Wrote Hunter skill coefficient evidence to {args.output}")


if __name__ == "__main__":
    main()
