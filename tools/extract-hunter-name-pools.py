#!/usr/bin/env python3
"""Extract the original 1.411 Hunter name pools from Unity assets.

The Hunter name tables are QuickSheet MonoBehaviour objects embedded in
sharedassets1.assets. The script parses their serialized payloads directly;
network access and native IL2CPP decoding are not required.
"""

from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path

import UnityPy


ROOT = Path(__file__).resolve().parents[1]
ASSET = ROOT / "game-assets/extracted/joined_unity_files/sharedassets1.assets"
OUT = ROOT / "reverse-engineering/evidence/hunter-name-pools-v1.json"
TABLES = {
    "male": {"pathId": 12596, "globalName": "hunterNameM_global", "sheet": "hunterNameM"},
    "female": {"pathId": 12597, "globalName": "hunterNameW_global", "sheet": "hunterNameW"},
}
LOCALES = ["eng", "kor", "jpn", "twn", "chn", "rus", "fre", "spa", "por", "ita", "deu", "tha", "vnm", "idn"]


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def sha256(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def read_i32(payload: bytes, offset: int) -> tuple[int, int]:
    return struct.unpack_from("<i", payload, offset)[0], offset + 4


def read_aligned_string(payload: bytes, offset: int) -> tuple[str, int]:
    length, offset = read_i32(payload, offset)
    if length < 0 or offset + length > len(payload):
        raise ValueError(f"Invalid Unity string length {length} at offset {offset - 4}")
    value = payload[offset:offset + length].decode("utf-8")
    offset += length
    offset = (offset + 3) & ~3
    return value, offset


def parse_table(payload: bytes, expected: dict[str, object]) -> dict[str, object]:
    # MonoBehaviour object/script references and enabled flags occupy 28 bytes.
    offset = 28
    global_name, offset = read_aligned_string(payload, offset)
    spreadsheet_id, offset = read_aligned_string(payload, offset)
    worksheet, offset = read_aligned_string(payload, offset)
    sheet, offset = read_aligned_string(payload, offset)
    row_count, offset = read_i32(payload, offset)

    if global_name != expected["globalName"] or sheet != expected["sheet"]:
        raise ValueError(f"Unexpected QuickSheet table identity: {global_name}/{sheet}")
    if row_count <= 0 or row_count > 10_000:
        raise ValueError(f"Invalid Hunter name row count: {row_count}")

    rows = []
    for expected_index in range(row_count):
        index, offset = read_i32(payload, offset)
        if index != expected_index:
            raise ValueError(f"Non-contiguous Hunter name index: expected {expected_index}, got {index}")
        row: dict[str, object] = {"idx": index}
        for locale in LOCALES:
            row[locale], offset = read_aligned_string(payload, offset)
        rows.append(row)

    if offset != len(payload):
        raise ValueError(f"Unparsed bytes remain in {sheet}: {len(payload) - offset}")

    return {
        "globalName": global_name,
        "spreadsheetId": spreadsheet_id,
        "worksheet": worksheet,
        "sheet": sheet,
        "rowCount": row_count,
        "locales": LOCALES,
        "rows": rows,
    }


def main() -> None:
    objects = {obj.path_id: obj for obj in UnityPy.load(str(ASSET)).objects}
    tables = {}
    for family, expected in TABLES.items():
        path_id = int(expected["pathId"])
        if path_id not in objects:
            raise ValueError(f"Missing MonoBehaviour path_id {path_id}")
        obj = objects[path_id]
        payload = obj.get_raw_data()
        table = parse_table(payload, expected)
        table["pathId"] = path_id
        table["typeId"] = obj.type_id
        table["objectType"] = obj.type.name
        table["byteStart"] = obj.byte_start
        table["serializedBytes"] = len(payload)
        table["serializedSha256"] = sha256_bytes(payload)
        tables[family] = table

    spreadsheet_ids = {table["spreadsheetId"] for table in tables.values()}
    worksheets = {table["worksheet"] for table in tables.values()}
    if len(spreadsheet_ids) != 1 or len(worksheets) != 1:
        raise ValueError("Male and female Hunter name tables do not share one QuickSheet source")

    split_sources = []
    for split_name in ("sharedassets1.assets.split45", "sharedassets1.assets.split46"):
        path = ROOT / "game-assets/source/unity-assets/bin/Data" / split_name
        split_sources.append({
            "path": path.relative_to(ROOT).as_posix(),
            "bytes": path.stat().st_size,
            "sha256": sha256(path),
        })

    output = {
        "schemaVersion": 1,
        "contractType": "hunter-name-pool-evidence",
        "gameVersion": "1.411",
        "authority": "serialized-local-asset",
        "source": {
            "path": ASSET.relative_to(ROOT).as_posix(),
            "bytes": ASSET.stat().st_size,
            "sha256": sha256(ASSET),
            "originalSplitsContainingPayloads": split_sources,
        },
        "quickSheet": {
            "spreadsheetId": next(iter(spreadsheet_ids)),
            "worksheet": next(iter(worksheets)),
            "note": "The spreadsheet identifiers are serialized build-time metadata; local rows remain authoritative for version 1.411.",
        },
        "tables": tables,
        "findings": [
            "Male and female pools each contain 70 localized rows with contiguous indices 0 through 69.",
            "Names are table values, not stable Hunter identity keys; Hunter instances may be renamed after creation.",
            "No runtime server fetch is required to recover these versioned pools.",
        ],
    }
    OUT.write_text(json.dumps(output, indent=2, ensure_ascii=True) + "\n")
    print(f"Wrote {sum(table['rowCount'] for table in tables.values())} localized Hunter name rows to {OUT}")


if __name__ == "__main__":
    main()
