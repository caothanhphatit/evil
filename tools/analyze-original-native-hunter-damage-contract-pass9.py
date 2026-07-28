#!/usr/bin/env python3
"""Normalize proven getDamage callers and EvilCtrl.Damaged parameter roles."""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs


ROOT = Path(__file__).resolve().parents[1]
CALLERS = ROOT / "reverse-engineering/evidence/original-native-hunter-getdamage-callers-api35-v1.json"
TARGETS = ROOT / "reverse-engineering/evidence/original-native-hunter-damage-contract-targets-api35-v1.json"
OUTPUT = ROOT / "reverse-engineering/evidence/original-native-hunter-damage-contract-pass9.json"
GET_DAMAGE_OFFSET = 0x33F51C4


def source(path: Path) -> dict:
    body = path.read_bytes()
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "bytes": len(body),
        "sha256": hashlib.sha256(body).hexdigest(),
    }


def methods(path: Path) -> list[dict]:
    return json.loads(path.read_text())["record"]["payload"]["methods"]


def body(method: dict) -> tuple[dict, bytes, list]:
    candidate = method["candidates"][0]
    raw = bytes.fromhex(candidate["codeHex"])
    if candidate["codeTruncated"] or len(raw) != candidate["nativeSizeBytes"]:
        raise ValueError(f"incomplete exact body: {method['className']}.{method['methodName']}")
    instructions = list(
        Cs(CS_ARCH_ARM64, CS_MODE_ARM).disasm(raw, int(candidate["moduleOffset"], 16))
    )
    return candidate, raw, instructions


def immediate_before(instructions: list, call_index: int, register: str) -> int:
    for instruction in reversed(instructions[max(0, call_index - 16) : call_index]):
        if not instruction.op_str.startswith(f"{register},"):
            continue
        if instruction.mnemonic != "mov":
            break
        value = instruction.op_str.split(",", 1)[1].strip()
        if value in {"wzr", "xzr"}:
            return 0
        if value.startswith("#"):
            return int(value[1:], 0)
        break
    raise ValueError(f"no immediate writer for {register} before getDamage")


def contains(instructions: list, address: int, mnemonic: str, operands: str) -> bool:
    return any(
        instruction.address == address
        and instruction.mnemonic == mnemonic
        and instruction.op_str == operands
        for instruction in instructions
    )


def find(targets: list[dict], class_name: str, method_name: str) -> dict:
    return next(
        row
        for row in targets
        if row["className"] == class_name and row["methodName"] == method_name
    )


def descriptor(method: dict) -> dict:
    candidate, raw, _ = body(method)
    return {
        "className": method["className"],
        "methodName": method["methodName"],
        "parameterTypes": method["parameterTypes"],
        "returnType": method["returnType"],
        "token": method["token"],
        "moduleOffset": candidate["moduleOffset"],
        "nativeSizeBytes": len(raw),
        "bodySha256": hashlib.sha256(raw).hexdigest(),
    }


