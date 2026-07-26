#!/usr/bin/env python3
"""Extract Hunter generation evidence from protected IL2CPP metadata.

The 1.411 build poisons selected metadata identifiers, so this extractor does
not assume that every type name is trustworthy. It scores all Assembly-CSharp
types from their surviving type, field and method names and preserves the raw
records needed for later native correlation.
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
OUT = ROOT / "reverse-engineering/evidence/il2cpp-hunter-generation-v1.json"

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

METHOD_DEFINITION_SIZE_V39 = 30
METHOD_TOKEN_OFFSET_V39 = 18
STRONG_TERMS = (
    "hunter", "characteristic", "bodyindex", "hunterlook", "jobtrait",
    "costumehat", "weaponcostume", "unitcreate", "gradeRank",
)
FLOW_TERMS = (
    "fixrandomhunterbodyindex", "unithuntingrandom", "inithunter", "addhunter",
    "entryhunter", "waithunter", "createhunter", "randomhunter", "set_bodyindex",
    "set_character", "set_costume", "set_hat", "set_coshat",
    "set_weaponcostume",
)
SOURCE_TERMS = (
    "hunter", "unitcreate", "jobtrait", "personality", "characteristic",
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def normalized(value: str) -> str:
    return "".join(character for character in value.lower() if character.isalnum() or character == "_")


def source(path: Path) -> dict[str, object]:
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "bytes": path.stat().st_size,
        "sha256": sha256(path),
    }


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


def extract_ascii_catalog(payload: bytes) -> list[str]:
    pattern = rb"(?:\\Assets\\[^\x00]{3,220}|\|[A-Za-z_][A-Za-z0-9_.+`]{2,})\x00"
    values = []
    for match in re.finditer(pattern, payload):
        value = match.group(0)[:-1].decode("ascii", "replace")
        values.append(value[1:] if value.startswith("|") else value)
    return sorted(set(values))


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

    image = parse_first_image(data, sections)
    type_section = sections["typeDefinitions"]
    field_section = sections["fields"]
    method_section = sections["methods"]
    candidates = []

    for type_index in range(image["firstTypeIndex"], image["firstTypeIndex"] + image["typeCount"]):
        record = parse_type(data, type_section["offset"] + type_index * 76)
        name = read_string(record["nameIndex"])
        namespace = read_string(record["namespaceIndex"])
        fields = []
        methods = []

        for field_index in bounded_range(record["firstFieldIndex"], record["fieldCount"], field_section["count"]):
            name_index, type_index_raw, token = struct.unpack_from(
                "<iHI", data, field_section["offset"] + field_index * 10
            )
            fields.append({
                "index": field_index,
                "name": read_string(name_index),
                "il2cppTypeIndex": type_index_raw,
                "token": token,
            })

        for method_index in bounded_range(record["firstMethodIndex"], record["methodCount"], method_section["count"]):
            method_offset = method_section["offset"] + method_index * METHOD_DEFINITION_SIZE_V39
            name_index = struct.unpack_from("<i", data, method_offset)[0]
            token = struct.unpack_from("<I", data, method_offset + METHOD_TOKEN_OFFSET_V39)[0]
            methods.append({"index": method_index, "name": read_string(name_index), "token": token})

        type_text = normalized(str(name["value"] or ""))
        field_text = [normalized(str(item["name"]["value"] or "")) for item in fields]
        method_text = [normalized(str(item["name"]["value"] or "")) for item in methods]
        score = 0
        reasons = []

        type_hits = [term for term in STRONG_TERMS if normalized(term) in type_text]
        if type_hits:
            score += 20
            reasons.append({"kind": "type", "matches": type_hits})

        field_hits = sorted({term for value in field_text for term in STRONG_TERMS if normalized(term) in value})
        if field_hits:
            score += min(12, len(field_hits) * 3)
            reasons.append({"kind": "field", "matches": field_hits})

        flow_hits = sorted({term for value in method_text for term in FLOW_TERMS if normalized(term) in value})
        if flow_hits:
            score += len(flow_hits) * 6
            reasons.append({"kind": "method", "matches": flow_hits})

        hunter_method_hits = [value for value in method_text if "hunter" in value]
        if hunter_method_hits:
            score += min(10, len(hunter_method_hits) * 2)
            reasons.append({"kind": "hunter-method-count", "count": len(hunter_method_hits)})

        if score < 6:
            continue

        candidates.append({
            "typeIndex": type_index,
            "token": record["token"],
            "name": name,
            "namespace": namespace,
            "fieldCount": record["fieldCount"],
            "methodCount": record["methodCount"],
            "score": score,
            "reasons": reasons,
            "fields": fields,
            "methods": methods,
        })

    candidates.sort(key=lambda item: (-int(item["score"]), int(item["typeIndex"])))

    default_section = sections["fieldAndParameterDefaultValueData"]
    default_payload = data[default_section["offset"]:default_section["offset"] + default_section["size"]]
    clean_catalog = extract_ascii_catalog(default_payload)
    clean_type_names = [
        value for value in clean_catalog
        if not value.startswith("\\Assets\\") and any(normalized(term) in normalized(value) for term in SOURCE_TERMS)
    ]
    source_paths = [
        value for value in clean_catalog
        if value.startswith("\\Assets\\") and any(normalized(term) in normalized(value) for term in SOURCE_TERMS)
    ]

    output = {
        "schemaVersion": 1,
        "contractType": "il2cpp-hunter-generation-evidence",
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
            "status": "native bodies still require protected-v39 registration recovery",
        },
        "assemblyCSharpImage": image,
        "candidateTypes": candidates,
        "cleanNameCatalog": clean_type_names,
        "sourcePathCatalog": source_paths,
        "confirmedDataBoundaries": {
            "nameTableTypes": ["AdminHunterNameMData", "AdminHunterNameWData"],
            "quickSheetTypes": ["hunterNameM", "hunterNameMData", "hunterNameW", "hunterNameWData"],
            "generationTypes": ["HunterManager", "AdminHunterData", "AdminUnitCreateData", "BuildHunterData"],
            "instanceSnapshotTypes": ["HunterData", "HunterLookData"],
        },
        "limitations": [
            "Type, field and method identifiers may be poisoned; candidate scores are correlation evidence, not recovered source names.",
            "The presence of QuickSheet/admin table classes proves schema code exists but does not prove their row values are embedded in metadata.",
            "Exact RNG ranges and call order require native method-body correlation or a runtime trace.",
            "Portrait filename ranges must not be treated as body RNG ranges without code or serialized-data evidence.",
        ],
    }
    OUT.write_text(json.dumps(output, indent=2, ensure_ascii=True) + "\n")
    print(f"Wrote {len(candidates)} Hunter generation candidates to {OUT}")


if __name__ == "__main__":
    main()
