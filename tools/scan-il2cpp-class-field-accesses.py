#!/usr/bin/env python3
"""Scan decrypted ARM64 method bodies for accesses rooted at managed `this`."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs
from capstone.arm64 import ARM64_OP_IMM, ARM64_OP_MEM, ARM64_OP_REG


TARGETS = {
    "HunterCtrl": {
        0x194: "mAttackDelay",
        0x1AC: "AttackAniTime",
        0x3D8: "DANCPPLMKIK",
        0x6AC: "BCEBGLKCDHN",
    },
    "StatusData": {
        0x88: "CalcAttackSpeed",
        0x198: "AttackSpeed",
        0x1AC: "WeaponSpeed",
        0x2F4: "OptionAttackSpeed",
        0x39C: "PersonalAttackSpeed",
        0x724: "RidingPetAttackSpeedUp",
        0x760: "GUP_Property",
        0x7A4: "RankAttackSpeed",
        0x7F8: "FuryValue",
        0x80C: "SpeedPotionValue",
        0x8D0: "Quicken",
    },
}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def register_name(md: Cs, register: int) -> str:
    name = md.reg_name(register)
    return "x" + name[1:] if name.startswith("w") else name


def register_bytes(md: Cs, register: int) -> int:
    name = md.reg_name(register)
    if name.startswith(("q", "v")):
        return 16
    if name.startswith(("x", "d")):
        return 8
    return 4


def scan_method(md: Cs, body: bytes, start: int, targets: dict[int, str]) -> list[dict]:
    instructions = list(md.disasm(body, start))
    stable_roots = set()
    for instruction in instructions[:40]:
        operands = instruction.operands
        if (
            instruction.mnemonic == "mov"
            and len(operands) == 2
            and all(op.type == ARM64_OP_REG for op in operands)
            and register_name(md, operands[1].reg) == "x0"
        ):
            stable_roots.add(register_name(md, operands[0].reg))

    aliases = {"x0": 0, **{register: 0 for register in stable_roots}}
    findings: list[dict] = []

    for instruction in instructions:
        operands = instruction.operands
        mnemonic = instruction.mnemonic

        if mnemonic.startswith(("ldr", "ldur", "str", "stur")) and operands:
            memory = next((operand for operand in operands if operand.type == ARM64_OP_MEM), None)
            if memory is not None:
                base = register_name(md, memory.mem.base)
                if base in aliases:
                    field_offset = aliases[base] + memory.mem.disp
                    if field_offset in targets:
                        findings.append(
                            {
                                "field": targets[field_offset],
                                "fieldOffset": field_offset,
                                "instructionOffset": instruction.address - start,
                                "access": "write" if mnemonic.startswith(("str", "stur")) else "read",
                                "instruction": f"{mnemonic} {instruction.op_str}",
                            }
                        )
        elif mnemonic in {"ldp", "stp"} and len(operands) >= 3 and operands[-1].type == ARM64_OP_MEM:
            memory = operands[-1]
            base = register_name(md, memory.mem.base)
            if base in aliases:
                field_offset = aliases[base] + memory.mem.disp
                step = register_bytes(md, operands[0].reg)
                for index in range(2):
                    offset = field_offset + index * step
                    if offset in targets:
                        findings.append(
                            {
                                "field": targets[offset],
                                "fieldOffset": offset,
                                "instructionOffset": instruction.address - start,
                                "access": "write" if mnemonic == "stp" else "read",
                                "instruction": f"{mnemonic} {instruction.op_str}",
                            }
                        )

        if mnemonic == "bl":
            for index in range(19):
                aliases.pop(f"x{index}", None)
            continue

        if mnemonic == "mov" and len(operands) == 2 and all(op.type == ARM64_OP_REG for op in operands):
            destination = register_name(md, operands[0].reg)
            source = register_name(md, operands[1].reg)
            if source in aliases:
                aliases[destination] = aliases[source]
            else:
                aliases.pop(destination, None)
            continue

        if (
            mnemonic == "add"
            and len(operands) == 3
            and operands[0].type == ARM64_OP_REG
            and operands[1].type == ARM64_OP_REG
            and operands[2].type == ARM64_OP_IMM
        ):
            destination = register_name(md, operands[0].reg)
            source = register_name(md, operands[1].reg)
            if source in aliases:
                aliases[destination] = aliases[source] + operands[2].imm
            else:
                aliases.pop(destination, None)
            continue

        try:
            _, written = instruction.regs_access()
        except Exception:
            written = []
        for register in written:
            name = register_name(md, register)
            if name not in stable_roots:
                aliases.pop(name, None)

    return findings


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--memory-image", type=Path, required=True)
    parser.add_argument("--memory-module-base", required=True)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    inventory = json.loads(args.inventory.read_text())
    module = inventory["record"]["payload"]["module"]
    memory = args.memory_image.read_bytes()
    if len(memory) < module["size"]:
        raise ValueError("memory image is shorter than the captured module")

    md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    md.detail = True
    methods_scanned: dict[str, int] = {}
    findings: list[dict] = []
    for method in inventory["record"]["payload"]["methods"]:
        class_name = method["className"]
        if class_name not in TARGETS:
            continue
        candidate = method["candidates"][0]
        start = int(candidate["moduleOffset"], 16)
        size = candidate.get("nativeSizeBytes")
        if not size:
            continue
        methods_scanned[class_name] = methods_scanned.get(class_name, 0) + 1
        accesses = scan_method(md, memory[start : start + size], start, TARGETS[class_name])
        if accesses:
            findings.append(
                {
                    "className": class_name,
                    "methodName": method["methodName"],
                    "parameterCount": method["parameterCount"],
                    "token": f"0x{method['token']:08X}",
                    "moduleOffset": candidate["moduleOffset"],
                    "nativeSizeBytes": size,
                    "accesses": accesses,
                }
            )

    result = {
        "schemaVersion": 1,
        "contractType": "il2cpp-class-field-access-scan",
        "runtimeCompatibility": "evidence-only",
        "sources": {
            "inventory": {"path": args.inventory.name, "sha256": digest(args.inventory)},
            "memoryImage": {
                "committed": False,
                "size": len(memory),
                "sha256": digest(args.memory_image),
                "moduleBaseAtDump": args.memory_module_base,
                "inventoryModuleBase": module["base"],
            },
        },
        "targets": {
            class_name: [{"name": name, "offset": offset} for offset, name in fields.items()]
            for class_name, fields in TARGETS.items()
        },
        "methodsScanned": methods_scanned,
        "runtimeConstants": [
            {"moduleOffset": f"0x{offset:x}", "float32": struct.unpack_from("<f", memory, offset)[0]}
            for offset in [0xD2AAB8, 0xD2AC8C, 0xD2B834]
        ],
        "findings": findings,
        "limitations": [
            "The scan follows direct ARM64 register aliases rooted at the managed this pointer.",
            "It does not claim indirect native-engine serialization writes or aliases reloaded from arbitrary memory.",
            "The decrypted memory image is intentionally not committed; its size and SHA-256 pin the runtime input.",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote {args.output}")


if __name__ == "__main__":
    main()
