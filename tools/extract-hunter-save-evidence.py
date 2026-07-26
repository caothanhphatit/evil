#!/usr/bin/env python3
"""Extract lightweight static evidence for the Hunter save boundary.

This does not decrypt a player save or infer field types from protected IL2CPP
indices. It records the surviving metadata around the known save/Hunter types
and confirms which generic persistence APIs are present in the packaged native
runtime.
"""

from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
METADATA = ROOT / "game-assets/source/unity-assets/bin/Data/Managed/Metadata/global-metadata.dat"
BINARY = ROOT / "reverse-engineering/native-libs/arm64-v8a/libil2cpp.so"
OUT = ROOT / "reverse-engineering/evidence/hunter-save-serialization-v1.json"

SECTION_NAMES = [
    "stringLiteral", "stringLiteralData", "string", "events", "properties",
    "methods", "parameterDefaultValues", "fieldDefaultValues",
    "fieldAndParameterDefaultValueData", "fieldMarshaledSizes", "parameters",
    "fields", "genericParameters", "genericParameterConstraints",
    "genericContainers", "nestedTypes", "interfaces", "vtableMethods",
    "interfaceOffsets", "typeDefinitions", "images", "assemblies", "fieldRefs",
    "referencedAssemblies", "attributeData", "attributeDataRange",
    "unresolvedVirtualCallParameterTypes", "unresolvedVirtualCallParameterRanges",
    "windowsRuntimeTypeNames", "windowsRuntimeStrings", "exportedTypeDefinitions",
]

TYPE_DEFINITION_SIZE_V39 = 76
FIELD_DEFINITION_SIZE_V39 = 10
METHOD_DEFINITION_SIZE_V39 = 30
METHOD_TOKEN_OFFSET_V39 = 18

TARGETS = {
    "userAggregate": {"typeIndex": 5, "expectedPrefix": "UserData"},
    "saveWrapper": {"typeIndex": 521, "expectedPrefix": "SaveData"},
    "hunterSnapshot": {"typeIndex": 1587, "expectedPrefix": "Hu"},
    "hunterLookProjection": {"typeIndex": 1972, "expectedPrefix": "HunterLoo"},
}

NATIVE_SYMBOLS = [
    "UnityEngine.Application::get_persistentDataPath_Injected",
    "UnityEngine.JsonUtility::FromJsonInternal_Injected",
    "UnityEngine.JsonUtility::ToJsonInternal_Injected",
    "UnityEngine.PlayerPrefs::DeleteAll()",
    "UnityEngine.PlayerPrefs::GetFloat_Injected",
    "UnityEngine.PlayerPrefs::GetInt_Injected",
    "UnityEngine.PlayerPrefs::GetString_Injected",
    "UnityEngine.PlayerPrefs::HasKey_Injected",
    "UnityEngine.PlayerPrefs::Save()",
    "UnityEngine.PlayerPrefs::TrySetInt_Injected",
    "UnityEngine.PlayerPrefs::TrySetSetString_Injected",
]

METADATA_MARKERS = [
    "CodeStage.AntiCheat.Storage|ObscuredFile",
    "CodeStage.AntiCheat.Storage|ObscuredFilePrefs",
    "CodeStage.AntiCheat.Storage|ObscuredPrefs",
    "Gpm.Common.ThirdParty.MessagePack|MessagePackSerializer",
    "Gpm.Common.Util|GpmMessagePackMapper",
    "\\Assets\\Scripts\\Data\\SaveData.cs",
]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def source(path: Path) -> dict[str, object]:
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "bytes": path.stat().st_size,
        "sha256": sha256(path),
    }


