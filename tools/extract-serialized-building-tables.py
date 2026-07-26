#!/usr/bin/env python3
"""Extract deterministic building-related QuickSheet snapshots from Unity assets.

Only tables whose serialized field layout is source-confirmed are decoded. Other
tables remain catalogued with their object identity and raw hash so later schema
recovery cannot silently turn guesses into content.
"""

from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path

import UnityPy


ROOT = Path(__file__).resolve().parents[1]
ASSET = ROOT / "game-assets/extracted/joined_unity_files/sharedassets1.assets"
OUT = ROOT / "reverse-engineering/evidence/serialized-building-tables-v1.json"

LOCALES = ("ko", "ja", "en", "zh-TW", "zh-CN", "ru", "fr", "es", "pt", "it", "de", "th", "vi", "id")

# QuickSheet ScriptableObjects are contiguous in this source asset. These are
# building/economy-relevant members of that block, not inferred runtime IDs.
FOCUS_TABLES = {
    12551: "trader-information",
    12557: "building-skins",
    12558: "buildings",
    12583: "gear-armor",
    12584: "gear-belt",
    12585: "gear-boots",
    12586: "gear-gloves",
    12587: "gear-helmet",
    12588: "gear-necklace",
    12589: "gear-properties",
    12590: "gear-ring",
    12591: "gear-set-properties",
    12592: "gear-skills",
    12593: "gear-weapons",
    12615: "products",
    12638: "trade-wagon",
}


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
        if length < 0:
            raise ValueError(f"negative string length at {self.offset - 4}")
        end = self.offset + length
        value = self.data[self.offset:end].decode("utf-8")
        self.offset = align4(end)
        return value

    def int32_array(self) -> list[int]:
        count = self.int32()
        if count < 0:
            raise ValueError(f"negative array length at {self.offset - 4}")
        return [self.int32() for _ in range(count)]

    def int64_array(self) -> list[int]:
        count = self.int32()
        if count < 0:
            raise ValueError(f"negative array length at {self.offset - 4}")
        return [self.int64() for _ in range(count)]

    def string_array(self) -> list[str]:
        count = self.int32()
        if count < 0:
            raise ValueError(f"negative array length at {self.offset - 4}")
        return [self.string() for _ in range(count)]


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def mono_behaviour_header(data: bytes) -> tuple[dict, Reader]:
    # Unity 6000 MonoBehaviour header: GameObject PPtr, enabled/alignment,
    # MonoScript PPtr, then the ScriptableObject name.
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


def decode_building_skins(data: bytes) -> list[dict]:
    header, reader = mono_behaviour_header(data)
    rows = []
    for _ in range(header["rowCount"]):
        index = reader.int32()
        titles = {"ko": reader.string()}
        building_id = reader.int32()
        skin_id = reader.int32()
        required_gold = reader.int64()
        material_ids = reader.int32_array()
        material_quantities = reader.int64_array()
        visibility = reader.int32()
        required_level = reader.int32()
        for locale in LOCALES[1:]:
            titles[locale] = reader.string()
        rows.append({
            "index": index,
            "buildingId": building_id,
            "skinId": skin_id,
            "requiredGold": required_gold,
            "requiredMaterialIds": material_ids,
            "requiredMaterialQuantities": material_quantities,
            "visibility": visibility,
            "requiredLevel": required_level,
            "titles": titles,
        })
    if reader.offset != len(data):
        raise ValueError(f"buildSkin trailing bytes: {len(data) - reader.offset}")
    return rows


def decode_buildings(data: bytes) -> list[dict]:
    header, reader = mono_behaviour_header(data)
    rows = []
    for _ in range(header["rowCount"]):
        index = reader.int32()
        localized = {
            "ko": {
                "title": reader.string(),
                "description": reader.string(),
                "subDescription": reader.string(),
                "descriptionText": reader.string_array(),
                "descriptionTextSecond": reader.string_array(),
            }
        }
        description_text_third = reader.string_array()
        building_type = reader.int32()
        max_build = reader.int32()
        max_level = reader.int32()
        compatible_skin = reader.int32()
        possible_build = reader.int32_array()
        possible_remove = reader.int32()
        create_build = reader.int32_array()
        size = reader.int32_array()
        required_gold = reader.int32_array()
        material_1_ids = reader.int32_array()
        material_1_quantities = reader.int32_array()
        material_2_ids = reader.int32_array()
        material_2_quantities = reader.int32_array()
        material_3_ids = reader.int32_array()
        material_3_quantities = reader.int32_array()
        material_4_ids = reader.int32_array()
        material_4_quantities = reader.int32_array()
        entry_counts = reader.int32_array()
        first_values = reader.int32_array()
        second_values = reader.int32_array()
        third_values = reader.int32_array()
        in_building_flag = reader.int32()
        visibility = reader.int32()
        movable = reader.int32()
        for locale in LOCALES[1:]:
            localized[locale] = {
                "title": reader.string(),
                "description": reader.string(),
                "subDescription": reader.string(),
                "descriptionText": reader.string_array(),
                "descriptionTextSecond": reader.string_array(),
            }
        dummy_text = reader.string_array()
        rows.append({
            "index": index,
            "type": building_type,
            "maxBuild": max_build,
            "maxLevel": max_level,
            "compatibleSkin": compatible_skin,
            "possibleBuild": possible_build,
            "possibleRemove": possible_remove,
            "createBuild": create_build,
            "size": size,
            "requiredGold": required_gold,
            "requiredMaterials": [
                {"ids": material_1_ids, "quantities": material_1_quantities},
                {"ids": material_2_ids, "quantities": material_2_quantities},
                {"ids": material_3_ids, "quantities": material_3_quantities},
                {"ids": material_4_ids, "quantities": material_4_quantities},
            ],
            "entryCounts": entry_counts,
            "firstValues": first_values,
            "secondValues": second_values,
            "thirdValues": third_values,
            "inBuildingFlag": in_building_flag,
            "visibility": visibility,
            "movable": movable,
            "descriptionTextThird": description_text_third,
            "dummyText": dummy_text,
            "localized": localized,
        })
    if reader.offset != len(data):
        raise ValueError(f"build trailing bytes: {len(data) - reader.offset}")
    return rows


