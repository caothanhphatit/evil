#!/usr/bin/env python3
"""Validate deterministic sentinels in the building capability evidence contract."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CONTRACT = ROOT / "reverse-engineering/evidence/building-capability-contract-v1.json"

EXPECTED_NAMES = {
    2: "Sanctuary of Resurrection",
    3: "Trading Post",
    4: "Bounty Hut",
    7: "Weapon Shop",
    8: "Armor Shop",
    10: "Blacksmith",
    11: "Potion Shop",
    14: "Alchemist's Home",
    20: "Accessory Shop",
    21: "Jeweler",
}

EXPECTED_LAYOUTS = {
    "BuildingPop": (4368, 560.0, 900.0),
    "BuildingReviveCheckPop": (254, 450.0, 380.0),
    "ConsumCreatePop": (958, 450.0, 830.0),
    "GearCreatePop": (708, 450.0, 950.0),
    "QuestPop": (2891, 560.0, 1100.0),
    "RequestPop": (1879, 480.0, 820.0),
    "TradeWagonExchangePop": (927, 450.0, 710.0),
}

EXPECTED_ECONOMY_TABLES = {
    2: [],
    3: [("materials", 369)],
    4: [],
    7: [("gearWeapons", 315)],
    8: [("gearArmor", 43), ("gearHelmet", 107), ("gearBoots", 43), ("gearGloves", 43)],
    10: [("gearWeapons", 315), ("gearArmor", 43), ("gearHelmet", 107), ("gearBoots", 43), ("gearGloves", 43)],
    11: [("consumables", 5)],
    14: [("consumables", 5)],
    20: [("gearBelt", 34), ("gearNecklace", 43), ("gearRing", 43)],
    21: [("gearBelt", 34), ("gearNecklace", 43), ("gearRing", 43)],
}


def main() -> None:
    contract = json.loads(CONTRACT.read_text())
    assert contract["schemaVersion"] == 1
    assert contract["runtimeCompatibility"] == "not-claimed"
    assert contract["sentinels"] == {
        "targetBuildingIds": list(EXPECTED_NAMES),
        "buildingPopTypeIndex": 388,
        "buildingPopToken": 33554816,
        "serializedBuildingCount": 79,
        "uiControllerCount": 92,
    }
    buildings = {item["buildingId"]: item for item in contract["buildings"]}
    assert set(buildings) == set(EXPECTED_NAMES)
    for building_id, name in EXPECTED_NAMES.items():
        item = buildings[building_id]
        assert item["identity"]["en"] == name
        assert item["capability"]["descriptionEn"]
        assert item["progression"]["maxLevel"] == len(item["progression"]["requiredGold"])
        assert item["ui"]["nativeCallSiteStatus"] == "unresolved"
        assert [(table["name"], table["rowCount"]) for table in item["economyTables"]["tables"]] == EXPECTED_ECONOMY_TABLES[building_id]

    for name, (script_path_id, width, height) in EXPECTED_LAYOUTS.items():
        layout = contract["popupLayouts"][name]
        assert layout["controller"]["scriptPathId"] == script_path_id
        assert layout["controller"]["class"] == name
        assert layout["panel"]["width"] == width
        assert layout["panel"]["height"] == height

    tokens = {item["token"] for item in contract["buildingPopControllerEvidence"]["fieldReferences"]}
    assert tokens == {67114492, 67114493, 67114494, 67114496, 67114497, 67114499, 67114501, 67114503, 67114505}
    print(f"Validated {len(buildings)} building capabilities and {len(EXPECTED_LAYOUTS)} popup layouts")


if __name__ == "__main__":
    main()