def main() -> None:
    data = METADATA.read_bytes()
    magic, version = struct.unpack_from("<II", data)
    if magic != 0xFAB11BAF or version != 39:
        raise ValueError(f"Expected IL2CPP metadata v39, got magic={magic:#x} version={version}")

    sections: dict[str, dict[str, int]] = {}
    header_offset = 8
    for name in SECTION_NAMES:
        section_offset, size, count = struct.unpack_from("<iii", data, header_offset)
        header_offset += 12
        sections[name] = {"offset": section_offset, "size": size, "count": count}

    string_section = sections["string"]

    def read_string(index: int) -> dict[str, object]:
        if index < 0 or index >= string_section["size"]:
            return {"index": index, "value": None, "status": "poisoned-index"}
        start = string_section["offset"] + index
        end = data.find(b"\0", start, string_section["offset"] + string_section["size"])
        if end < 0:
            return {"index": index, "value": None, "status": "unterminated"}
        raw = data[start:end]
        return {
            "index": index,
            "value": raw.decode("utf-8", "replace"),
            "status": "clean" if all(32 <= byte < 127 for byte in raw) else "obfuscated",
        }

    def parse_type(type_index: int) -> dict[str, object]:
        offset = sections["typeDefinitions"]["offset"] + type_index * TYPE_DEFINITION_SIZE_V39
        name_index, namespace_index = struct.unpack_from("<ii", data, offset)
        offset += 8 + 12
        indices = struct.unpack_from("<iiiiiiii", data, offset)
        offset += 32
        counts = struct.unpack_from("<HHHHHHHH", data, offset)
        offset += 16
        _, token = struct.unpack_from("<II", data, offset)

        fields = []
        for field_index in range(indices[0], indices[0] + counts[2]):
            field_offset = sections["fields"]["offset"] + field_index * FIELD_DEFINITION_SIZE_V39
            field_name_index, il2cpp_type_index, field_token = struct.unpack_from("<iHI", data, field_offset)
            fields.append({
                "index": field_index,
                "name": read_string(field_name_index),
                "il2cppTypeIndex": il2cpp_type_index,
                "token": field_token,
            })

        methods = []
        for method_index in range(indices[1], indices[1] + counts[0]):
            method_offset = sections["methods"]["offset"] + method_index * METHOD_DEFINITION_SIZE_V39
            method_name_index = struct.unpack_from("<i", data, method_offset)[0]
            method_token = struct.unpack_from("<I", data, method_offset + METHOD_TOKEN_OFFSET_V39)[0]
            methods.append({
                "index": method_index,
                "name": read_string(method_name_index),
                "token": method_token,
            })

        return {
            "typeIndex": type_index,
            "token": token,
            "name": read_string(name_index),
            "namespace": read_string(namespace_index),
            "fieldCount": counts[2],
            "methodCount": counts[0],
            "fields": fields,
            "methods": methods,
        }

    target_types = {}
    for role, target in TARGETS.items():
        record = parse_type(target["typeIndex"])
        name = str(record["name"]["value"] or "")
        if not name.startswith(target["expectedPrefix"]):
            raise ValueError(f"Type index {target['typeIndex']} no longer matches {role}: {name!r}")
        target_types[role] = record

    binary_payload = BINARY.read_bytes()
    native_symbols = [symbol for symbol in NATIVE_SYMBOLS if symbol.encode("ascii") in binary_payload]
    metadata_markers = [marker for marker in METADATA_MARKERS if marker.encode("ascii") in data]

    output = {
        "schemaVersion": 1,
        "contractType": "hunter-save-static-evidence",
        "runtimeCompatibility": "evidence-only",
        "sources": {
            "metadata": source(METADATA),
            "binary": source(BINARY),
            "metadataVersion": version,
            "unityVersion": "6000.3.9f1",
        },
        "targetTypes": target_types,
        "runtimeApiPresence": {
            "nativeSymbols": native_symbols,
            "metadataMarkers": metadata_markers,
            "interpretation": "API and library presence only; this does not prove which serializer SaveData invokes.",
        },
        "confirmedBoundaries": [
            "SaveData is a small wrapper with one serialized/backing field, not the full player aggregate.",
            "UserData is the large player aggregate and contains surviving EntryHunter, HunterPack, and SaveFormat fragments.",
            "HunterData is a per-instance snapshot carrying progression, live state, appearance, gear, item, consumable, skill, trait, and body-index fragments.",
            "HunterLookData is a separate compact appearance/look projection.",
        ],
        "limitations": [
            "The protected metadata poisons identifiers and selected method records; raw names are evidence fragments, not recovered source.",
            "il2cppTypeIndex values require native metadata-registration recovery before they can be mapped to trustworthy C# field types.",
            "No on-device PlayerPrefs XML, ACTk obscured file, cloud-save payload, or original account save is present in the repository.",
            "PlayerPrefs, JsonUtility, ACTk storage, and MessagePack are packaged dependencies; static presence does not identify the SaveData serialization path.",
            "No field ordering or type inferred here should be used as a binary save compatibility contract.",
        ],
    }
    OUT.write_text(json.dumps(output, indent=2, ensure_ascii=True) + "\n")
    print(f"Wrote Hunter save evidence for {len(target_types)} target types to {OUT}")


if __name__ == "__main__":
    main()
