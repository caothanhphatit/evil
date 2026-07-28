#!/usr/bin/env python3
"""Emit deterministic evidence for proven Hunter outgoing-damage boundaries."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs

ROOT = Path(__file__).resolve().parents[1]
COMBAT = ROOT / "reverse-engineering/evidence/original-native-combat-formula-exact-boundaries-api35-v1.json"
TARGETS = ROOT / "reverse-engineering/evidence/original-native-hunter-outgoing-target-resolution-api35-v1.json"
OUTPUT = ROOT / "reverse-engineering/evidence/original-native-hunter-outgoing-damage-pass6.json"


def find(payload, cls, name):
    return next(m for m in payload["methods"] if m["className"] == cls and m["methodName"] == name)


def candidate(method, exact_size=None):
    item = method["candidates"][0]
    raw = bytes.fromhex(item["codeHex"])
    if exact_size is not None:
        raw = raw[:exact_size]
    return item, raw


def instructions(item, raw):
    return list(Cs(CS_ARCH_ARM64, CS_MODE_ARM).disasm(raw, int(item["moduleOffset"], 16)))


def has(ins, mnemonic, operand=None):
    return any(i.mnemonic == mnemonic and (operand is None or i.op_str == operand) for i in ins)


def descriptor(item, raw):
    return {
        "moduleOffset": item["moduleOffset"],
        "nativeSizeBytes": len(raw),
        "sha256": hashlib.sha256(raw).hexdigest(),
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--combat", type=Path, default=COMBAT)
    parser.add_argument("--targets", type=Path, default=TARGETS)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    combat = json.loads(args.combat.read_text())["record"]["payload"]
    targets = json.loads(args.targets.read_text())["record"]["payload"]
    gd_i, gd = candidate(find(combat, "HunterCtrl", "getDamage"))
    cd_i, cd = candidate(find(combat, "HunterCtrl", "getCriticalDamage"))
    ed_i, ed = candidate(find(combat, "EvilCtrl", "Damaged"))
    lc_i, lc = candidate(find(targets, "StatusData", "LCENGICKKGP"), 136)
    gd_ins, cd_ins, lc_ins = instructions(gd_i, gd), instructions(cd_i, cd), instructions(lc_i, lc)
    invariants = {
        "baseHasFloatDivision": has(lc_ins, "fdiv", "s0, s1, s0"),
        "criticalHasBase175": any(
            instruction.mnemonic == "fmov"
            and instruction.op_str.startswith("s9, #1.75")
            for instruction in cd_ins
        ),
        "finalConversionTruncatesTowardZero": has(gd_ins, "fcvtzs", "x9, d0"),
        "getDamageDoesNotDirectlyCallRandDamage": not any(i.mnemonic == "bl" and i.op_str == "#0x2706384" for i in gd_ins),
    }
    if not all(invariants.values()):
        raise SystemExit("native anchor mismatch; evidence not written")
    output = {
        "schemaVersion": 1,
        "contractType": "original-native-hunter-outgoing-damage-pass6",
        "runtimeCompatibility": "evidence-only-disconnected",
        "methods": {
            "StatusData.LCENGICKKGP": descriptor(lc_i, lc),
            "HunterCtrl.getDamage": descriptor(gd_i, gd),
            "HunterCtrl.getCriticalDamage": descriptor(cd_i, cd),
            "EvilCtrl.Damaged": descriptor(ed_i, ed),
        },
        "invariants": invariants,
        "proven": {
            "base": {
                "equation": "ObscuredDouble(float(CalcDamage) / CalcAttackSpeed)",
                "fields": {"CalcDamage": "StatusData+0x28", "CalcAttackSpeed": "StatusData+0x88"},
            },
            "criticalThreshold": {
                "equation": "min(100, CalcCritical + gatedOptionalBonus); Random.Range(0,100) < threshold",
                "fields": {"CalcCritical": "StatusData+0xB0", "gatedOptionalBonus": "HunterCtrl.JOFGKPCLDAI+0x90C", "bonusGate": "HunterCtrl+0x924"},
                "outerGate": "getDamage second Boolean argument bypasses the roll when true",
            },
            "criticalMultiplier": {
                "base": 1.75,
                "namedStatusInputs": [
                    "VillagePetCriDemUp+0x548", "CollectionCriDem+0x558", "RelicCollectionCriDem+0x5AC",
                    "HeroicJobTraitCriDemUp+0x61C", "RidingPetCriDemUp+0x658", "SylphBlessCriDemUp+0x878"
                ],
                "warning": "additional HunterCtrl values and managed-array gates remain obfuscated",
            },
            "rounding": "getDamage performs chained double arithmetic then FCVTZS x9,d0 (toward zero)",
            "variance": "RandDamage is absent from getDamage direct calls and is consumed downstream by EvilCtrl.Damaged",
        },
        "unresolved": [
            "full CalcDamage producer and GetGearDamage caller adjustment",
            "all opaque modifier semantics and target/tag gates",
            "caller-provided skill coefficient boundary",
            "monster armor and minimum-damage consumer after outgoing construction",
            "complete normal/critical/skill/type caller vectors",
        ],
    }
    args.output.write_text(json.dumps(output, indent=2) + "\n")


if __name__ == "__main__":
    main()
