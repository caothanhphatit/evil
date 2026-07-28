#!/usr/bin/env python3
"""Normalize exact-boundary ARM64 combat captures into deterministic evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CAPTURE = (
    ROOT
    / "reverse-engineering/evidence/original-native-combat-formula-exact-boundaries-api35-v1.json"
)
DEFAULT_SCHEMAS = [
    ROOT / "reverse-engineering/evidence/evil-ai-drop-runtime-schema-android-api35-v1.json",
    ROOT / "reverse-engineering/evidence/hunter-manager-runtime-schema-android-api30-v1.json",
]
DEFAULT_OUTPUT = (
    ROOT / "reverse-engineering/evidence/original-native-combat-formula-analysis-v1.json"
)


FLOAT_BINARY_OPS = {
    0x1E200800: "fmul",
    0x1E201800: "fdiv",
    0x1E202800: "fadd",
    0x1E203800: "fsub",
    0x1E204800: "fmax",
    0x1E205800: "fmin",
}

UNSIGNED_MEMORY_OPS = {
    0xB9400000: ("ldr_w", 4),
    0xB9000000: ("str_w", 4),
    0xF9400000: ("ldr_x", 8),
    0xF9000000: ("str_x", 8),
    0xBD400000: ("ldr_s", 4),
    0xBD000000: ("str_s", 4),
    0xFD400000: ("ldr_d", 8),
    0xFD000000: ("str_d", 8),
}

ZERO_FLOAT_BODY = bytes.fromhex("00e4002fc0035fd6")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture", type=Path, default=DEFAULT_CAPTURE)
    parser.add_argument("--schema", type=Path, action="append", dest="schemas")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sign_extend(value: int, bits: int) -> int:
    sign = 1 << (bits - 1)
    return (value ^ sign) - sign


def float_immediate(imm8: int) -> float:
    sign = (imm8 >> 7) & 1
    exponent_bit = (imm8 >> 6) & 1
    exponent = (
        ((1 - exponent_bit) << 7)
        | ((0x1F if exponent_bit else 0) << 2)
        | ((imm8 >> 4) & 0x3)
    )
    fraction = (imm8 & 0xF) << 19
    bits = (sign << 31) | (exponent << 23) | fraction
    return struct.unpack(">f", struct.pack(">I", bits))[0]


def iter_words(body: bytes):
    if len(body) % 4:
        raise ValueError(f"ARM64 body length is not word aligned: {len(body)}")
    for offset in range(0, len(body), 4):
        yield offset, int.from_bytes(body[offset : offset + 4], "little")


def schema_fields(paths: list[Path]) -> dict[str, dict[int, list[dict[str, Any]]]]:
    result: dict[str, dict[int, list[dict[str, Any]]]] = {}

    def walk(value: Any) -> None:
        if isinstance(value, dict):
            name = value.get("name")
            fields = value.get("fields")
            if isinstance(name, str) and isinstance(fields, list):
                by_offset = result.setdefault(name, {})
                for field in fields:
                    if not isinstance(field, dict) or not isinstance(field.get("offset"), int):
                        continue
                    record = {"name": field.get("name"), "type": field.get("type")}
                    if record not in by_offset.setdefault(field["offset"], []):
                        by_offset[field["offset"]].append(record)
            for child in value.values():
                walk(child)
        elif isinstance(value, list):
            for child in value:
                walk(child)

    for path in paths:
        walk(json.loads(path.read_text()))
    return result


def candidate_body(method: dict[str, Any]) -> tuple[dict[str, Any], bytes]:
    candidates = method.get("candidates", [])
    if not candidates:
        raise ValueError(f"No candidate for {method['className']}.{method['methodName']}")
    first = candidates[0]
    body = bytes.fromhex(first["codeHex"])
    for candidate in candidates[1:]:
        if candidate.get("moduleOffset") != first.get("moduleOffset"):
            raise ValueError("methodPointer and virtualMethodPointer offsets disagree")
        if candidate.get("codeHex") != first.get("codeHex"):
            raise ValueError("methodPointer and virtualMethodPointer bodies disagree")
    if not first.get("codeTruncated") and len(body) != first.get("nativeSizeBytes"):
        raise ValueError("captured native size does not match exact body")
    if first.get("codeTruncated") and len(body) >= first.get("nativeSizeBytes"):
        raise ValueError("truncated capture length is inconsistent with the exact boundary")
    return first, body


def decode_float_constants(body: bytes) -> list[dict[str, Any]]:
    constants = []
    for offset, word in iter_words(body):
        if word & 0xFF601FE0 != 0x1E201000:
            continue
        constants.append(
            {
                "instructionOffset": offset,
                "register": word & 0x1F,
                "value": float_immediate((word >> 13) & 0xFF),
            }
        )
    return constants


def decode_float_arithmetic(body: bytes) -> list[dict[str, Any]]:
    operations = []
    for offset, word in iter_words(body):
        operation = FLOAT_BINARY_OPS.get(word & 0xFF20FC00)
        if operation is None or word & 0x00400000:
            continue
        operations.append(
            {
                "instructionOffset": offset,
                "operation": operation,
                "destination": word & 0x1F,
                "left": (word >> 5) & 0x1F,
                "right": (word >> 16) & 0x1F,
            }
        )
    return operations


def decode_memory_accesses(
    body: bytes, type_name: str, fields: dict[str, dict[int, list[dict[str, Any]]]]
) -> list[dict[str, Any]]:
    accesses = []
    for instruction_offset, word in iter_words(body):
        decoded = UNSIGNED_MEMORY_OPS.get(word & 0xFFC00000)
        if decoded is None:
            continue
        operation, scale = decoded
        base_register = (word >> 5) & 0x1F
        field_offset = ((word >> 10) & 0xFFF) * scale
        if base_register != 0:
            continue
        accesses.append(
            {
                "instructionOffset": instruction_offset,
                "operation": operation,
                "baseRegister": "x0",
                "offset": field_offset,
                "schemaFields": fields.get(type_name, {}).get(field_offset, []),
            }
        )
    return accesses


def decode_calls(
    body: bytes, method_offset: int, starts: dict[int, list[str]]
) -> list[dict[str, Any]]:
    calls: dict[tuple[int, tuple[str, ...]], int] = {}
    for instruction_offset, word in iter_words(body):
        if word & 0xFC000000 != 0x94000000:
            continue
        delta = sign_extend(word & 0x03FFFFFF, 26) << 2
        target = method_offset + instruction_offset + delta
        names = tuple(sorted(starts.get(target, [])))
        calls[(target, names)] = calls.get((target, names), 0) + 1
    return [
        {
            "targetModuleOffset": f"0x{target:x}",
            "resolvedMethods": list(names),
            "count": count,
        }
        for (target, names), count in sorted(calls.items())
    ]


def normalized_call_hash(body: bytes) -> str:
    normalized = bytearray(body)
    for offset, word in iter_words(body):
        if word & 0xFC000000 == 0x94000000:
            normalized[offset : offset + 4] = (0x94000000).to_bytes(4, "little")
    return sha256(bytes(normalized))


def build(capture_path: Path, schema_paths: list[Path]) -> dict[str, Any]:
    capture = json.loads(capture_path.read_text())
    payload = capture["record"]["payload"]
    if payload.get("exactBoundaries") is not True:
        raise ValueError("capture is not marked exactBoundaries=true")
    methods = payload["methods"]
    if len(methods) != 16:
        raise ValueError(f"expected 16 combat methods, found {len(methods)}")

    fields = schema_fields(schema_paths)
    starts: dict[int, list[str]] = {}
    bodies: dict[tuple[str, str], bytes] = {}
    candidates: dict[tuple[str, str], dict[str, Any]] = {}
    for method in methods:
        candidate, body = candidate_body(method)
        key = (method["className"], method["methodName"])
        bodies[key] = body
        candidates[key] = candidate
        start = int(candidate["moduleOffset"], 16)
        starts.setdefault(start, []).append(".".join(key))

    analyzed = []
    for method in methods:
        key = (method["className"], method["methodName"])
        candidate = candidates[key]
        body = bodies[key]
        start = int(candidate["moduleOffset"], 16)
        arithmetic = decode_float_arithmetic(body)
        signature = ",".join(item["operation"] for item in arithmetic)
        declared_size = candidate["nativeSizeBytes"]
        descriptor = json.dumps(
            {
                "type": key[0],
                "method": key[1],
                "token": method["token"],
                "moduleOffset": candidate["moduleOffset"],
                "boundaryModuleOffset": candidate["boundaryModuleOffset"],
                "nativeSizeBytes": declared_size,
                "capturedCodeSha256": sha256(body),
            },
            sort_keys=True,
            separators=(",", ":"),
        ).encode()
        analyzed.append(
            {
                "type": key[0],
                "method": key[1],
                "parameterCount": method["parameterCount"],
                "parameterTypes": method["parameterTypes"],
                "returnType": method["returnType"],
                "token": f"0x{method['token']:08X}",
                "moduleOffset": candidate["moduleOffset"],
                "boundaryModuleOffset": candidate["boundaryModuleOffset"],
                "nativeSizeBytes": declared_size,
                "capturedCodeBytes": len(body),
                "codeTruncated": candidate["codeTruncated"],
                "bodySha256": None if candidate["codeTruncated"] else sha256(body),
                "capturedCodeSha256": sha256(body),
                "exactBoundaryDescriptorSha256": sha256(descriptor),
                "callTargetNormalizedSha256": normalized_call_hash(body),
                "directCalls": decode_calls(body, start, starts),
                "selfFieldAccesses": decode_memory_accesses(body, key[0], fields),
                "floatImmediateConstants": decode_float_constants(body),
                "floatArithmetic": arithmetic,
                "floatArithmeticSignature": signature,
                "floatArithmeticSignatureSha256": sha256(signature.encode()),
            }
        )

    by_key = {(row["type"], row["method"]): row for row in analyzed}
    reduce_row = by_key[("EvilCtrl", "GetReduceAttackValue")]
    reduce_offsets = [
        access["offset"]
        for access in reduce_row["selfFieldAccesses"]
        if access["operation"] == "ldr_s"
    ]
    reduce_signature = reduce_row["floatArithmeticSignature"]
    if reduce_row["nativeSizeBytes"] != 88:
        raise ValueError("GetReduceAttackValue exact boundary changed from 88 bytes")
    if reduce_offsets != [484, 492, 500]:
        raise ValueError(f"GetReduceAttackValue field offsets changed: {reduce_offsets}")
    if reduce_signature != "fsub,fsub,fsub,fmul,fmul":
        raise ValueError(f"GetReduceAttackValue arithmetic changed: {reduce_signature}")

    costume_facts = {}
    for method_name in ["GetCostumeAttackUp", "GetCostumeArmorUp"]:
        body = bodies[("GameManager", method_name)]
        if body != ZERO_FLOAT_BODY:
            raise ValueError(f"{method_name} no longer has the exact zero-return body")
        costume_facts[method_name] = {
            "nativeSizeBytes": len(body),
            "returns": 0.0,
            "bodySha256": sha256(body),
        }

    critical_row = by_key[("HunterCtrl", "getCriticalDamage")]
    critical_constants = [
        constant for constant in critical_row["floatImmediateConstants"] if constant["value"] == 1.75
    ]
    if not critical_constants:
        raise ValueError("getCriticalDamage no longer contains the native 1.75 base")

    return {
        "schemaVersion": 1,
        "contractType": "original-native-combat-formula-analysis",
        "runtimeCompatibility": "evidence-only",
        "source": {
            "path": capture_path.relative_to(ROOT).as_posix(),
            "sha256": sha256(capture_path.read_bytes()),
            "schemas": [
                {"path": path.relative_to(ROOT).as_posix(), "sha256": sha256(path.read_bytes())}
                for path in schema_paths
            ],
        },
        "capture": capture["capture"],
        "module": payload["module"],
        "methodBoundaryRule": (
            "Every analyzed boundary ends at the next unique Assembly-CSharp method pointer "
            "and has matching methodPointer and virtualMethodPointer bytes. One method body "
            "exceeds the capture byte limit; its exact size and prefix are recorded separately."
        ),
        "normalization": {
            "bodySha256": "SHA-256 of the complete exact native body, or null for a truncated prefix.",
            "exactBoundaryDescriptorSha256": (
                "SHA-256 of stable method identity, exact start/end/size, and captured-code hash."
            ),
            "callTargetNormalizedSha256": (
                "SHA-256 after replacing each direct BL immediate with opcode 0x94000000."
            ),
            "fieldAccessRule": (
                "Only unsigned-immediate scalar loads/stores whose base is entry register x0 "
                "are promoted as direct self-field accesses."
            ),
        },
        "methods": analyzed,
        "recoveredExactFacts": {
            "getReduceAttackValue": {
                "nativeSizeBytes": 88,
                "fieldOffsets": reduce_offsets,
                "schemaFields": [
                    access["schemaFields"]
                    for access in reduce_row["selfFieldAccesses"]
                    if access["operation"] == "ldr_s"
                ],
                "floatImmediate": 1.0,
                "arithmeticSignature": reduce_signature,
                "expression": "(1 - field_484) * (1 - field_492) * (1 - field_500)",
            },
            "costumeModifiers": costume_facts,
            "criticalDamage": {
                "baseMultiplier": 1.75,
                "instructionOffsets": [row["instructionOffset"] for row in critical_constants],
                "note": "The complete surrounding modifier and rounding chain remains unresolved.",
            },
        },
        "limitations": [
            "Resolved direct-call names cover only the sixteen captured exact method starts; other targets remain module offsets.",
            "HunterCtrl.Damaged exceeds the 16,384-byte capture limit; its exact boundary is known but arithmetic/call analysis covers only the captured prefix.",
            "Direct self-field accesses intentionally exclude pointers copied from x0 into other registers.",
            "Arithmetic signatures describe decoded scalar single-precision instructions, not semantic variable names.",
            "No formula is authorized for runtime use until all inputs and golden vectors are independently resolved.",
        ],
    }


def main() -> None:
    args = parse_args()
    schema_paths = args.schemas or DEFAULT_SCHEMAS
    evidence = build(args.capture, schema_paths)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote {args.output}")


if __name__ == "__main__":
    main()
