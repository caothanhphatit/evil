#!/usr/bin/env python3
"""Normalize bounded IL2CPP AI method dumps into reviewable evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path


TARGETS = (
    ("EvilCtrl", "FixedUpdate"),
    ("EvilCtrl", "Move"),
    ("EvilCtrl", "FsmMoveEnd"),
    ("EvilCtrl", "UnitAttack"),
    ("EvilCtrl", "Dead"),
    ("HunterCtrl", "Hunting"),
    ("HunterCtrl", "HuntingFirst"),
    ("HunterCtrl", "HuntingSecond"),
    ("HunterCtrl", "HuntingAttackSetting"),
    ("HunterCtrl", "HuntingAttackAction"),
)

CALL_RE = re.compile(r"; call ([^/]+)/([0-9]+)")
FIELD_RE = re.compile(r"; x[0-9]+\.([A-Za-z0-9_<>]+)")
ADDRESS_RE = re.compile(r"0x[0-9a-fA-F]+")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture", type=Path, required=True)
    parser.add_argument("--binary-dir", type=Path, required=True)
    parser.add_argument("--disassembly-dir", type=Path, required=True)
    parser.add_argument("--schema", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--disassembly-output", type=Path, required=True)
    return parser.parse_args()


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def normalize_addresses(text: str, base: int, size: int) -> str:
    def replace(match: re.Match[str]) -> str:
        address = int(match.group(0), 16)
        if base <= address < base + size:
            return f"libil2cpp+0x{address - base:x}"
        return match.group(0).lower()

    return ADDRESS_RE.sub(replace, text)


def schema_field_maps(schema: dict[str, object]) -> dict[str, dict[str, dict[str, object]]]:
    result: dict[str, dict[str, dict[str, object]]] = {}

    def walk(value: object) -> None:
        if isinstance(value, dict):
            name = value.get("name")
            fields = value.get("fields")
            if isinstance(name, str) and isinstance(fields, list):
                result[name] = {
                    field["name"]: {
                        "offset": field.get("offset"),
                        "type": field.get("type"),
                    }
                    for field in fields
                    if isinstance(field, dict) and isinstance(field.get("name"), str)
                }
            for nested in value.values():
                walk(nested)
        elif isinstance(value, list):
            for nested in value:
                walk(nested)

    walk(schema)
    return result


def main() -> None:
    args = parse_args()
    capture = json.loads(args.capture.read_text())
    schema = json.loads(args.schema.read_text())
    payload = capture["record"]["payload"]
    module = payload["module"]
    module_base = int(module["base"], 16)
    module_size = int(module["size"])
    captured = {
        (method["className"], method["methodName"]): method
        for method in payload["methods"]
    }
    fields_by_type = schema_field_maps(schema)

    methods = []
    disassembly_sections = []
    for class_name, method_name in TARGETS:
        key = (class_name, method_name)
        if key not in captured:
            raise ValueError(f"capture is missing {class_name}.{method_name}")
        stem = f"{class_name}.{method_name}"
        binary_path = args.binary_dir / f"{stem}.bin"
        disassembly_path = args.disassembly_dir / f"{stem}.annotated.txt"
        body = binary_path.read_bytes()
        disassembly = disassembly_path.read_text()
        normalized = normalize_addresses(disassembly, module_base, module_size)
        disassembly_sections.append(
            f"## {stem}\n\n"
            f"token: 0x{captured[key]['token']:08x}\n"
            f"moduleOffset: {captured[key]['candidates'][0]['moduleOffset']}\n"
            f"nativeSizeBytes: {len(body)}\n\n"
            f"{normalized.rstrip()}\n"
        )

        calls: dict[str, int] = {}
        for call in CALL_RE.findall(disassembly):
            calls[call[0]] = calls.get(call[0], 0) + 1
        field_names = sorted(set(FIELD_RE.findall(disassembly)))
        field_map = fields_by_type.get(class_name, {})
        field_references = [
            {
                "name": name,
                **field_map.get(name, {"offset": None, "type": None}),
            }
            for name in field_names
        ]
        method = captured[key]
        methods.append(
            {
                "type": class_name,
                "method": method_name,
                "parameterCount": method["parameterCount"],
                "parameterTypes": method["parameterTypes"],
                "returnType": method["returnType"],
                "token": f"0x{method['token']:08X}",
                "moduleOffset": method["candidates"][0]["moduleOffset"],
                "nativeSizeBytes": len(body),
                "bodySha256": sha256(body),
                "knownDirectCalls": [
                    {"method": name, "count": count}
                    for name, count in sorted(calls.items())
                ],
                "schemaFieldReferences": field_references,
            }
        )

    evidence = {
        "schemaVersion": 1,
        "contractType": "original-native-ai-runtime-evidence",
        "runtimeCompatibility": "evidence-only",
        "capture": capture["capture"],
        "module": module,
        "methodBoundaryRule": (
            "Each body was read from the live decrypted method pointer through the nearest "
            "higher unique Assembly-CSharp method pointer; aliases that share the same pointer "
            "share one native body and do not create a boundary. Module offsets are relative "
            "to the live libil2cpp mapping base."
        ),
        "schemaSource": args.schema.as_posix(),
        "methods": methods,
        "limitations": [
            "Direct calls are named only when the live HunterCtrl/EvilCtrl method map resolves the target.",
            "Indirect calls and external Unity/runtime helpers remain unresolved unless separately captured.",
            "Field references identify exact offsets and types, but obfuscated field semantics are not invented.",
            "Raw decrypted native method bytes are not embedded; the record retains bounded size and SHA-256 evidence.",
            "The state-machine interpretation belongs in the companion migration report, not in this mechanical record.",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    args.disassembly_output.parent.mkdir(parents=True, exist_ok=True)
    args.disassembly_output.write_text("\n".join(disassembly_sections))


if __name__ == "__main__":
    main()
