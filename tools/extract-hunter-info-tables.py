#!/usr/bin/env python3
"""Decode packaged tables used by the Hunter information tabs."""

from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path
from typing import Callable

import UnityPy


ROOT = Path(__file__).resolve().parents[1]
ASSET = ROOT / "game-assets/extracted/joined_unity_files/sharedassets1.assets"
OUT = ROOT / "reverse-engineering/evidence/hunter-info-tables-v1.json"
LOCALES = ("ko", "ja", "en", "zh-TW", "zh-CN", "ru", "fr", "es", "pt", "it", "de", "th", "vi", "id")


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
            raise ValueError(f"invalid int array count {count} at {self.offset - 4}")
        return [self.int32() for _ in range(count)]

    def float32_array(self) -> list[float]:
        count = self.int32()
        if count < 0 or count > (len(self.data) - self.offset) // 4:
            raise ValueError(f"invalid float array count {count} at {self.offset - 4}")
        return [self.float32() for _ in range(count)]


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_sha256(value: object) -> str:
    payload = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    return sha256_bytes(payload)


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


def localized(reader: Reader, first: dict[str, str], fields: tuple[str, ...]) -> dict[str, dict[str, str]]:
    values = {"ko": first}
    for locale in LOCALES[1:]:
        values[locale] = {field: reader.string() for field in fields}
    return values


def decode_skills(data: bytes) -> tuple[dict[str, object], list[dict[str, object]]]:
    table, reader = header(data)
    rows = []
    for _ in range(int(table["rowCount"])):
        index = reader.int32()
        korean = {"name": reader.string()}
        row = {"index": index, "job": reader.int32()}
        korean["description"] = reader.string()
        korean["detailDescription"] = reader.string()
        row.update({
            "maxLevel": reader.int32(),
            "coolTime": reader.float32(),
            "keepTimeByLevel": reader.float32_array(),
            "keepValueByLevel": reader.int32_array(),
            "valueTimeByLevel": reader.float32_array(),
            "valueCountByLevel": reader.int32_array(),
            "studyLevel": reader.int32_array(),
            "studyMoney": reader.int32_array(),
            "localized": localized(reader, korean, ("name", "description", "detailDescription")),
        })
        rows.append(row)
    finish(data, reader, str(table["worksheetName"]))
    return table, rows


def decode_sub_job_skills(data: bytes) -> tuple[dict[str, object], list[dict[str, object]]]:
    table, reader = header(data)
    rows = []
    for _ in range(int(table["rowCount"])):
        index = reader.int32()
        korean = {"name": reader.string()}
        row = {
            "index": index,
            "job": reader.int32(),
            "subJob": reader.int32(),
            "thirdJob": reader.int32(),
            "fourthJob": reader.int32(),
        }
        korean["description"] = reader.string()
        korean["detailDescription"] = reader.string()
        row.update({
            "maxLevel": reader.int32(),
            "coolTime": reader.float32(),
            "upCoolTime": reader.float32(),
            "keepTime": reader.float32(),
            "upKeepTime": reader.float32(),
            "keepValue": reader.float32(),
            "upKeepValue": reader.float32(),
            "secondValue": reader.float32(),
            "upSecondValue": reader.float32(),
            "valueTime": reader.float32(),
            "upValueTime": reader.float32(),
            "valueCount": reader.int32(),
            "upValueCount": reader.int32(),
            "firstStudySoul": reader.int32(),
            "addStudySoul": reader.int32(),
            "localized": localized(reader, korean, ("name", "description", "detailDescription")),
        })
        rows.append(row)
    finish(data, reader, str(table["worksheetName"]))
    return table, rows


def decode_growth_properties(data: bytes) -> tuple[dict[str, object], list[dict[str, object]]]:
    table, reader = header(data)
    rows = []
    for _ in range(int(table["rowCount"])):
        index = reader.int32()
        korean = {"name": reader.string(), "description": reader.string()}
        rows.append({
            "index": index,
            "upValue": reader.float32(),
            "localized": localized(reader, korean, ("name", "description")),
        })
    finish(data, reader, str(table["worksheetName"]))
    return table, rows


def decode_experience(data: bytes) -> tuple[dict[str, object], list[dict[str, object]]]:
    table, reader = header(data)
    rows = []
    for _ in range(int(table["rowCount"])):
        rows.append({
            "index": reader.int32(),
            "experienceByDifficulty": [reader.int32() for _ in range(6)],
            "unresolvedDummyFields": [reader.string() for _ in range(6)],
        })
    finish(data, reader, str(table["worksheetName"]))
    return table, rows


