#!/usr/bin/env python3
"""Decode source-embedded Hunter definition/name/Characteristic tables exactly."""

from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path

import UnityPy


ROOT = Path(__file__).resolve().parents[1]
ASSET = ROOT / "game-assets/extracted/joined_unity_files/sharedassets1.assets"
OUT = ROOT / "reverse-engineering/evidence/hunter-generation-tables-v1.json"

NAME_LOCALES = ("en", "ko", "ja", "zh-TW", "zh-CN", "ru", "fr", "es", "pt", "it", "de", "th", "vi", "id")
CONTENT_LOCALES = ("ko", "ja", "en", "zh-TW", "zh-CN", "ru", "fr", "es", "pt", "it", "de", "th", "vi", "id")


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

    def float32(self) -> float:
        value = struct.unpack_from("<f", self.data, self.offset)[0]
        self.offset += 4
        return value

    def string(self) -> str:
        length = self.int32()
        if length < 0 or length > len(self.data) - self.offset:
            raise ValueError(f"invalid string length {length} at {self.offset - 4}")
        end = self.offset + length
        value = self.data[self.offset:end].decode("utf-8")
        self.offset = align4(end)
        return value

    def int32_array(self) -> list[int]:
        count = self.int32()
        if count < 0 or count > (len(self.data) - self.offset) // 4:
            raise ValueError(f"invalid int32 array length {count} at {self.offset - 4}")
        return [self.int32() for _ in range(count)]

    def string_array(self) -> list[str]:
        count = self.int32()
        if count < 0 or count > (len(self.data) - self.offset) // 4:
            raise ValueError(f"invalid string array length {count} at {self.offset - 4}")
        return [self.string() for _ in range(count)]


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_sha256(value: object) -> str:
    encoded = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    return sha256_bytes(encoded)


def header(data: bytes) -> tuple[dict[str, object], Reader]:
    script_path_id = struct.unpack_from("<q", data, 20)[0]
    reader = Reader(data, 28)
    result = {
        "name": reader.string(),
        "monoScriptPathId": script_path_id,
        "spreadsheetId": reader.string(),
        "spreadsheetName": reader.string(),
        "worksheetName": reader.string(),
        "rowCount": reader.int32(),
    }
    return result, reader


def finish(data: bytes, reader: Reader, table: str) -> None:
    if reader.offset != len(data):
        raise ValueError(f"{table} decoder left {len(data) - reader.offset} trailing bytes")


def decode_names(data: bytes) -> tuple[dict[str, object], list[dict[str, object]]]:
    table, reader = header(data)
    rows = []
    for _ in range(int(table["rowCount"])):
        row = {"index": reader.int32()}
        row["localizedNames"] = {locale: reader.string() for locale in NAME_LOCALES}
        rows.append(row)
    finish(data, reader, str(table["worksheetName"]))
    return table, rows


def decode_hunter_definitions(data: bytes) -> tuple[dict[str, object], list[dict[str, object]]]:
    table, reader = header(data)
    rows = []
    stat_fields = (
        "hp", "hpSecond", "hpThird", "damage", "damageSecond", "damageThird",
        "armor", "armorSecond", "armorThird", "dodge", "dodgeSecond",
        "dodgeThird", "critical", "criticalSecond", "criticalThird",
    )
    for _ in range(int(table["rowCount"])):
        row: dict[str, object] = {
            "index": reader.int32(),
            "localizedJobNames": {"ko": reader.string_array()},
        }
        for field in stat_fields:
            row[field] = reader.int32_array()
        row["attackSpeed"] = reader.float32()
        row["revivePercent"] = reader.int32_array()
        row["hpPercent"] = reader.int32()
        row["armorPercent"] = reader.int32()
        row["damagePercent"] = reader.int32()
        for locale in CONTENT_LOCALES[1:]:
            row["localizedJobNames"][locale] = reader.string_array()
        rows.append(row)
    finish(data, reader, str(table["worksheetName"]))
    return table, rows


def decode_characteristics(data: bytes) -> tuple[dict[str, object], list[dict[str, object]]]:
    table, reader = header(data)
    rows = []
    for _ in range(int(table["rowCount"])):
        row: dict[str, object] = {
            "index": reader.int32(),
            "localized": {"ko": {"name": reader.string(), "description": reader.string()}},
            "keepValue": reader.int32(),
        }
        for locale in CONTENT_LOCALES[1:]:
            row["localized"][locale] = {"name": reader.string()}
        rows.append(row)
    finish(data, reader, str(table["worksheetName"]))
    return table, rows


def main() -> None:
    environment = UnityPy.load(str(ASSET))
    objects = {obj.path_id: obj for obj in environment.objects}
    specs = {
        "maleNames": (12596, decode_names),
        "femaleNames": (12597, decode_names),
        "hunterDefinitions": (12599, decode_hunter_definitions),
        "characteristics": (12613, decode_characteristics),
    }
    catalog = []
    decoded = {}
    for key, (path_id, decoder) in specs.items():
        obj = objects[path_id]
        if obj.type.name != "MonoBehaviour":
            raise ValueError(f"path {path_id} is {obj.type.name}, expected MonoBehaviour")
        raw = obj.get_raw_data()
        table, rows = decoder(raw)
        if len(rows) != table["rowCount"]:
            raise ValueError(f"{key} decoded {len(rows)} of {table['rowCount']} rows")
        decoded[key] = rows
        catalog.append({
            "key": key,
            "pathId": path_id,
            "byteSize": len(raw),
            "rawSha256": sha256_bytes(raw),
            "decodedSha256": canonical_sha256(rows),
            "decodeStatus": "decoded-exact",
            **table,
        })

    output = {
        "schemaVersion": 1,
        "contractType": "hunter-generation-table-evidence",
        "runtimeCompatibility": "evidence-only",
        "source": {
            "asset": ASSET.relative_to(ROOT).as_posix(),
            "assetSha256": sha256_bytes(ASSET.read_bytes()),
            "unityVersion": "6000.3.9f1",
        },
        "policy": "Public QuickSheet headers identify serialized field order/types only; all emitted row values are decoded from the packaged Unity asset and each decoder consumes its object exactly.",
        "catalog": catalog,
        **decoded,
        "limitations": [
            "The tables define pools and stat ranges; they do not reveal the RNG distribution or creation call order.",
            "The Characteristic table is named personality in the package; the exact assignment method remains in protected native code.",
            "No table here binds body, costume, hat, weapon, rarity, or generated name into one Hunter instance.",
        ],
    }
    OUT.write_text(json.dumps(output, ensure_ascii=False, indent=2) + "\n")
    print(f"Wrote exact Hunter generation tables to {OUT}")


if __name__ == "__main__":
    main()
