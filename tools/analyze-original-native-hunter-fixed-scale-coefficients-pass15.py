#!/usr/bin/env python3
"""Recover exact fixed-scale ConstantData Hunter damage caller arithmetic."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import struct
import zipfile
from pathlib import Path

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs


ROOT = Path(__file__).resolve().parents[1]
CALLERS = ROOT / "reverse-engineering/evidence/original-native-hunter-getdamage-callers-api35-v1.json"
ACTIONS = ROOT / "reverse-engineering/evidence/original-native-hunter-skill-coefficient-action-targets-api35-v1.json"
CONSTANT_SCHEMA = ROOT / "reverse-engineering/evidence/constant-data-runtime-schema-api35-v1.json"
XAPK = ROOT / "game-assets/source/Evil+Hunter+Tycoon_1.411_APKPure.xapk"
OUTPUT = ROOT / "reverse-engineering/evidence/original-native-hunter-fixed-scale-coefficients-pass15.json"


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
    return next(
        row for row in rows
        if row["className"] == class_name and row["methodName"] == method_name
    )


def decode(method: dict) -> tuple[bytes, list]:
    candidate = method["candidates"][0]
    raw = bytes.fromhex(candidate["codeHex"])
    if candidate["codeTruncated"] or candidate["nativeSizeBytes"] != len(raw):
        raise ValueError(f"incomplete exact body: {method['methodName']}")
    instructions = list(
        Cs(CS_ARCH_ARM64, CS_MODE_ARM).disasm(raw, int(candidate["moduleOffset"], 16))
    )
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


def package_literals(xapk_path: Path) -> tuple[dict, dict[str, dict]]:
    with zipfile.ZipFile(xapk_path) as outer:
        apk_bytes = outer.read("config.arm64_v8a.apk")
    with zipfile.ZipFile(io.BytesIO(apk_bytes)) as apk:
        lib = apk.read("lib/arm64-v8a/libil2cpp.so")

    specs = {
        "integerPercentScale": 0xD2AC8C,
        "featherShotFixedScale": 0xD2A064,
        "darkRiftFixedScale": 0xD29EB8,
    }
    literals = {}
    for name, offset in specs.items():
        raw = lib[offset : offset + 4]
        literals[name] = {
            "moduleOffset": f"0x{offset:x}",
            "rawHex": raw.hex(),
            "type": "float32",
            "value": struct.unpack("<f", raw)[0],
        }
    return {
        "containerEntry": "config.arm64_v8a.apk",
        "libraryEntry": "lib/arm64-v8a/libil2cpp.so",
        "libraryBytes": len(lib),
        "librarySha256": hashlib.sha256(lib).hexdigest(),
    }, literals


def build(callers_path: Path, actions_path: Path, constant_schema_path: Path, xapk_path: Path) -> dict:
    caller_rows = methods(callers_path)
    action_rows = methods(actions_path)
    constants = constant_fields(json.loads(constant_schema_path.read_text()))
    package_library, literals = package_literals(xapk_path)

    expected_literals = {
        "integerPercentScale": ("0xd2ac8c", "0ad7233c", 0.009999999776482582),
        "featherShotFixedScale": ("0xd2a064", "00209544", 1193.0),
        "darkRiftFixedScale": ("0xd29eb8", "00a0c744", 1597.0),
    }
    for name, (offset, raw_hex, value) in expected_literals.items():
        actual = literals[name]
        if (actual["moduleOffset"], actual["rawHex"], actual["value"]) != (offset, raw_hex, value):
            raise ValueError(f"package literal changed: {name}")

    expected_fields = {
        0x2870: "FEATHER_SHOT_POWER_VALUE",
        0x2850: "DARK_RIFT_POWER_VALUE",
    }
    for offset, name in expected_fields.items():
        field = constants[offset]
        if field["name"] != name or field["type"] != "CodeStage.AntiCheat.ObscuredTypes.ObscuredInt":
            raise ValueError(f"ConstantData field changed at 0x{offset:X}")

    feather_percent = find(caller_rows, "HunterCtrl", "JDONOEEBDCD")
    require(feather_percent, [
        (0x3421B78, "mov", "w1, #1"), (0x3421B7C, "mov", "w2, wzr"),
        (0x3421B80, "mov", "w3, wzr"), (0x3421B84, "bl", "#0x33f51c4"),
        (0x3421BD0, "ldr", "x0, [x8, #0x2870]"), (0x3421BD8, "bl", "#0x245672c"),
        (0x3421BE8, "ldr", "s1, [x8, #0xc8c]"), (0x3421BF8, "fmul", "s0, s0, s1"),
        (0x3421C04, "fmul", "s0, s0, s2"), (0x3421C08, "fcvtzs", "x9, s0"),
        (0x3421CB4, "bl", "#0x27e8964"), (0x3421DDC, "mov", "x4, x22"),
        (0x3421DEC, "bl", "#0x32ff6cc"),
    ])

    feather_fixed = find(caller_rows, "HunterCtrl", "BGIJEDLALGE")
    require(feather_fixed, [
        (0x343325C, "mov", "w1, #1"), (0x3433260, "mov", "w2, wzr"),
        (0x3433264, "mov", "w3, wzr"), (0x3433268, "bl", "#0x33f51c4"),
        (0x34332B4, "ldr", "x0, [x8, #0x2870]"), (0x34332BC, "bl", "#0x245672c"),
        (0x34332CC, "ldr", "s1, [x8, #0x64]"), (0x34332DC, "fmul", "s0, s0, s1"),
        (0x34332E8, "fmul", "s0, s0, s2"), (0x34332EC, "fcvtzs", "x9, s0"),
        (0x343339C, "bl", "#0x27e8964"), (0x34334D4, "mov", "x4, x22"),
        (0x34334E4, "bl", "#0x32ff6cc"),
    ])

    dark_rift_fixed = find(caller_rows, "HunterCtrl", "BGHEAJHAICN")
    require(dark_rift_fixed, [
        (0x3464B5C, "mov", "w1, #1"), (0x3464B60, "mov", "w2, #1"),
        (0x3464B64, "mov", "w3, wzr"), (0x3464B68, "bl", "#0x33f51c4"),
        (0x3464B98, "ldr", "x0, [x8, #0x2850]"), (0x3464BA0, "bl", "#0x245672c"),
        (0x3464BB0, "ldr", "s1, [x8, #0xeb8]"), (0x3464BB8, "fmul", "s0, s0, s1"),
        (0x3464BBC, "bl", "#0x245787c"), (0x3464BE4, "bl", "#0x24567e8"),
        (0x3464C04, "bl", "#0x245668c"), (0x3464C28, "fmul", "s0, s0, s1"),
        (0x3464C4C, "fcvtzs", "x12, s0"), (0x3464CDC, "mov", "x4, x22"),
        (0x3464CE0, "mov", "w6, #1"), (0x3464CE8, "bl", "#0x2c572b0"),
    ])

    flame = find(action_rows, "FlameExplosionCtrl", "Action")
    feather_field = {**constants[0x2870], "runtimeDataOffset": "0x2870"}
    dark_rift_field = {**constants[0x2850], "runtimeDataOffset": "0x2850"}

    return {
        "schemaVersion": 1,
        "contractType": "original-native-hunter-fixed-scale-coefficients-pass15",
        "runtimeCompatibility": "evidence-only-disconnected",
        "sources": [
            source(callers_path), source(actions_path), source(constant_schema_path), source(xapk_path),
        ],
        "packageLibrary": package_library,
        "packageLiterals": literals,
        "families": [
            {
                "id": "direct-obscured-int-package-scale",
                "pipeline": "decode ConstantData ObscuredInt; convert to float32; multiply by exact package float32 scale; multiply by float32(baseDamage); FCVTZS",
                "rounding": "all multiplication is float32; final FCVTZS truncates toward zero",
                "members": [
                    {
                        "method": descriptor(feather_percent),
                        "getDamageVector": [True, False, False],
                        "coefficientSource": feather_field,
                        "packageScale": literals["integerPercentScale"],
                        "equation": "damage = trunc_i64(float32(baseDamage) * float32(decode(FEATHER_SHOT_POWER_VALUE)) * 0.01f)",
                        "presentationCall": {"targetModuleOffset": "0x27e8964", "damageForwarded": False, "selectorRegister": "w3=0"},
                        "damageRoute": {"targetModuleOffset": "0x32ff6cc", "managedIdentity": None, "damageArgumentRegister": "x4"},
                    },
                    {
                        "method": descriptor(feather_fixed),
                        "getDamageVector": [True, False, False],
                        "coefficientSource": feather_field,
                        "packageScale": literals["featherShotFixedScale"],
                        "equation": "damage = trunc_i64(float32(baseDamage) * float32(decode(FEATHER_SHOT_POWER_VALUE)) * 1193.0f)",
                        "presentationCall": {"targetModuleOffset": "0x27e8964", "damageForwarded": False, "selectorRegister": "w3=1"},
                        "damageRoute": {"targetModuleOffset": "0x32ff6cc", "managedIdentity": None, "damageArgumentRegister": "x4"},
                    },
                ],
            },
            {
                "id": "obscured-int-fixed-scale-obscured-float-roundtrip",
                "pipeline": "decode ConstantData ObscuredInt; convert to float32; multiply by exact package float32 scale; initialize/decode ObscuredFloat; multiply by float32(baseDamage); FCVTZS",
                "rounding": "all multiplication is float32; final FCVTZS truncates toward zero",
                "members": [
                    {
                        "method": descriptor(dark_rift_fixed),
                        "getDamageVector": [True, True, False],
                        "coefficientSource": dark_rift_field,
                        "packageScale": literals["darkRiftFixedScale"],
                        "equation": "wrappedCoefficient = ObscuredFloat(float32(decode(DARK_RIFT_POWER_VALUE)) * 1597.0f); damage = trunc_i64(float32(baseDamage) * decode(wrappedCoefficient))",
                        "damageRoute": {"target": "FlameExplosionCtrl.Action", "damageParameter": 4, "selectorParameter6": 1},
                    }
                ],
            },
        ],
        "resolvedActionTargets": {"flameExplosion": descriptor(flame)},
        "coverage": {
            "exactGetDamageCallerBodies": 49,
            "coefficientArithmeticResolvedBeforeThisPass": 15,
            "coefficientArithmeticResolvedThisPass": 3,
            "remainingCoefficientArithmeticCallerBodies": 31,
            "semanticCallerResolution": "lower than arithmetic coverage because two native route identities and all three public skill-row mappings remain unresolved",
        },
        "unresolved": [
            "managed identities and parameter contracts of module targets 0x27e8964 and 0x32ff6cc",
            "public skill-row mappings for JDONOEEBDCD, BGIJEDLALGE, and BGHEAJHAICN",
            "why the same FEATHER_SHOT_POWER_VALUE field has exact package scales 0.01f and 1193.0f in sibling native callers",
            "why the DARK_RIFT_POWER_VALUE sibling uses exact package scale 1597.0f while BCLCCDFCHFC uses 0.01f",
            "coefficient producers and action routes for the remaining thirty-one caller bodies",
        ],
        "integrationStatus": "disconnected_no_literal_reinterpretation_or_live_combat_use",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--callers", type=Path, default=CALLERS)
    parser.add_argument("--actions", type=Path, default=ACTIONS)
    parser.add_argument("--constant-schema", type=Path, default=CONSTANT_SCHEMA)
    parser.add_argument("--xapk", type=Path, default=XAPK)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    args.output.write_text(
        json.dumps(build(args.callers, args.actions, args.constant_schema, args.xapk), ensure_ascii=True, indent=2) + "\n"
    )
    print(f"Wrote Hunter fixed-scale coefficient Pass15 evidence to {args.output}")


if __name__ == "__main__":
    main()
