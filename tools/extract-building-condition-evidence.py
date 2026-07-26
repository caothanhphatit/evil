#!/usr/bin/env python3
"""Extract the serialized Town Hall condition contract used by building levels."""

from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path

import UnityPy


ROOT = Path(__file__).resolve().parents[1]
LOCAL_UI_ASSET = ROOT / "game-assets/extracted/joined_unity_files/sharedassets0.assets"
BUILDING_TABLES = ROOT / "reverse-engineering/evidence/serialized-building-tables-v1.json"
OUT = ROOT / "reverse-engineering/evidence/building-condition-evidence-v1.json"
LOCAL_UI_PATH_ID = 9082
LOCALES = ("ko", "ja", "en", "zh-TW", "zh-CN", "ru", "fr", "es", "pt", "it", "de", "th", "vi", "id")
TARGET_KEYS = {"buildpop_9", "buildtoast_0"}


def align4(offset: int) -> int:
    return (offset + 3) & ~3


class Reader:
    def __init__(self, data: bytes, offset: int = 0) -> None:
        self.data = data
        self.offset = offset

    def int32(self) -> int:
        value = struct.unpack_from("<i", self.data, self.offset)[0]
        self.offset += 4
        return value

    def string(self) -> str:
        length = self.int32()
        end = self.offset + length
        value = self.data[self.offset:end].decode("utf-8")
        self.offset = align4(end)
        return value


def sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def decode_local_ui(data: bytes) -> tuple[dict, list[dict]]:
    script_path_id = struct.unpack_from("<q", data, 20)[0]
    reader = Reader(data, 28)
    header = {
        "name": reader.string(),
        "monoScriptPathId": script_path_id,
        "spreadsheetId": reader.string(),
        "spreadsheetName": reader.string(),
        "worksheetName": reader.string(),
        "rowCount": reader.int32(),
    }
    rows = []
    for _ in range(header["rowCount"]):
        index = reader.int32()
        key = reader.string()
        explanation = reader.string()
        localized = {locale: reader.string() for locale in LOCALES}
        if key in TARGET_KEYS:
            rows.append({"index": index, "key": key, "explanationKo": explanation, "localized": localized})
    if reader.offset != len(data):
        raise ValueError(f"localUI trailing bytes: {len(data) - reader.offset}")
    return header, rows


def main() -> None:
    environment = UnityPy.load(str(LOCAL_UI_ASSET))
    obj = next(entry for entry in environment.objects if entry.path_id == LOCAL_UI_PATH_ID)
    raw = obj.get_raw_data()
    header, ui_rows = decode_local_ui(raw)
    tables_payload = BUILDING_TABLES.read_bytes()
    tables = json.loads(tables_payload)
    condition_rows = [
        {
            "key": f"build_{building['index']}:level:{level_index + 1}",
            "buildingId": building["index"],
            "buildingLevel": level_index + 1,
            "requiredTownHallLevel": required_level,
            "sourceLocator": f"buildings[index={building['index']}].possibleBuild[{level_index}]",
        }
        for building in tables["buildings"]
        for level_index, required_level in enumerate(building["possibleBuild"])
    ]
    output = {
        "schemaVersion": 1,
        "contractType": "building-town-hall-condition-evidence",
        "sources": {
            "localUi": {
                "path": LOCAL_UI_ASSET.relative_to(ROOT).as_posix(),
                "pathId": LOCAL_UI_PATH_ID,
                "assetSha256": sha256(LOCAL_UI_ASSET.read_bytes()),
                "rawBytes": len(raw),
                "rawSha256": sha256(raw),
                "header": header,
            },
            "buildingTables": {
                "path": BUILDING_TABLES.relative_to(ROOT).as_posix(),
                "bytes": len(tables_payload),
                "sha256": sha256(tables_payload),
            },
        },
        "evaluator": {
            "subjectId": "build_1.level",
            "operator": "greater-than-or-equal",
            "operandField": "AdminBuildData.possibleBuild[levelIndex]",
            "confidence": "strongly-inferred",
            "reason": "Both recovered build-condition UI rows format the operand as a minimum Town Hall level.",
        },
        "localizationRows": sorted(ui_rows, key=lambda row: row["index"]),
        "conditionRows": condition_rows,
    }
    OUT.write_text(json.dumps(output, ensure_ascii=False, indent=2) + "\n")
    print(f"Wrote {len(condition_rows)} Town Hall condition rows to {OUT}")


if __name__ == "__main__":
    main()
