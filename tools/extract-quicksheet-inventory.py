#!/usr/bin/env python3
"""Inventory every serialized QuickSheet object in sharedassets1.assets.

This intentionally records identity and hashes only. Unknown worksheet layouts
must not be decoded with guessed field meanings.
"""

from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path

import UnityPy


ROOT = Path(__file__).resolve().parents[1]
ASSET = ROOT / "game-assets/extracted/joined_unity_files/sharedassets1.assets"
OUT = ROOT / "reverse-engineering/evidence/quicksheet-inventory-v1.json"

DECODED = {
    "met", "consum", "gearArmor", "gearBelt", "gearBoots", "gearGlove",
    "gearHelmet", "gearNeck", "gearRing", "gearWeapon", "runeCraft", "runes",
    "hunter", "hunterNameM", "hunterNameW", "personality", "evil", "dropUniqueGear",
    "exp", "skill", "subJobSkill", "growupProperty", "jobTrait", "ridingPet",
    "ridingPetSkill", "ridingPetTrait", "build", "buildSkin", "product", "tradeWagon",
}


def align4(value: int) -> int:
    return (value + 3) & ~3


def read_string(data: bytes, offset: int) -> tuple[str, int]:
    length = struct.unpack_from("<i", data, offset)[0]
    offset += 4
    if length < 0 or length > 100_000 or offset + length > len(data):
        raise ValueError("invalid Unity string")
    value = data[offset : offset + length].decode("utf-8")
    return value, align4(offset + length)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def parse_header(data: bytes) -> dict[str, object]:
    script_path_id = struct.unpack_from("<q", data, 20)[0]
    offset = 28
    name, offset = read_string(data, offset)
    spreadsheet_id, offset = read_string(data, offset)
    spreadsheet_name, offset = read_string(data, offset)
    worksheet_name, offset = read_string(data, offset)
    row_count = struct.unpack_from("<i", data, offset)[0]
    return {
        "name": name,
        "monoScriptPathId": script_path_id,
        "spreadsheetId": spreadsheet_id,
        "spreadsheetName": spreadsheet_name,
        "worksheetName": worksheet_name,
        "rowCount": row_count,
        "headerBytes": offset + 4,
    }


def main() -> None:
    environment = UnityPy.load(str(ASSET))
    records = []
    for obj in environment.objects:
        if obj.type.name != "MonoBehaviour":
            continue
        raw = obj.get_raw_data()
        try:
            header = parse_header(raw)
        except (UnicodeDecodeError, struct.error, ValueError):
            continue
        if header["spreadsheetName"] != "evilhunterdata_global":
            continue
        header["pathId"] = obj.path_id
        header["byteSize"] = len(raw)
        header["rawSha256"] = sha256(raw)
        header["decodeStatus"] = (
            "decoded-exact" if header["worksheetName"] in DECODED else "schema-unresolved"
        )
        records.append(header)

    records.sort(key=lambda item: int(item["pathId"]))
    result = {
        "schemaVersion": 1,
        "contractType": "quicksheet-inventory",
        "runtimeCompatibility": "evidence-only",
        "source": {
            "asset": ASSET.relative_to(ROOT).as_posix(),
            "assetSha256": sha256(ASSET.read_bytes()),
            "unityVersion": "6000.3.9f1",
        },
        "policy": "Headers and raw hashes are exact serialized evidence; unresolved rows are not assigned invented field meanings.",
        "counts": {
            "worksheets": len(records),
            "decodedExact": sum(item["decodeStatus"] == "decoded-exact" for item in records),
            "schemaUnresolved": sum(item["decodeStatus"] == "schema-unresolved" for item in records),
            "rows": sum(max(0, int(item["rowCount"])) for item in records),
        },
        "worksheets": records,
        "limitations": [
            "A worksheet header does not establish field semantics or runtime ownership.",
            "Rows marked schema-unresolved require a source-confirmed decoder or controlled runtime capture.",
        ],
    }
    OUT.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(f"Wrote {len(records)} QuickSheet records ({result['counts']['decodedExact']} exact, {result['counts']['schemaUnresolved']} unresolved) to {OUT}")


if __name__ == "__main__":
    main()
