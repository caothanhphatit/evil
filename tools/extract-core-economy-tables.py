#!/usr/bin/env python3
"""Decode source-confirmed core economy tables from sharedassets1.assets.

The public QuickSheet worksheet headers are used only to recover serialized
field order and types. Every emitted gameplay value is read from the embedded
Unity object bytes, and every decoder must consume its object exactly.
"""

from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path
from typing import Callable

import UnityPy


ROOT = Path(__file__).resolve().parents[1]
ASSET = ROOT / "game-assets/extracted/joined_unity_files/sharedassets1.assets"
OUT = ROOT / "reverse-engineering/evidence/core-economy-tables-v1.json"

LOCALES = ("ko", "ja", "en", "zh-TW", "zh-CN", "ru", "fr", "es", "pt", "it", "de", "th", "vi", "id")

TABLES: dict[int, tuple[str, Callable[[bytes], list[dict]]]] = {}


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

    def int64(self) -> int:
        value = struct.unpack_from("<q", self.data, self.offset)[0]
        self.offset += 8
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

    def float32_array(self) -> list[float]:
        count = self.int32()
        if count < 0 or count > (len(self.data) - self.offset) // 4:
            raise ValueError(f"invalid float32 array length {count} at {self.offset - 4}")
        return [self.float32() for _ in range(count)]


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_sha256(value: object) -> str:
    encoded = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    return sha256_bytes(encoded)


def mono_behaviour_header(data: bytes) -> tuple[dict, Reader]:
    script_path_id = struct.unpack_from("<q", data, 20)[0]
    reader = Reader(data, 28)
    name = reader.string()
    spreadsheet_id = reader.string()
    spreadsheet_name = reader.string()
    worksheet_name = reader.string()
    row_count = reader.int32()
    return {
        "name": name,
        "monoScriptPathId": script_path_id,
        "spreadsheetId": spreadsheet_id,
        "spreadsheetName": spreadsheet_name,
        "worksheetName": worksheet_name,
        "rowCount": row_count,
    }, reader


def finish(data: bytes, reader: Reader, table: str) -> None:
    if reader.offset != len(data):
        raise ValueError(f"{table} decoder left {len(data) - reader.offset} trailing bytes")


def decode_materials(data: bytes) -> list[dict]:
    header, reader = mono_behaviour_header(data)
    rows = []
    for _ in range(header["rowCount"]):
        index = reader.int32()
        names = {"ko": reader.string()}
        row = {
            "index": index,
            "price": reader.int32(),
            "convert": reader.int32(),
            "compose": reader.int32(),
            "parentIndex": reader.int32(),
            "rating": reader.int32(),
            "level": reader.int32(),
            "magic": reader.int32(),
            "unresolvedDummyFields": [reader.string(), reader.string(), reader.string()],
            "localizedNames": names,
        }
        for locale in LOCALES[1:]:
            names[locale] = reader.string()
        rows.append(row)
    finish(data, reader, "met")
    return rows


def decode_gear(data: bytes, *, weapon: bool) -> list[dict]:
    header, reader = mono_behaviour_header(data)
    rows = []
    for _ in range(header["rowCount"]):
        index = reader.int32()
        localized = {"ko": {"title": reader.string()}}
        row = {
            "index": index,
            "job": reader.int32(),
            "uniqueType": reader.int32(),
            "group": reader.int32(),
            "itemLevel": reader.int32(),
            "growLevel": reader.int32(),
            "growTarget": reader.int32(),
            "growMaterialId": reader.int32(),
            "growCount": reader.int32(),
            "growPassible": reader.int32(),
        }
        localized["ko"]["description"] = reader.string()
        row.update({
            "ratingValues": reader.int32_array(),
            "allValues": reader.int32_array(),
            "firstValue": reader.float32() if weapon else reader.int32(),
            "firstPercent": reader.int32_array(),
            "secondValue": reader.int32(),
            "secondPercent": reader.int32_array(),
        })
        modifiers = []
        for _slot in range(3):
            modifiers.append({
                "plusType": reader.int32(),
                "plusValues": reader.int32_array(),
                "minusType": reader.int32(),
                "minusValues": reader.int32_array(),
            })
        row["modifiers"] = modifiers
        row.update({
            "additionalPlusTypes": reader.int32_array(),
            "additionalPlusValues": reader.int32_array(),
            "additionalMinusTypes": reader.int32_array(),
            "additionalMinusValues": reader.int32_array(),
            "additionalValueRange": reader.int32_array(),
        })
        materials = []
        for _rating in range(5):
            materials.append({"ids": reader.int32_array(), "quantities": reader.int32_array()})
        row["craftingMaterialsByRating"] = materials
        row["buyMoneyByRating"] = reader.int32_array()
        row["visible"] = reader.int32()
        if weapon:
            row["sortGroup"] = reader.int32()
        for locale in LOCALES[1:]:
            localized[locale] = {"title": reader.string(), "description": reader.string()}
        row["localized"] = localized
        rows.append(row)
    finish(data, reader, header["worksheetName"])
    return rows