def decode_job_traits(data: bytes) -> tuple[dict[str, object], list[dict[str, object]]]:
    table, reader = header(data)
    rows = []
    for _ in range(int(table["rowCount"])):
        index = reader.int32()
        row = {
            "index": index,
            "jobCategory": reader.int32(),
            "job": reader.int32(),
            "subJob": reader.int32(),
            "thirdJob": reader.int32(),
            "fourthJob": reader.int32(),
            "skillTreeStep": reader.int32(),
            "skillTreeOrder": reader.int32(),
            "duplicateIndices": reader.int32_array(),
            "firstStudyMaterial": reader.int32(),
        }
        korean = {
            "title": reader.string(),
            "description": reader.string(),
            "detailDescription": reader.string(),
        }
        row.update({
            "maxLevel": reader.int32(),
            "firstValue": reader.float32(),
            "upFirstValue": reader.float32(),
            "secondValue": reader.float32(),
            "upSecondValue": reader.float32(),
            "thirdValue": reader.float32(),
            "upThirdValue": reader.float32(),
            "localized": localized(reader, korean, ("title", "description", "detailDescription")),
        })
        rows.append(row)
    finish(data, reader, str(table["worksheetName"]))
    return table, rows


def decode_riding_pets(data: bytes) -> tuple[dict[str, object], list[dict[str, object]]]:
    table, reader = header(data)
    rows = []
    for _ in range(int(table["rowCount"])):
        index = reader.int32()
        korean = {"title": reader.string(), "content": reader.string()}
        row = {
            "index": index,
            "grade": reader.int32(),
            "backgroundType": reader.int32(),
            "localized": {"ko": korean},
        }
        for locale in LOCALES[1:]:
            row["localized"][locale] = {"title": reader.string()}
        rows.append(row)
    finish(data, reader, str(table["worksheetName"]))
    return table, rows


def decode_riding_pet_skills(data: bytes) -> tuple[dict[str, object], list[dict[str, object]]]:
    table, reader = header(data)
    rows = []
    for _ in range(int(table["rowCount"])):
        index = reader.int32()
        korean = {
            "title": reader.string(),
            "description": reader.string(),
            "detailDescription": reader.string(),
        }
        rows.append({
            "index": index,
            "firstValue": reader.int32_array(),
            "upFirstValue": reader.int32_array(),
            "secondValue": reader.int32_array(),
            "upSecondValue": reader.int32_array(),
            "thirdValue": reader.int32_array(),
            "upThirdValue": reader.int32_array(),
            "localized": localized(reader, korean, ("title", "description", "detailDescription")),
        })
    finish(data, reader, str(table["worksheetName"]))
    return table, rows


def decode_riding_pet_traits(data: bytes) -> tuple[dict[str, object], list[dict[str, object]]]:
    table, reader = header(data)
    rows = []
    for _ in range(int(table["rowCount"])):
        index = reader.int32()
        korean = {"title": reader.string()}
        max_level = reader.int32()
        korean["description"] = reader.string()
        korean["detailDescription"] = reader.string()
        rows.append({
            "index": index,
            "maxLevel": max_level,
            "firstValue": reader.float32(),
            "upFirstValue": reader.float32(),
            "localized": localized(reader, korean, ("title", "description", "detailDescription")),
        })
    finish(data, reader, str(table["worksheetName"]))
    return table, rows


TABLES: dict[str, tuple[int, Callable[[bytes], tuple[dict[str, object], list[dict[str, object]]]]]] = {
    "skills": (12636, decode_skills),
    "subJobSkills": (12637, decode_sub_job_skills),
    "growthProperties": (12594, decode_growth_properties),
    "experience": (12579, decode_experience),
    "jobTraits": (12602, decode_job_traits),
    "ridingPets": (12627, decode_riding_pets),
    "ridingPetSkills": (12625, decode_riding_pet_skills),
    "ridingPetTraits": (12626, decode_riding_pet_traits),
}


def main() -> None:
    environment = UnityPy.load(str(ASSET))
    objects = {obj.path_id: obj for obj in environment.objects}
    catalog = []
    decoded = {}
    for key, (path_id, decoder) in TABLES.items():
        raw = objects[path_id].get_raw_data()
        table, rows = decoder(raw)
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
    result = {
        "schemaVersion": 1,
        "contractType": "hunter-info-table-evidence",
        "runtimeCompatibility": "evidence-only",
        "source": {
            "asset": ASSET.relative_to(ROOT).as_posix(),
            "assetSha256": sha256_bytes(ASSET.read_bytes()),
            "unityVersion": "6000.3.9f1",
        },
        "policy": "Worksheet headers recover field order/types only; every emitted value is decoded from the packaged Unity object and each decoder consumes it exactly.",
        "catalog": catalog,
        **decoded,
        "limitations": [
            "These definitions do not reveal which skills or riding pets a particular Hunter owns.",
            "Asset filenames are not bound to rows unless a serialized/runtime reference proves the mapping.",
            "Definitions do not reveal the per-Hunter Secret Point allocation, learned Job Trait ranks, or current experience threshold selection.",
        ],
    }
    OUT.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(f"Wrote {sum(len(rows) for rows in decoded.values())} exact rows to {OUT}")


if __name__ == "__main__":
    main()
