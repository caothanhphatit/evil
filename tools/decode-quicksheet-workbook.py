#!/usr/bin/env python3
"""Decode QuickSheet rows by proving workbook serialization against Unity bytes.

The public workbook supplies column order and source values; a row is emitted
only when the inferred primitive encodings reproduce the serialized object
payload exactly. This keeps unknown layouts fail-closed instead of guessing.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
import zipfile
import xml.etree.ElementTree as ET
from pathlib import Path

import UnityPy

ROOT = Path(__file__).resolve().parents[1]
ASSET = ROOT / "game-assets/extracted/joined_unity_files/sharedassets1.assets"
OUT = ROOT / "reverse-engineering/evidence/quicksheet-decoded-v1.json"
KNOWN_EXACT = {
    "met", "consum", "gearArmor", "gearBelt", "gearBoots", "gearGlove",
    "gearHelmet", "gearNeck", "gearRing", "gearWeapon", "runeCraft", "runes",
    "hunter", "hunterNameM", "hunterNameW", "personality", "evil", "dropUniqueGear",
    "exp", "skill", "subJobSkill", "growupProperty", "jobTrait", "ridingPet",
    "ridingPetSkill", "ridingPetTrait", "build", "buildSkin", "product", "tradeWagon",
}
NS = {"m": "http://schemas.openxmlformats.org/spreadsheetml/2006/main"}
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
NUMERIC = re.compile(r"^-?(?:\d+|\d+\.\d+)$")


def a4(n: int) -> int:
    return (n + 3) & ~3


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_string(data: bytes, offset: int) -> tuple[str, int]:
    length = struct.unpack_from("<i", data, offset)[0]
    offset += 4
    if length < 0 or length > 100_000 or offset + length > len(data):
        raise ValueError("invalid serialized string")
    end = offset + length
    return data[offset:end].decode("utf-8"), a4(end)


def quick_header(data: bytes) -> dict:
    script = struct.unpack_from("<q", data, 20)[0]
    offset = 28
    name, offset = read_string(data, offset)
    spreadsheet, offset = read_string(data, offset)
    workbook, offset = read_string(data, offset)
    sheet, offset = read_string(data, offset)
    count = struct.unpack_from("<i", data, offset)[0]
    return {"name": name, "spreadsheetId": spreadsheet, "workbook": workbook,
            "sheet": sheet, "rowCount": count, "scriptPathId": script,
            "dataOffset": offset + 4}


def workbook_rows(path: Path) -> dict[str, list[list[str]]]:
    with zipfile.ZipFile(path) as archive:
        shared: list[str] = []
        if "xl/sharedStrings.xml" in archive.namelist():
            root = ET.fromstring(archive.read("xl/sharedStrings.xml"))
            shared = ["".join(t.text or "" for t in si.iter(f"{{{NS['m']}}}t"))
                      for si in root.findall("m:si", NS)]
        book = ET.fromstring(archive.read("xl/workbook.xml"))
        rel_root = ET.fromstring(archive.read("xl/_rels/workbook.xml.rels"))
        rels = {x.attrib["Id"]: x.attrib["Target"] for x in rel_root}
        result = {}
        for sheet in book.find("m:sheets", NS):
            rel_id = sheet.attrib[f"{{{REL}}}id"]
            target = "xl/" + rels[rel_id]
            root = ET.fromstring(archive.read(target))
            rows = []
            for row in root.findall(".//m:sheetData/m:row", NS):
                values = {}
                for cell in row.findall("m:c", NS):
                    ref = cell.attrib.get("r", "A1")
                    col = 0
                    for char in re.match(r"[A-Z]+", ref).group(0):
                        col = col * 26 + ord(char) - 64
                    col -= 1
                    value = cell.find("m:v", NS)
                    text = "" if value is None else value.text or ""
                    if cell.attrib.get("t") == "s" and text:
                        text = shared[int(text)]
                    values[col] = text
                if values:
                    width = max(values) + 1
                    rows.append([values.get(i, "") for i in range(width)])
            result[sheet.attrib["name"]] = rows
        return result


def number(value: str) -> float | int:
    return float(value) if "." in value else int(value)


def candidates(values: list[str]) -> list[str]:
    nonempty = [v for v in values if v not in ("", "-")]
    if any("," in v for v in nonempty):
        parts = [p for v in nonempty for p in v.split(",") if p]
        if all(NUMERIC.fullmatch(p) for p in parts):
            # A zero/empty first row can hide whether later rolls are int or
            # float arrays; retain both candidates and prove against bytes.
            return ["arr-i32", "arr-i64", "arr-f32"]
    numeric_values = [v for v in nonempty if NUMERIC.fullmatch(v)]
    if nonempty and len(numeric_values) / len(nonempty) < 0.8:
        return ["str"]
    if not nonempty:
        return ["str"]
    result = ["i32", "i64"]
    if any("." in v for v in nonempty):
        result.append("f32")
    return result


def conservative_candidates(values: list[str]) -> list[str]:
    """Choose the narrowest package-safe type for ordinary admin tables."""
    nonempty = [v for v in values if v not in ("", "-")]
    if any("," in v for v in nonempty):
        parts = [p for v in nonempty for p in v.split(",") if p]
        if parts and all(NUMERIC.fullmatch(p) for p in parts):
            return ["arr-i32"]
    numeric = [v for v in nonempty if NUMERIC.fullmatch(v)]
    if nonempty and len(numeric) / len(nonempty) >= 0.8:
        if any(abs(int(float(v))) > 2_147_483_647 for v in numeric):
            return ["i64"]
        return ["i32"]
    return ["str"]


def encode(kind: str, value: str) -> bytes:
    if kind == "str":
        raw = value.encode("utf-8")
        return struct.pack("<i", len(raw)) + raw + b"\0" * ((-len(raw)) & 3)
    if kind == "i32":
        return struct.pack("<i", int(float(value or 0)))
    if kind == "i64":
        return struct.pack("<q", int(float(value or 0)))
    if kind == "f32":
        return struct.pack("<f", float(value or 0))
    if kind.startswith("arr-"):
        parts = [] if value in ("", "-") else value.split(",")
        subtype = kind[4:]
        payload = struct.pack("<i", len(parts))
        for part in parts:
            if subtype == "f32":
                payload += struct.pack("<f", float(part))
            elif subtype == "i64":
                payload += struct.pack("<q", int(float(part)))
            else:
                payload += struct.pack("<i", int(float(part)))
        return payload
    raise ValueError(kind)


def decode_sheet(raw: bytes, rows: list[list[str]], header: dict) -> tuple[list[str], list[dict]] | None:
    if len(rows) < 2 or len(rows) < header["rowCount"] + 1:
        return None
    rows = rows[: header["rowCount"] + 1]
    columns = max(len(r) for r in rows)
    names = rows[0] + [f"column_{i}" for i in range(len(rows[0]), columns)]
    values = [r + [""] * (columns - len(r)) for r in rows[1:]]
    kinds = [candidates([r[i] for r in values]) for i in range(columns)]
    # Trailing empty workbook columns are not serialized fields.
    while columns and all(r[columns - 1] == "" for r in values):
        columns -= 1
    kinds, names, values = kinds[:columns], names[:columns], [r[:columns] for r in values]
    payload = raw[header["dataOffset"]:]
    matches: list[list[str]] = []

    def parse_row(offset: int, row: list[str], index: int, selected: list[str]) -> list[tuple[int, list[str]]]:
        if index == columns:
            return [(offset, selected)]
        out = []
        choices = [selected[index]] if index < len(selected) else kinds[index]
        for kind in choices:
            try:
                blob = encode(kind, row[index])
            except (OverflowError, ValueError, struct.error):
                continue
            if payload[offset:offset + len(blob)] == blob:
                out.extend(parse_row(offset + len(blob), row, index + 1, selected + [kind]))
        return out

    first = parse_row(0, values[0], 0, [])
    for end, schema in first:
        offset = end
        decoded = [values[0]]
        ok = True
        for row in values[1:]:
            parsed = parse_row(offset, row, 0, schema)
            if not parsed:
                ok = False
                break
            offset, _ = parsed[0]
            decoded.append(row)
        if ok and offset == len(payload):
            return schema, [{names[i]: number(row[i]) if schema[i] in {"i32", "i64", "f32"} else ([number(x) for x in row[i].split(",")] if schema[i].startswith("arr-") and row[i] not in ("", "-") else ([] if schema[i].startswith("arr-") else row[i])) for i in range(columns)} for row in decoded]
    return None


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workbook", type=Path, required=True)
    parser.add_argument("--asset", type=Path, default=ASSET)
    parser.add_argument("--output", type=Path, default=OUT)
    args = parser.parse_args()
    rows_by_sheet = workbook_rows(args.workbook)
    env = UnityPy.load(str(args.asset))
    decoded, unresolved = {}, []
    for obj in env.objects:
        if obj.type.name != "MonoBehaviour":
            continue
        raw = obj.get_raw_data()
        try:
            header = quick_header(raw)
        except (ValueError, UnicodeDecodeError, struct.error):
            continue
        sheet = header["sheet"]
        if header["workbook"] != "evilhunterdata_global" or sheet not in rows_by_sheet:
            continue
        result = decode_sheet(raw, rows_by_sheet[sheet], header)
        record = {"pathId": obj.path_id, "worksheetName": sheet, "rowCount": header["rowCount"], "byteSize": len(raw), "rawSha256": sha(raw)}
        if result:
            schema, data = result
            record.update({"decodeStatus": "decoded-exact", "fieldKinds": schema, "rows": data})
            decoded[sheet] = record
        else:
            record["decodeStatus"] = "workbook-reference-unverified"
            # Preserve the source rows for analysis, but do not present them as
            # package-confirmed until a byte-exact serializer is recovered.
            record["fieldNames"] = rows_by_sheet[sheet][0]
            record["rows"] = rows_by_sheet[sheet][1 : header["rowCount"] + 1]
            unresolved.append(record)
    # Include the repository's older byte-exact decoders in the same manifest.
    # Their row schemas are richer than the generic primitive representation.
    legacy = {
        ROOT / "reverse-engineering/evidence/core-economy-tables-v1.json": {
            "consumables": "consum", "gearArmor": "gearArmor", "gearBelt": "gearBelt",
            "gearBoots": "gearBoots", "gearGloves": "gearGlove", "gearHelmet": "gearHelmet",
            "gearNecklace": "gearNeck", "gearRing": "gearRing", "gearWeapons": "gearWeapon",
            "materials": "met", "runeCraft": "runeCraft", "runes": "runes",
        },
        ROOT / "reverse-engineering/evidence/hunter-generation-tables-v1.json": {
            "maleNames": "hunterNameM", "femaleNames": "hunterNameW", "hunterDefinitions": "hunter", "characteristics": "personality",
        },
        ROOT / "reverse-engineering/evidence/hunter-info-tables-v1.json": {
            "monsters": "evil", "uniqueGearDrops": "dropUniqueGear", "skills": "skill", "subJobSkills": "subJobSkill",
            "growthProperties": "growupProperty", "experience": "exp", "jobTraits": "jobTrait", "ridingPets": "ridingPet",
            "ridingPetSkills": "ridingPetSkill", "ridingPetTraits": "ridingPetTrait",
        },
        ROOT / "reverse-engineering/evidence/serialized-building-tables-v1.json": {
            "buildings": "build", "buildingSkins": "buildSkin", "products": "product", "tradeWagon": "tradeWagon",
        },
    }
    unresolved_by_sheet = {item["worksheetName"]: item for item in unresolved}
    for source, mapping in legacy.items():
        if not source.exists():
            continue
        payload = json.loads(source.read_text())
        for key, sheet in mapping.items():
            if sheet in decoded or key not in payload:
                continue
            record = {"worksheetName": sheet, "decodeStatus": "legacy-decoded-exact", "sourceEvidence": source.relative_to(ROOT).as_posix(), "rows": payload[key]}
            decoded[sheet] = record
            unresolved_by_sheet.pop(sheet, None)
    unresolved = list(unresolved_by_sheet.values())
    output = {"schemaVersion": 1, "contractType": "quicksheet-decoded-workbook", "source": {"workbook": "public spreadsheet 1_i2_M1-enmBOAqqqRKPCBMGnQid6uxlkGvxd-5YsA6U", "asset": args.asset.relative_to(ROOT).as_posix()}, "counts": {"decodedExact": len(decoded), "schemaUnresolved": len(unresolved), "worksheets": len(decoded) + len(unresolved)}, "decoded": decoded, "unresolved": unresolved}
    args.output.write_text(json.dumps(output, ensure_ascii=False, indent=2) + "\n")
    print(f"Decoded exact={len(decoded)} unresolved={len(unresolved)}")


if __name__ == "__main__":
    main()
