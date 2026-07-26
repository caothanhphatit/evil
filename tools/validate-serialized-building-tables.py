#!/usr/bin/env python3
"""Validate the embedded building-table evidence contract."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ASSET = ROOT / "game-assets/extracted/joined_unity_files/sharedassets1.assets"
EVIDENCE = ROOT / "reverse-engineering/evidence/serialized-building-tables-v1.json"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def main() -> None:
    evidence = json.loads(EVIDENCE.read_text())
    require(evidence["schemaVersion"] == 1, "unexpected schema version")
    require(evidence["source"]["unityVersion"] == "6000.3.9f1", "unexpected Unity version")
    require(
        evidence["source"]["assetSha256"] == hashlib.sha256(ASSET.read_bytes()).hexdigest(),
        "source asset hash mismatch",
    )

    catalog = {row["pathId"]: row for row in evidence["catalog"]}
    require(catalog[12558]["name"] == "build_global", "building table identity mismatch")
    require(catalog[12558]["rowCount"] == 79, "building row count mismatch")
    require(catalog[12558]["decodeStatus"] == "decoded", "building table is not decoded")
    require(catalog[12593]["rowCount"] == 315, "weapon row count mismatch")
    require(catalog[12615]["rowCount"] == 52, "product row count mismatch")
    require(catalog[12638]["rowCount"] == 3, "trade-wagon row count mismatch")

    buildings = {row["index"]: row for row in evidence["buildings"]}
    require(len(buildings) == 79, "decoded building count mismatch")
    require(buildings[1]["localized"]["en"]["title"] == "Town Hall", "building 1 mismatch")
    require(buildings[3]["localized"]["en"]["title"] == "Trading Post", "building 3 mismatch")
    require(buildings[7]["localized"]["en"]["title"] == "Weapon Shop", "building 7 mismatch")
    require(buildings[7]["maxLevel"] == 5, "weapon-shop max level mismatch")
    require(buildings[7]["requiredGold"] == [660, 5280, 17820, 53460, 160380], "weapon-shop costs mismatch")
    require(buildings[7]["possibleBuild"] == [2, 5, 7, 9, 11], "weapon-shop conditions mismatch")
    require(buildings[11]["maxLevel"] == 8, "potion-shop max level mismatch")
    require(buildings[24]["requiredGold"] == [32400, 97200, 1944000], "bank costs mismatch")
    require(buildings[1]["requiredGold"][-1] == 39366000, "town-hall final cost mismatch")

    skins = {(row["buildingId"], row["skinId"]): row for row in evidence["buildingSkins"]}
    require(len(evidence["buildingSkins"]) == 61, "building-skin count mismatch")
    require(skins[(7, 1)]["titles"]["en"] == "Middle Ages Weapon Shop", "weapon skin mismatch")
    require(skins[(62, 1)]["titles"]["en"] == "Middle Ages Medical Storage Room", "medical-storage skin mismatch")

    products = evidence["products"]
    require(len(products) == 52, "product count mismatch")
    require(products[0]["duration"] == 10.0, "product duration float decode mismatch")
    infirmary_products = [row for row in products if row["buildingId"] == 12]
    require(len(infirmary_products) == 7, "infirmary product count mismatch")
    require(infirmary_products[0]["localized"]["en"]["title"] == "Linen Bandage", "infirmary first product mismatch")

    trade = evidence["tradeWagon"]
    require(len(trade) == 3, "trade-wagon decode count mismatch")
    require(trade[0]["localized"]["en"]["title"] == "Prince of Darkness' Castle Rune Exchange", "trade row 0 mismatch")
    print("Serialized building table evidence is valid")


if __name__ == "__main__":
    main()
