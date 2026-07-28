#!/usr/bin/env python3
"""Verify the native CalcLevel/CalcRevive producer arithmetic."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs


METHODS = {
    ".ctor": (100670729, "0x2d6a404"),
    "OLHJKKDDMHM": (100670733, "0x2d5f990"),
    "MDCNDJHNOAE": (100670744, "0x2d6b0b4"),
}


def method_map(payload: dict) -> dict[str, dict]:
    return {
        method["methodName"]: method
        for method in payload["record"]["payload"]["methods"]
        if method.get("className") == "StatusData" and method.get("methodName") in METHODS
    }


def find_class(payload: dict, class_name: str) -> dict:
    pending = [payload]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            if value.get("name") == class_name and isinstance(value.get("fields"), list):
                return value
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)
    raise ValueError(f"class schema not found: {class_name}")


def disassemble(code_hex: str, address: int) -> list[str]:
    decoder = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    return [
        f"0x{instruction.address:x}: {instruction.mnemonic} {instruction.op_str}".rstrip()
        for instruction in decoder.disasm(bytes.fromhex(code_hex), address)
    ]


def require_lines(lines: list[str], fragments: list[str], method_name: str) -> None:
    text = "\n".join(lines)
    missing = [fragment for fragment in fragments if fragment not in text]
    if missing:
        raise ValueError(f"{method_name} missing native patterns: {missing}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--methods", type=Path, required=True)
    parser.add_argument("--static-factors", type=Path, required=True)
    parser.add_argument("--calc-damage-producer", type=Path, required=True)
    parser.add_argument("--guild-owner-method", type=Path, required=True)
    parser.add_argument("--guild-schema", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    methods_payload = json.loads(args.methods.read_text())
    static_payload = json.loads(args.static_factors.read_text())
    calc_damage_payload = json.loads(args.calc_damage_producer.read_text())
    guild_owner_payload = json.loads(args.guild_owner_method.read_text())
    guild_schema_payload = json.loads(args.guild_schema.read_text())
    selected = method_map(methods_payload)
    if set(selected) != set(METHODS):
        raise ValueError(f"missing StatusData methods: {sorted(set(METHODS) - set(selected))}")

    records = {}
    for name, (token, expected_offset) in METHODS.items():
        method = selected[name]
        candidate = method["candidates"][0]
        if method["token"] != token or candidate["moduleOffset"] != expected_offset:
            raise ValueError(f"unexpected identity for StatusData.{name}")
        code_hex = candidate["codeHex"]
        records[name] = {
            "token": token,
            "moduleOffset": expected_offset,
            "nativeSizeBytes": candidate["nativeSizeBytes"],
            "sha256": hashlib.sha256(bytes.fromhex(code_hex)).hexdigest(),
            "disassembly": disassemble(code_hex, int(expected_offset, 16)),
        }

    require_lines(
        records[".ctor"]["disassembly"],
        ["mov w8, #1", "str w8, [x19, #0x7e0]", "add x8, x19, #0x7e4"],
        ".ctor",
    )
    require_lines(
        records["OLHJKKDDMHM"]["disassembly"],
        [
            "bl #0x2d5bed0",
            "ldp x8, x1, [x0, #0x88]",
            "scvtf s0, w0",
            "ldr s1, [x8, #0x96c]",
            "fmul s0, s0, s1",
            "fmov s1, #1.00000000",
            "fadd s0, s0, s1",
            "add x8, x19, #0x7e4",
        ],
        "OLHJKKDDMHM",
    )
    require_lines(
        records["MDCNDJHNOAE"]["disassembly"],
        [
            "bl #0x2d5bed0",
            "ldur x8, [x0, #0xc4]",
            "ldur x1, [x0, #0xcc]",
            "cmp w0, #1",
            "b.lt #0x2d6b108",
            "add w8, w0, w0, lsl #1",
            "str w8, [x19, #0x7e0]",
        ],
        "MDCNDJHNOAE",
    )

    factor = static_payload["calcLevelFactor"]
    raw = bytes.fromhex(factor["rawHex"])
    decoded = struct.unpack("<f", raw)[0]
    if factor["moduleOffset"].lower() != "0xd2a96c" or raw.hex() != "a69b443b":
        raise ValueError("unexpected CalcLevel package literal")

    calc_damage_method = next(
        method
        for method in calc_damage_payload["record"]["payload"]["methods"]
        if method["className"] == "StatusData" and method["methodName"] == "EBNGMMPBEDA"
    )
    calc_damage_candidate = calc_damage_method["candidates"][0]
    calc_damage_disassembly = disassemble(
        calc_damage_candidate["codeHex"], int(calc_damage_candidate["moduleOffset"], 16)
    )
    require_lines(
        calc_damage_disassembly,
        [
            "cmp w0, #0x4e",
            "cmp w0, #0x1a2",
            "cmp w0, #0x257",
            "cmp w0, #0x258",
            "cmp w0, #0x168",
            "cmp w0, #0x2ec",
            "cmp w0, #0x305",
            "cmp w0, #0x31",
            "bl #0x2fa2d94",
            "ldur q0, [x0, #0x98]",
        ],
        "EBNGMMPBEDA",
    )

    fairy_factors = static_payload["fairyAttackFactors"]
    expected_fairy = {
        "fairy_index_78_418_599_600": ("0xd2b4f8", "0ad7a33c"),
        "fairy_index_360": ("0xd2baec", "0ad7233d"),
        "fairy_index_748_773": ("0xd2a6a0", "8fc2753d"),
    }
    for name, (expected_offset, expected_raw) in expected_fairy.items():
        observed = fairy_factors[name]
        if observed["moduleOffset"].lower() != expected_offset or observed["rawHex"] != expected_raw:
            raise ValueError(f"unexpected fairy attack factor: {name}")
    poly_factor = static_payload["polyIndex49Multiplier"]
    if (
        poly_factor["moduleOffset"].lower() != "0xd282f8"
        or poly_factor["rawHex"] != "000000c0ccccf43f"
    ):
        raise ValueError("unexpected PolyIndex 49 multiplier")

    guild_method = guild_owner_payload["record"]["payload"]["methods"][0]
    if (
        guild_method["className"] != "GuildManager"
        or guild_method["methodName"] != "getInstance"
        or guild_method["candidates"][0]["moduleOffset"] != "0x2fa2d94"
    ):
        raise ValueError("CalcDamage singleton target is not GuildManager.getInstance")
    guild_class = find_class(guild_schema_payload, "GuildManager")
    guild_field = next(field for field in guild_class["fields"] if field["offset"] == 152)
    if guild_field["name"] != "mRankBuffAttack" or guild_field["type"] != "CodeStage.AntiCheat.ObscuredTypes.ObscuredFloat":
        raise ValueError("GuildManager+0x98 is not mRankBuffAttack")

    output = {
        "schemaVersion": 1,
        "contractType": "original-native-status-data-level-revive-analysis",
        "runtimeCompatibility": "evidence-only",
        "source": {
            "methodInventory": str(args.methods),
            "methodInventorySha256": hashlib.sha256(args.methods.read_bytes()).hexdigest(),
            "staticFactorCapture": str(args.static_factors),
            "staticFactorCaptureSha256": hashlib.sha256(args.static_factors.read_bytes()).hexdigest(),
            "calcDamageProducer": str(args.calc_damage_producer),
            "calcDamageProducerSha256": hashlib.sha256(args.calc_damage_producer.read_bytes()).hexdigest(),
            "guildOwnerMethod": str(args.guild_owner_method),
            "guildOwnerMethodSha256": hashlib.sha256(args.guild_owner_method.read_bytes()).hexdigest(),
            "guildSchema": str(args.guild_schema),
            "guildSchemaSha256": hashlib.sha256(args.guild_schema.read_bytes()).hexdigest(),
        },
        "methods": records,
        "resolved": {
            "hunterDataFields": {
                "level": {"offset": 136, "type": "ObscuredInt"},
                "revive": {"offset": 196, "type": "ObscuredInt"},
            },
            "statusDataFields": {
                "CalcRevive": {"offset": 2016, "type": "Int32"},
                "CalcLevel": {"offset": 2020, "type": "ObscuredFloat"},
            },
            "constructorDefaults": {"CalcRevive": 1.0, "CalcLevel": 0.0},
            "calcLevelFactor": {"rawHex": raw.hex(), "float32": decoded},
            "formulas": {
                "CalcLevel": "float32(1.0 + float32(HunterData.level) * float32(0.003))",
                "CalcRevive": "HunterData.revive < 1 ? 1 : wrapping_i32(HunterData.revive * 3)",
            },
            "fairyAttackUp": {
                "sourceField": "HunterData.fairyIndex",
                "sourceOffset": 1244,
                "statusField": "StatusData.FairyAttackUp",
                "statusOffset": 1224,
                "indexGroups": {
                    "78,418,599,600": fairy_factors["fairy_index_78_418_599_600"]["float32"],
                    "360": fairy_factors["fairy_index_360"]["float32"],
                    "748,773": fairy_factors["fairy_index_748_773"]["float32"],
                    "otherwise": 0.0,
                },
                "application": "damage *= 1.0 + FairyAttackUp when a listed fairyIndex branch matches",
            },
            "polyIndex49": {
                "sourceField": "StatusData.PolyIndex",
                "sourceOffset": 2120,
                "multiplier": poly_factor["float64"],
                "application": "if PolyIndex == 49: damage *= float64(1.2999999523162842)",
            },
            "tormentGuildLayer": {
                "formula": "damage *= 1.0 + UserData.mTormentAttackUp + GuildManager.mRankBuffAttack",
                "guildGetter": "GuildManager.getInstance",
                "guildGetterModuleOffset": "0x2fa2d94",
                "guildFieldOffset": 152,
            },
        },
        "integrationStatus": "disconnected_until_complete_CalcDamage_and_target_damage_consumers_are_resolved",
        "limitations": [
            "This analysis resolves only the CalcLevel and CalcRevive producer operands.",
            "It does not authorize replacing fixture live combat with the original formula.",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote StatusData level/revive analysis to {args.output}")


if __name__ == "__main__":
    main()