def build(callers_path: Path, targets_path: Path) -> dict:
    caller_rows = methods(callers_path)
    targets = methods(targets_path)
    catalog = []
    for method in caller_rows:
        candidate, _, instructions = body(method)
        calls = [
            (index, instruction)
            for index, instruction in enumerate(instructions)
            if instruction.mnemonic == "bl"
            and instruction.op_str == f"#0x{GET_DAMAGE_OFFSET:x}"
        ]
        if len(calls) != 1:
            raise ValueError(f"expected one getDamage call in {method['methodName']}")
        call_index, call = calls[0]
        vector = [
            immediate_before(instructions, call_index, register)
            for register in ("w1", "w2", "w3")
        ]
        if any(value not in (0, 1) for value in vector):
            raise ValueError(f"non-Boolean getDamage vector in {method['methodName']}")
        catalog.append(
            {
                "method": method["methodName"],
                "parameterTypes": method["parameterTypes"],
                "token": method["token"],
                "methodOffset": candidate["moduleOffset"],
                "callSiteOffset": f"0x{call.address:x}",
                "arguments": vector,
            }
        )

    vectors = Counter(tuple(row["arguments"]) for row in catalog)
    expected_vectors = {(0, 0, 0): 2, (1, 0, 0): 10, (1, 0, 1): 36, (1, 1, 0): 1}
    if len(catalog) != 49 or vectors != expected_vectors:
        raise ValueError(f"getDamage caller boundary changed: {len(catalog)=}, {vectors=}")
    basic = next(row for row in catalog if row["method"] == "HuntingAttackAction")
    if basic["arguments"] != [0, 0, 0] or basic["callSiteOffset"] != "0x34173d4":
        raise ValueError("basic attack getDamage vector changed")

    gdb = find(targets, "HunterCtrl", "GDBMICDJBOK")
    _, _, gdb_instructions = body(gdb)
    gdb_anchors = [
        (0x34337F0, "fmov", "s8, s2"),
        (0x34337F4, "fmov", "s10, s1"),
        (0x34337FC, "fmov", "s9, s0"),
        (0x343380C, "mov", "w20, w1"),
        (0x34338A0, "mov", "w1, #1"),
        (0x34338A4, "mov", "w2, wzr"),
        (0x34338A8, "mov", "w3, #1"),
        (0x3433D6C, "fmul", "s0, s14, s10"),
        (0x3433D8C, "fadd", "s0, s0, s10"),
        (0x3433D98, "fmul", "s0, s0, s1"),
        (0x3433DA4, "fmul", "s0, s0, s13"),
        (0x3433DA8, "fcvtzs", "x10, s0"),
        (0x3433E58, "mov", "x4, x22"),
        (0x3433E60, "bl", "#0x33e9d84"),
    ]
    if not all(contains(gdb_instructions, *anchor) for anchor in gdb_anchors):
        raise ValueError("GDBMICDJBOK coefficient or forwarding chain changed")

    blizzard = find(targets, "BlizzardCtrl", "Action")
    damage_show = find(targets, "DamageManager", "Show")
    evil = find(targets, "EvilCtrl", "Damaged")
    if blizzard["parameterTypes"] != [
        "System.Int32",
        "System.String",
        "System.Int32",
        "System.Single",
        "System.Single",
        "System.Int64",
        "System.Int32",
    ]:
        raise ValueError("BlizzardCtrl.Action signature changed")
    if damage_show["parameterTypes"] != [
        "System.Int32",
        "System.Int64",
        "UnityEngine.Vector3",
        "System.Int32",
        "System.Boolean",
    ]:
        raise ValueError("DamageManager.Show signature changed")

    _, _, evil_instructions = body(evil)
    evil_anchors = [
        (0x2F2BE44, "mov", "w26, w6"),
        (0x2F2BE48, "mov", "w22, w5"),
        (0x2F2BE50, "mov", "w19, w4"),
        (0x2F2BE54, "mov", "w23, w3"),
        (0x2F2C600, "tbz", "w26, #0, #0x2f2c644"),
        (0x2F2C644, "cmp", "w23, #2"),
        (0x2F2C698, "tbz", "w26, #0, #0x2f2c6b0"),
        (0x2F2CB38, "mov", "w1, w23"),
        (0x2F2CB40, "mov", "w3, w22"),
        (0x2F2CB4C, "bl", "#0x2fcba7c"),
        (0x2F2CEBC, "and", "w2, w19, #1"),
    ]
    if not all(contains(evil_instructions, *anchor) for anchor in evil_anchors):
        raise ValueError("EvilCtrl.Damaged parameter-use chain changed")

    return {
        "schemaVersion": 1,
        "contractType": "original-native-hunter-damage-contract-pass9",
        "runtimeCompatibility": "evidence-only-disconnected",
        "sources": [source(callers_path), source(targets_path)],
        "capturedMethods": {
            "skillBuilder": descriptor(gdb),
            "skillAction": descriptor(blizzard),
            "damageConsumer": descriptor(evil),
            "damagePresentation": descriptor(damage_show),
        },
        "getDamageCallerBoundary": {
            "directCallerCount": len(catalog),
            "argumentVectorCounts": {
                "false,false,false": vectors[(0, 0, 0)],
                "true,false,false": vectors[(1, 0, 0)],
                "true,false,true": vectors[(1, 0, 1)],
                "true,true,false": vectors[(1, 1, 0)],
            },
            "basicAttack": basic,
            "callers": sorted(catalog, key=lambda row: int(row["callSiteOffset"], 16)),
            "semanticBoundary": "The three Boolean meanings are not assigned from vector frequency or obfuscated caller names.",
        },
        "provenSkillCoefficientSegment": {
            "method": "HunterCtrl.GDBMICDJBOK(Int32, Single, Single, Single)",
            "getDamageArguments": [True, False, True],
            "coefficientInput": "second Single argument (ARM64 s1, preserved in s10)",
            "equation": "trunc_i64(baseDamage * coefficientPercent * (1 + modifierAggregate) * 0.01)",
            "forwarding": "The resulting Int64 is passed as BlizzardCtrl.Action parameter 6.",
            "limits": "modifierAggregate contributors are structurally captured but not all semantically named; this segment is not generalized to other skills.",
        },
        "evilDamagedParameterRoles": {
            "parameter3": {
                "type": "Int32",
                "provenUses": [
                    "value 2 gates addition of EvilCtrl field 0x1F8 to the pre-armor bonus accumulator",
                    "forwarded as DamageManager.Show parameter 1",
                ],
                "semanticName": None,
            },
            "parameter4": {
                "type": "Boolean",
                "provenUses": ["forwarded as a Boolean to a post-damage virtual callback"],
                "semanticName": None,
            },
            "parameter5": {
                "type": "Int32",
                "provenUses": ["forwarded as DamageManager.Show parameter 4"],
                "semanticName": None,
            },
            "parameter6": {
                "type": "Boolean",
                "provenUses": [
                    "enables the captured RidingPetGearProperty[16] pre-armor bonus branch",
                    "when true bypasses the normal effective-armor calculation path",
                ],
                "semanticName": "armorBypassGate (mechanical role only)",
            },
        },
        "unresolved": [
            "product-facing meanings of getDamage Boolean parameters 1 through 3",
            "product-facing names of EvilCtrl.Damaged parameters 3 through 5",
            "virtual projectile/effect call paths that invoke EvilCtrl.Damaged",
            "all skill coefficient contracts other than the proven GDBMICDJBOK to BlizzardCtrl.Action segment",
            "target/tag branch semantics inside getDamage and the content rows selecting each obfuscated caller",
        ],
        "integrationStatus": "disconnected_until_skill_rows_target_rules_and_complete_caller_vectors_are_semantically_bound",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--callers", type=Path, default=CALLERS)
    parser.add_argument("--targets", type=Path, default=TARGETS)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    args.output.write_text(json.dumps(build(args.callers, args.targets), ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote Hunter damage contract evidence to {args.output}")


if __name__ == "__main__":
    main()
