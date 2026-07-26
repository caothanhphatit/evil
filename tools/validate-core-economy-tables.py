#!/usr/bin/env python3
"""Validate the exact embedded core-economy evidence contract."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import UnityPy


ROOT = Path(__file__).resolve().parents[1]
ASSET = ROOT / "game-assets/extracted/joined_unity_files/sharedassets1.assets"
EVIDENCE = ROOT / "reverse-engineering/evidence/core-economy-tables-v1.json"

EXPECTED = {
    12566: ("consumables", 5, "2f64120c96abed73bd91078325049f9316f9e3a0c51697efc846fa1a91565508", "85976c0423c6964ddce4f45f83201b378aaf051599b991654ed1bfea68de1b3b"),
    12583: ("gearArmor", 43, "63f74e1ee8ba09e0718fb874dbd23d2179850d03aa0381c3e03580a6369d4354", "7887b4fa952bfc6436dc3ba97cfb31a49823b318a68a34eee5e33035847271ff"),
    12584: ("gearBelt", 34, "551b659d3136b76084a3e6bfd9a396bbe642eda19af38be63d6609797e1cc6b5", "41aaa4bd31355808b66c1d416c7d678424dc0a23c868dee8fc9ca48e54e27748"),
    12585: ("gearBoots", 43, "2040b3745ebdf02d5f0e0e01763e434e3b0b130a6f5359028fe8387ce76eee1b", "7507b215fd97a35c5f780801688b433cab2f7dfa51b19a0b948ce071c8be8eee"),
    12586: ("gearGloves", 43, "14c1eae894265954c4b937837311e891418446ec76e62a06f3c8195563d4999c", "85c796a3dad0db490cdf500a786e259c546cd57214776f16cd2728128c1c63e9"),
    12587: ("gearHelmet", 107, "1631d9d02dfbaa5aaf5b0e7545e5972495f6dc97830b25566a60fbde832ac59e", "56916d809006bbb07ca686a436c69b6ef74bd5d9c3bdedfee97ed5ee9874d247"),
    12588: ("gearNecklace", 43, "87e2141cbbbc6f9e38d07100f0c539441ad8687028ffa5b8e511347481cea069", "5fc10da14e7fe94c0500441118c87b79d7906733572bef6c8ba16774f577a672"),
    12590: ("gearRing", 43, "fa83f9eccb7abb55113722f755d779bb1cbaed4cbbf869d81b4715cf9d719bbb", "582538cfd33679062424a702945eb620d9bfb516cdb718e24cef6e0098baba56"),
    12593: ("gearWeapons", 315, "3f652aba95b6a144fdcb68e64c0593dbc046fc56e3d100387b2f1a669b62995e", "ba71a5807d51ddb119c3e846689000a87a015da49b94faf3b31eee8b7f04762a"),
    12606: ("materials", 369, "3b97173e5685be68aa235407d34fd5d8ceac49e3942d1e8c436809e13c0482f7", "7dc6444edce69f50095f868bfa839b1682f7caad81c739fe21550543fbb964e2"),
    12631: ("runeCraft", 10, "a7009fedde8c972838ec62d0be039e98a16d3b0fb419a2f804ab29ffa4ef1c9c", "d3c0e0da73d6da1b66ddd56391e4820ecdda089239eaf50c5e8cb59114c94762"),
    12632: ("runes", 61, "503a0bfd647994743becf3d40d03d0a749d5856bdcb040fd889c7e9e7a20bccb", "cc143bb2aeb8e8239ade8d98f1373148ef11adadfad814287ed86a389f23b427"),
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_sha256(value: object) -> str:
    encoded = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    return sha256_bytes(encoded)


def by_index(rows: list[dict]) -> dict[int, dict]:
    result = {row["index"]: row for row in rows}
    require(len(result) == len(rows), "duplicate table index")
    return result


def main() -> None:
    evidence = json.loads(EVIDENCE.read_text())
    require(evidence["schemaVersion"] == 1, "unexpected schema version")
    require(evidence["source"]["unityVersion"] == "6000.3.9f1", "unexpected Unity version")
    require(evidence["source"]["assetSha256"] == sha256_bytes(ASSET.read_bytes()), "source asset hash mismatch")

    objects = {obj.path_id: obj for obj in UnityPy.load(str(ASSET)).objects}
    catalog = {row["pathId"]: row for row in evidence["catalog"]}
    require(set(catalog) == set(EXPECTED), "catalog path IDs mismatch")
    for path_id, (key, count, raw_hash, decoded_hash) in EXPECTED.items():
        entry = catalog[path_id]
        require(entry["key"] == key, f"path {path_id} key mismatch")
        require(entry["rowCount"] == count == len(evidence[key]), f"{key} row count mismatch")
        require(entry["decodeStatus"] == "decoded-exact", f"{key} is not exact-decoded")
        require(entry["rawSha256"] == raw_hash, f"{key} catalog raw hash mismatch")
        require(sha256_bytes(objects[path_id].get_raw_data()) == raw_hash, f"{key} source object hash mismatch")
        require(entry["decodedSha256"] == decoded_hash, f"{key} catalog decoded hash mismatch")
        require(canonical_sha256(evidence[key]) == decoded_hash, f"{key} decoded content hash mismatch")

    materials = by_index(evidence["materials"])
    require(materials[1]["localizedNames"]["en"] == "Linen Cloth", "material 1 title mismatch")
    require(materials[1]["unresolvedDummyFields"] == ["3", "1.5", "3"], "material unresolved fields mismatch")
    require(materials[220]["localizedNames"]["en"] == "Wyvern's Growth Stone I", "weapon growth material mismatch")
    require(materials[248]["localizedNames"]["en"] == "Efreet's Growth Stone I", "armor growth material mismatch")
    require(materials[368]["price"] == 10_000_000, "last material price mismatch")

    weapons = by_index(evidence["gearWeapons"])
    require(weapons[0]["localized"]["en"]["title"] == "Junk Sword", "weapon 0 title mismatch")
    require(weapons[0]["firstValue"] == 60.0, "weapon float stat mismatch")
    require(weapons[7]["craftingMaterialsByRating"][0] == {"ids": [21, 26, 83, 123], "quantities": [30, 2, 5, 3]}, "weapon recipe mismatch")
    require(weapons[314]["localized"]["en"]["title"] == "Abyssal Spear", "last weapon mismatch")

    armor = by_index(evidence["gearArmor"])
    require(armor[0]["localized"]["en"]["title"] == "Tattered Armor", "armor 0 title mismatch")
    require(armor[10]["craftingMaterialsByRating"][0] == {"ids": [166, 31], "quantities": [5, 5]}, "armor recipe mismatch")
    require(armor[42]["firstValue"] == 2622, "last armor stat mismatch")

    consumables = by_index(evidence["consumables"])
    require(consumables[0]["localized"]["en"]["title"] == "Healing Potion", "healing potion mismatch")
    require(consumables[0]["craftingMaterialsByLevel"][0] == {"ids": [139], "quantities": [3]}, "healing potion recipe mismatch")
    require(consumables[4]["keepTimeByLevel"][-1] == 270.0, "luck potion duration mismatch")

    rune_craft = by_index(evidence["runeCraft"])
    require(rune_craft[0]["price"] == 5, "rune craft first price mismatch")
    require(rune_craft[9]["price"] == 98_415, "rune craft last price mismatch")
    runes = by_index(evidence["runes"])
    require(runes[1]["localized"]["en"]["title"] == "Mood Consumption Rune", "rune 1 mismatch")
    require(runes[60]["localized"]["en"]["title"] == "Rune of Darkness: Dragon Breath", "last rune mismatch")
    print("Core economy table evidence is valid")


if __name__ == "__main__":
    main()
