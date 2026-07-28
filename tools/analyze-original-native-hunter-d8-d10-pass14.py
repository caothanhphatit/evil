#!/usr/bin/env python3
"""Normalize the proven early D8/D10 tree in HunterCtrl.getDamage."""
from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "reverse-engineering/evidence/original-native-combat-formula-exact-boundaries-api35-v1.json"
IMAGE = Path("/tmp/evil-libil2cpp-memory.bin")
OUTPUT = ROOT / "reverse-engineering/evidence/original-native-hunter-d8-d10-pass14.json"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, default=SOURCE)
    parser.add_argument("--memory-image", type=Path, default=IMAGE)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    payload = json.loads(args.source.read_text())["record"]["payload"]
    method = next(x for x in payload["methods"] if x["className"] == "HunterCtrl" and x["methodName"] == "getDamage")
    candidate = method["candidates"][0]
    body = bytes.fromhex(candidate["codeHex"])
    base = int(candidate["moduleOffset"], 16)
    ins = {x.address - base: (x.mnemonic, x.op_str) for x in Cs(CS_ARCH_ARM64, CS_MODE_ARM).disasm(body, base)}
    checks = {
        0x10C:("tbz","w23, #0, #0x33f5454"), 0x140:("fmov","d8, d0"),
        0x164:("fcmp","s0, s1"), 0x18C:("bl","#0x33fa49c"),
        0x26C:("adrp","x8, #0xd28000"), 0x274:("fmul","s0, s10, s0"),
        0x278:("fadd","s0, s9, s0"), 0x27C:("fcvt","d0, s0"),
        0x280:("fmul","d0, d0, d1"), 0x284:("fmul","d0, d8, d0"),
        0x288:("fadd","d8, d8, d0"), 0x2B0:("scvtf","d8, x0"),
        0x2FC:("fsub","s0, s1, s0"), 0x304:("fmul","d8, d8, d0"),
        0x39C:("ldr","q0, [x0, #0x930]"), 0x500:("fmul","s0, s0, s1"),
        0x504:("fadd","s9, s9, s0"), 0x524:("fmul","d0, d8, d0"),
        0x528:("fadd","d10, d8, d0"), 0xF40:("fmul","d10, d10, d0"),
    }
    for offset, expected in checks.items():
        if ins.get(offset) != expected:
            raise ValueError(f"0x{offset:x}: {ins.get(offset)} != {expected}")
    with args.memory_image.open("rb") as stream:
        stream.seek(0xD282B0)
        percent = stream.read(8)
    if percent.hex() != "7b14ae47e17a843f":
        raise ValueError("double percent literal changed")
    result = {
        "schemaVersion": 2,
        "contractType": "original-native-hunter-d8-d10-pass14",
        "runtimeCompatibility": "evidence-only-disconnected",
        "method": {"offset": candidate["moduleOffset"], "size": len(body), "sha256": hashlib.sha256(body).hexdigest()},
        "staticDouble": {"offset": "0xD282B0", "raw": percent.hex(), "float64": struct.unpack("<d", percent)[0]},
        "arg1Tree": {
            "arg1False": "D8 = decoded StatusData.CalcDamage@0x28 converted Int64->Float64",
            "arg1True": "D8 = StatusData.LCENGICKKGP = CalcDamage / CalcAttackSpeed",
            "commonPost": "if HunterCtrl.DJDEHDEKGIO@0x7A0 != 0, D8 *= (1 - DJDEHDEKGIO)",
        },
        "jobTrait5Augmentation": {
            "conditions": ["arg1 is true", "decoded HunterCtrl.HMKFKBCNPDH@0x6C0 > 1.0", "CheckJobTrait(5) decodes true"],
            "operands": ["DataManager+0x1B8 array element 5, opaque ObscuredFloat at +0xD0", "same object opaque ObscuredFloat at +0xE4", "decoded integer returned by HunterData.skill@0x5B8 lookup with unresolved static key"],
            "float32Stage": "P32 = opaqueFloat0 + opaqueFloat1 * float32(decodedInteger)",
            "float64Stage": "P64 = float64(P32) * 0.01; D8 = D8 + D8 * P64",
            "castsRounding": "SCVTF Int32->Float32, FMUL/FADD Float32, FCVT Float32->Float64; no integer rounding instruction in this branch",
        },
        "earlyPercent": {
            "initial": "S9 = StatusData.gearPropertyNeedMoveSpeed@0x930",
            "adds": ["gated StatusData.DragonProtectionFairyAtkValue@0x518", "positive RidingPetGearProperty[11] * 0.01 while not in meze state"],
            "construction": "D10 = D8 * (1 + S9)",
        },
        "jobSubJobSelector": {"decodedFields":["HunterData.job@0x20","HunterData.subJob@0x30"], "observedSpecialPairs":["job=1, subJob=2","job=1, subJob=3","job=4, subJob=1","job=4, subJob=3"]},
        "arrays": {"GearProperty":"StatusData+0x210; fixed observed gates 79 and 99 plus dynamic indices", "RunesProperty":"StatusData+0x230; dynamically selected row/elements", "RidingPetGearProperty":"StatusData+0x238; element 11", "AdminRows":"DataManager+0x128/+0x1B8 and GameManager static pairs; class labels unresolved"},
        "commonJobMultiplier": "selected opaque0 + opaque1*decodedInteger; Float32 arithmetic, *0.01, +1, FCVT to Float64, then D10 *= factor",
        "unresolved": ["AdminData class/field names", "semantic labels for GearProperty 79/99 and dynamic indices", "caller-facing meaning of arg1", "static dictionary key used by JobTrait(5)", "full dynamic GameManager index values"],
    }
    args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
