#!/usr/bin/env python3
"""Extract building-related IL2CPP metadata without trusting poisoned indices.

The 1.411 Android build uses metadata v39 and deliberately corrupts selected
identifier records. This extractor keeps every raw record attributable to the
Assembly-CSharp image, marks malformed strings, and separately recovers clean
source-path/type-name catalogs embedded in the metadata default-value data.
"""

from __future__ import annotations

import hashlib
import json
import re
import struct
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
METADATA = ROOT / "game-assets/source/unity-assets/bin/Data/Managed/Metadata/global-metadata.dat"
BINARY = ROOT / "reverse-engineering/native-libs/arm64-v8a/libil2cpp.so"
OUT = ROOT / "reverse-engineering/evidence/il2cpp-building-metadata-v1.json"

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
KEYWORDS = (
    "build", "building", "tradewagon", "request", "gearcreate", "revive",
    "storage", "shop", "rune", "material", "weapon", "armor",
)
METHOD_DEFINITION_SIZE_V39 = 30
METHOD_TOKEN_OFFSET_V39 = 18


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def normalized(value: str) -> str:
    return "".join(character for character in value.lower() if character.isalnum())


def main() -> None:
    data = METADATA.read_bytes()
    magic, version = struct.unpack_from("<II", data)
    if magic != 0xFAB11BAF or version != 39:
        raise ValueError(f"Expected IL2CPP metadata v39, got magic={magic:#x} version={version}")

    sections = {}
    offset = 8
    for name in SECTION_NAMES:
        section_offset, size, count = struct.unpack_from("<iii", data, offset)
        offset += 12
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

    image = parse_first_image(data, sections)
    type_section = sections["typeDefinitions"]
    field_section = sections["fields"]
    method_section = sections["methods"]
    candidates = []

    for type_index in range(image["firstTypeIndex"], image["firstTypeIndex"] + image["typeCount"]):
        record = parse_type(data, type_section["offset"] + type_index * 76)
        name = read_string(record["nameIndex"])
        namespace = read_string(record["namespaceIndex"])
        searchable = normalized(str(name["value"] or ""))
        if not any(keyword in searchable for keyword in KEYWORDS):
            continue

        fields = []
        for field_index in bounded_range(record["firstFieldIndex"], record["fieldCount"], field_section["count"]):
            name_index, type_index_raw, token = struct.unpack_from("<iHI", data, field_section["offset"] + field_index * 10)
            fields.append({
                "index": field_index,
                "name": read_string(name_index),
                "il2cppTypeIndex": type_index_raw,
                "token": token,
            })

        methods = []
        for method_index in bounded_range(record["firstMethodIndex"], record["methodCount"], method_section["count"]):
            method_offset = method_section["offset"] + method_index * METHOD_DEFINITION_SIZE_V39
            name_index = struct.unpack_from("<i", data, method_offset)[0]
            # Metadata v39 uses 2-byte declaring/return/generic-container indices
            # and a 4-byte parameter index, placing the token at byte 18.
            token = struct.unpack_from("<I", data, method_offset + METHOD_TOKEN_OFFSET_V39)[0]
            methods.append({"index": method_index, "name": read_string(name_index), "token": token})

        candidates.append({
            "typeIndex": type_index,
            "token": record["token"],
            "name": name,
            "namespace": namespace,
            "fields": fields,
            "methods": methods,
        })

    default_data = sections["fieldAndParameterDefaultValueData"]
    default_bytes = data[default_data["offset"]:default_data["offset"] + default_data["size"]]
    clean_type_names = sorted(set(match.group(1).decode("ascii") for match in re.finditer(rb"\|([A-Za-z_][A-Za-z0-9_.+`]{2,})\x00", default_bytes)))
    source_paths = sorted(set(match.group(1).decode("ascii") for match in re.finditer(rb"(\\Assets\\Scripts\\[^\x00]+?\.cs)\x00", default_bytes)))
    relevant_clean_names = [value for value in clean_type_names if any(keyword in normalized(value) for keyword in KEYWORDS)]
    relevant_source_paths = [value for value in source_paths if any(keyword in normalized(value) for keyword in KEYWORDS)]

    output = {
        "schemaVersion": 1,
        "contractType": "il2cpp-building-metadata-evidence",
        "runtimeCompatibility": "evidence-only",
        "sources": {
            "metadata": source(METADATA),
            "binary": source(BINARY),
            "unityVersion": "6000.3.9f1",
            "metadataVersion": version,
        },
        "binaryRegistrationEvidence": {
            "codeRegistrationVirtualAddress": "0x5D99D98",
            "metadataRegistrationVirtualAddress": "0x5F3EC28",
            "assemblyCSharpMethodPointers": 38247,
            "recoveredWith": "LibCpp2IL main 01c1748, tolerant poisoned-record reads",
        },
        "assemblyCSharpImage": image,
        "candidateTypes": candidates,
        "cleanNameCatalog": relevant_clean_names,
        "sourcePathCatalog": relevant_source_paths,
        "limitations": [
            "Selected metadata names and indices are intentionally poisoned; obfuscated values are evidence, not runtime identifiers.",
            "Clean type-name/source-path catalogs are recovered but not yet correlated one-to-one with obfuscated type records.",
            "Native method bodies and serialized AdminBuildData rows require the next correlation pass.",
        ],
    }
    OUT.write_text(json.dumps(output, indent=2, ensure_ascii=True) + "\n")
    print(f"Wrote {len(candidates)} IL2CPP building candidates to {OUT}")


def source(path: Path) -> dict[str, object]:
    return {"path": path.relative_to(ROOT).as_posix(), "bytes": path.stat().st_size, "sha256": sha256(path)}


def bounded_range(start: int, count: int, maximum: int) -> range:
    if start < 0 or count <= 0 or start >= maximum:
        return range(0)
    return range(start, min(start + count, maximum))


def parse_first_image(data: bytes, sections: dict[str, dict[str, int]]) -> dict[str, int]:
    offset = sections["images"]["offset"]
    name_index, assembly_index = struct.unpack_from("<ii", data, offset)
    offset += 8
    first_type_index = struct.unpack_from("<H", data, offset)[0]
    offset += 2
    type_count = struct.unpack_from("<I", data, offset)[0]
    offset += 4
    offset += 2 + 4 + 4 + 4
    custom_attribute_start, custom_attribute_count = struct.unpack_from("<iI", data, offset)
    return {
        "imageIndex": 0,
        "nameIndex": name_index,
        "assemblyIndex": assembly_index,
        "firstTypeIndex": first_type_index,
        "typeCount": type_count,
        "customAttributeStart": custom_attribute_start,
        "customAttributeCount": custom_attribute_count,
    }


def parse_type(data: bytes, offset: int) -> dict[str, int]:
    name_index, namespace_index = struct.unpack_from("<ii", data, offset)
    offset += 8
    offset += 2 + 2 + 2 + 2 + 4
    indices = struct.unpack_from("<iiiiiiii", data, offset)
    offset += 32
    counts = struct.unpack_from("<HHHHHHHH", data, offset)
    offset += 16
    _, token = struct.unpack_from("<II", data, offset)
    return {
        "nameIndex": name_index,
        "namespaceIndex": namespace_index,
        "firstFieldIndex": indices[0],
        "firstMethodIndex": indices[1],
        "methodCount": counts[0],
        "fieldCount": counts[2],
        "token": token,
    }


if __name__ == "__main__":
    main()