def decode_trade_wagon(data: bytes) -> list[dict]:
    header, reader = mono_behaviour_header(data)
    rows = []
    for _ in range(header["rowCount"]):
        index = reader.int32()
        localized = {"ko": {"title": reader.string(), "description": reader.string()}}
        required_level = reader.int32()
        for locale in LOCALES[1:]:
            localized[locale] = {"title": reader.string(), "description": reader.string()}
        rows.append({
            "index": index,
            "requiredLevel": required_level,
            "localized": localized,
        })
    if reader.offset != len(data):
        raise ValueError(f"tradeWagon trailing bytes: {len(data) - reader.offset}")
    return rows


def decode_products(data: bytes) -> list[dict]:
    header, reader = mono_behaviour_header(data)
    rows = []
    for _ in range(header["rowCount"]):
        index = reader.int32()
        title_ko = reader.string()
        product_type = reader.int32()
        level = reader.int32()
        description_ko = reader.string()
        building_id = reader.int32()
        building_name_ko = reader.string()
        duration = reader.float32()
        first_value = reader.int32()
        price = reader.int32()
        material_ids = reader.int32_array()
        material_quantities = reader.int32_array()
        completion_counts = reader.int32_array()
        required_cash = reader.int32()
        cash_completion_count = reader.int32()
        required_elemental = reader.int32()
        elemental_completion_count = reader.int32()
        localized = {"ko": {"title": title_ko, "description": description_ko}}
        for locale in LOCALES[1:]:
            localized[locale] = {"title": reader.string(), "description": reader.string()}
        rows.append({
            "index": index,
            "type": product_type,
            "level": level,
            "buildingId": building_id,
            "buildingNameKo": building_name_ko,
            "duration": duration,
            "timeSeconds": duration,
            "firstValue": first_value,
            "price": price,
            "useMoney": price,
            "requiredMaterialIds": material_ids,
            "requiredMaterialQuantities": material_quantities,
            "completionCounts": completion_counts,
            "requiredCash": required_cash,
            "cashCompletionCount": cash_completion_count,
            "requiredElemental": required_elemental,
            "elementalCompletionCount": elemental_completion_count,
            "localized": localized,
        })
    if reader.offset != len(data):
        raise ValueError(f"product trailing bytes: {len(data) - reader.offset}")
    return rows


def main() -> None:
    environment = UnityPy.load(str(ASSET))
    objects = {obj.path_id: obj for obj in environment.objects}
    catalog = []
    raw_by_path: dict[int, bytes] = {}
    for path_id, role in FOCUS_TABLES.items():
        obj = objects[path_id]
        if obj.type.name != "MonoBehaviour":
            raise ValueError(f"path {path_id} is {obj.type.name}, expected MonoBehaviour")
        raw = obj.get_raw_data()
        raw_by_path[path_id] = raw
        header, _ = mono_behaviour_header(raw)
        catalog.append({
            "pathId": path_id,
            "role": role,
            "byteSize": len(raw),
            "rawSha256": sha256_bytes(raw),
            **header,
            "decodeStatus": "decoded" if path_id in (12557, 12558, 12615, 12638) else "schema-unresolved",
        })

    output = {
        "schemaVersion": 1,
        "source": {
            "asset": ASSET.relative_to(ROOT).as_posix(),
            "assetSha256": hashlib.sha256(ASSET.read_bytes()).hexdigest(),
            "unityVersion": "6000.3.9f1",
        },
        "policy": "Unknown fields and tables remain explicitly unresolved; no inferred gameplay values.",
        "catalog": catalog,
        "buildings": decode_buildings(raw_by_path[12558]),
        "buildingSkins": decode_building_skins(raw_by_path[12557]),
        "products": decode_products(raw_by_path[12615]),
        "tradeWagon": decode_trade_wagon(raw_by_path[12638]),
        "schemaEvidence": {
            "buildingSkinTypeIndex": 1753,
            "buildingTypeIndex": 1757,
            "productTypeIndex": 1110,
            "tradeWagonTypeIndex": 748,
            "inspectorArtifact": "/tmp/building-types.json",
            "publicWorksheetSchema": "https://docs.google.com/spreadsheets/d/1_i2_M1-enmBOAqqqRKPCBMGnQid6uxlkGvxd-5YsA6U",
            "productWorksheetColumns": [
                "idx", "title", "type", "level", "desc", "buildIdx", "buildName",
                "time", "firstValue", "useMoney", "needMetIdx", "needMetCount",
                "completeCount", "needCashCount", "cashCompleteCount", "needEleCount",
                "eleCompleteCount",
            ],
            "productTimeUnitEvidence": "The public product worksheet names the field 'time'; all 14 localized descriptions render its placeholder as seconds/secs.",
            "note": "Type shapes came from recovered IL2CPP metadata. Surviving worksheet headers name fields; decoded values are always read from the embedded asset snapshot.",
        },
    }
    OUT.write_text(json.dumps(output, ensure_ascii=False, indent=2) + "\n")
    print(f"Wrote {len(catalog)} table records, {len(output['buildingSkins'])} skins, and {len(output['tradeWagon'])} trade rows to {OUT}")


if __name__ == "__main__":
    main()