def decode_consumables(data: bytes) -> list[dict]:
    header, reader = mono_behaviour_header(data)
    rows = []
    for _ in range(header["rowCount"]):
        index = reader.int32()
        localized = {"ko": {
            "title": reader.string(),
            "description": reader.string(),
            "detailDescription": reader.string(),
        }}
        row = {
            "index": index,
            "type": reader.int32(),
            "maxLevel": reader.int32(),
            "keepTimeByLevel": reader.float32_array(),
            "keepValueByLevel": reader.int32_array(),
            "coolTime": reader.float32(),
            "priceByLevel": reader.int32_array(),
        }
        materials = []
        for _level in range(8):
            materials.append({"ids": reader.int32_array(), "quantities": reader.int32_array()})
        row["craftingMaterialsByLevel"] = materials
        for locale in LOCALES[1:]:
            localized[locale] = {
                "title": reader.string(),
                "description": reader.string(),
                "detailDescription": reader.string(),
            }
        row["localized"] = localized
        rows.append(row)
    finish(data, reader, "consum")
    return rows


def decode_rune_craft(data: bytes) -> list[dict]:
    header, reader = mono_behaviour_header(data)
    rows = []
    for _ in range(header["rowCount"]):
        index = reader.int32()
        localized = {"ko": {"title": reader.string(), "contents": reader.string()}}
        row = {"index": index, "grade": reader.int32(), "price": reader.int64()}
        for locale in LOCALES[1:]:
            localized[locale] = {"title": reader.string(), "contents": reader.string()}
        row["localized"] = localized
        rows.append(row)
    finish(data, reader, "runeCraft")
    return rows


def decode_runes(data: bytes) -> list[dict]:
    header, reader = mono_behaviour_header(data)
    rows = []
    for _ in range(header["rowCount"]):
        index = reader.int32()
        localized = {"ko": {
            "title": reader.string(),
            "description": reader.string(),
        }}
        row = {
            "index": index,
            "runeType": reader.int32(),
            "useJob": reader.int32(),
            "propertyIndex": reader.int32(),
        }
        localized["ko"]["plusUnit"] = reader.string()
        row.update({
            "percentByGrade": reader.int32_array(),
            "minimumRange": reader.int32_array(),
            "maximumRange": reader.int32_array(),
        })
        for locale in LOCALES[1:]:
            localized[locale] = {
                "title": reader.string(),
                "description": reader.string(),
                "plusUnit": reader.string(),
            }
        row["localized"] = localized
        rows.append(row)
    finish(data, reader, "runes")
    return rows


TABLES.update({
    12566: ("consumables", decode_consumables),
    12583: ("gearArmor", lambda data: decode_gear(data, weapon=False)),
    12584: ("gearBelt", lambda data: decode_gear(data, weapon=False)),
    12585: ("gearBoots", lambda data: decode_gear(data, weapon=False)),
    12586: ("gearGloves", lambda data: decode_gear(data, weapon=False)),
    12587: ("gearHelmet", lambda data: decode_gear(data, weapon=False)),
    12588: ("gearNecklace", lambda data: decode_gear(data, weapon=False)),
    12590: ("gearRing", lambda data: decode_gear(data, weapon=False)),
    12593: ("gearWeapons", lambda data: decode_gear(data, weapon=True)),
    12606: ("materials", decode_materials),
    12631: ("runeCraft", decode_rune_craft),
    12632: ("runes", decode_runes),
})


def main() -> None:
    environment = UnityPy.load(str(ASSET))
    objects = {obj.path_id: obj for obj in environment.objects}
    catalog = []
    decoded: dict[str, list[dict]] = {}
    for path_id, (key, decoder) in TABLES.items():
        obj = objects[path_id]
        if obj.type.name != "MonoBehaviour":
            raise ValueError(f"path {path_id} is {obj.type.name}, expected MonoBehaviour")
        raw = obj.get_raw_data()
        header, _ = mono_behaviour_header(raw)
        rows = decoder(raw)
        if len(rows) != header["rowCount"]:
            raise ValueError(f"{key} decoded {len(rows)} of {header['rowCount']} rows")
        decoded[key] = rows
        catalog.append({
            "pathId": path_id,
            "key": key,
            "byteSize": len(raw),
            "rawSha256": sha256_bytes(raw),
            "decodedSha256": canonical_sha256(rows),
            "decodeStatus": "decoded-exact",
            **header,
        })

    output = {
        "schemaVersion": 1,
        "source": {
            "asset": ASSET.relative_to(ROOT).as_posix(),
            "assetSha256": sha256_bytes(ASSET.read_bytes()),
            "unityVersion": "6000.3.9f1",
        },
        "policy": "Worksheet headers recover schema only; all emitted values come from embedded bytes and every decoder consumes its object exactly.",
        "schemaEvidence": {
            "publicWorksheet": "https://docs.google.com/spreadsheets/d/1_i2_M1-enmBOAqqqRKPCBMGnQid6uxlkGvxd-5YsA6U",
            "usage": "field names/order/type hints only; public cell values are not copied into this artifact",
            "unknowns": "Source dummy fields and the misspelled growPassible field are preserved without invented semantics.",
        },
        "catalog": catalog,
        **decoded,
    }
    OUT.write_text(json.dumps(output, ensure_ascii=False, indent=2) + "\n")
    print(f"Wrote {sum(len(rows) for rows in decoded.values())} exact economy rows across {len(decoded)} tables to {OUT}")


if __name__ == "__main__":
    main()
