#!/usr/bin/env python3
"""Extract the evidence-backed capability contract for the core town buildings."""

from __future__ import annotations

import csv
import hashlib
import json
from pathlib import Path

import UnityPy

from scene_evidence_lib import mono_behaviour_header


ROOT = Path(__file__).resolve().parents[1]
SERIALIZED = ROOT / "reverse-engineering/evidence/serialized-building-tables-v1.json"
IL2CPP = ROOT / "reverse-engineering/evidence/il2cpp-building-metadata-v1.json"
UI = ROOT / "reverse-engineering/evidence/building-ui-contract-v1.json"
ECONOMY = ROOT / "reverse-engineering/evidence/core-economy-tables-v1.json"
SCENE = ROOT / "reverse-engineering/evidence/level1-scene-hierarchy.json"
MONOSCRIPTS = ROOT / "reverse-engineering/evidence/monoscripts.csv"
LEVEL1 = ROOT / "game-assets/extracted/joined_unity_files/level1"
OUT = ROOT / "reverse-engineering/evidence/building-capability-contract-v1.json"

TARGET_IDS = (2, 3, 4, 7, 8, 10, 11, 14, 20, 21)

# These bindings combine the exact build-table description with a matching original
# popup/controller. They are not represented as native call-site proof.
CAPABILITY_BINDINGS = {
    2: ("automatic-revival", "BuildingReviveCheckPop", "correlated-original-controller"),
    3: ("loot-purchase-reservations", "RequestPop", "correlated-original-controller"),
    4: ("bounty-quest-list", "QuestPop", "correlated-original-controller"),
    7: ("weapon-display-and-sale", "BuildingPop", "correlated-buildingpop-field-family"),
    8: ("armor-display-and-sale", "BuildingPop", "correlated-buildingpop-field-family"),
    10: ("weapon-and-armor-crafting", "GearCreatePop", "correlated-buildingpop-field-family"),
    11: ("potion-display-and-sale", "BuildingPop", "correlated-buildingpop-field-family"),
    14: ("potion-crafting", "ConsumCreatePop", "correlated-buildingpop-field-family"),
    20: ("accessory-display-and-sale", "BuildingPop", "correlated-buildingpop-field-family"),
    21: ("accessory-crafting", "GearCreatePop", "correlated-buildingpop-field-family"),
}

ECONOMY_TABLE_BINDINGS = {
    2: [],
    3: ["materials"],
    4: [],
    7: ["gearWeapons"],
    8: ["gearArmor", "gearHelmet", "gearBoots", "gearGloves"],
    10: ["gearWeapons", "gearArmor", "gearHelmet", "gearBoots", "gearGloves"],
    11: ["consumables"],
    14: ["consumables"],
    20: ["gearBelt", "gearNecklace", "gearRing"],
    21: ["gearBelt", "gearNecklace", "gearRing"],
}

POPUP_SENTINELS = {
    "BuildingPop": (932, 4368, 560.0, 900.0),
    "BuildingReviveCheckPop": (755, 254, 450.0, 380.0),
    "ConsumCreatePop": (1264, 958, 450.0, 830.0),
    "GearCreatePop": (1272, 708, 450.0, 950.0),
    "QuestPop": (1919, 2891, 560.0, 1100.0),
    "RequestPop": (2504, 1879, 480.0, 820.0),
    "TradeWagonExchangePop": (1896, 927, 450.0, 710.0),
}

# Field tokens are stable metadata identities. Names and type names are poisoned,
# so the contract preserves only the readable fragments and does not repair them.
BUILDING_POP_FIELD_TOKENS = (
    67114492,  # request list-like reference
    67114493,  # request controller-like reference
    67114494,  # exchange controller-like reference
    67114496,  # gear display-like reference
    67114497,  # gear create-like reference
    67114499,  # consumable create-like reference
    67114501,  # potion display-like reference
    67114503,  # revive hunter-like reference
    67114505,  # trade-like reference
)


def digest(path: Path) -> dict:
    payload = path.read_bytes()
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "bytes": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }


def load_scripts() -> dict[int, dict]:
    with MONOSCRIPTS.open(newline="") as handle:
        return {int(row["path_id"]): row for row in csv.DictReader(handle)}


def popup_components() -> dict[str, dict]:
    scripts = load_scripts()
    environment = UnityPy.load(str(LEVEL1))
    game_object_names = {}
    components_by_game_object: dict[int, list[dict]] = {}
    for reader in environment.objects:
        if reader.type.name == "GameObject":
            try:
                game_object_names[reader.path_id] = reader.read().m_Name
            except Exception:
                continue
        elif reader.type.name == "MonoBehaviour":
            try:
                header = mono_behaviour_header(reader.get_raw_data())
            except Exception:
                continue
            script = scripts.get(header["scriptPathId"])
            components_by_game_object.setdefault(header["gameObjectPathId"], []).append({
                "componentPathId": reader.path_id,
                "scriptPathId": header["scriptPathId"],
                "class": script["class"] if script else None,
                "assembly": script["assembly"] if script else None,
            })

    result = {}
    for popup_name, (path_id, script_path_id, _, _) in POPUP_SENTINELS.items():
        components = components_by_game_object.get(path_id, [])
        controller = next((item for item in components if item["scriptPathId"] == script_path_id), None)
        if game_object_names.get(path_id) != popup_name or controller is None:
            raise ValueError(f"Popup sentinel failed for {popup_name}")
        result[popup_name] = controller
    return result


def popup_layouts(scene: dict, controllers: dict[str, dict]) -> dict[str, dict]:
    game_objects = {item["pathId"]: item for item in scene["gameObjects"]}
    children: dict[int, list[dict]] = {}
    for item in scene["gameObjects"]:
        parent = item.get("transform", {}).get("parentGameObjectPathId")
        if parent is not None:
            children.setdefault(parent, []).append(item)

    result = {}
    for name, (path_id, script_path_id, width, height) in POPUP_SENTINELS.items():
        root = game_objects[path_id]
        panels = [
            child for child in children.get(path_id, [])
            if child.get("transform", {}).get("sizeDelta") == {"x": width, "y": height}
        ]
        if len(panels) != 1:
            raise ValueError(f"Expected one {width}x{height} panel under {name}, got {len(panels)}")
        panel = panels[0]
        result[name] = {
            "rootGameObjectPathId": path_id,
            "rootActiveInScene": root["active"],
            "controller": controllers[name],
            "panel": {
                "gameObjectPathId": panel["pathId"],
                "name": panel["name"],
                "width": width,
                "height": height,
            },
            "bindingConfidence": "confirmed-serialized-component",
        }
    return result


def main() -> None:
    serialized = json.loads(SERIALIZED.read_text())
    il2cpp = json.loads(IL2CPP.read_text())
    ui = json.loads(UI.read_text())
    economy = json.loads(ECONOMY.read_text())
    scene = json.loads(SCENE.read_text())

    rows = {row["index"]: row for row in serialized["buildings"]}
    building_pop = next(
        item for item in il2cpp["candidateTypes"]
        if item["typeIndex"] == 388 and item["name"]["value"] == "BuildingPop"
    )
    field_by_token = {field["token"]: field for field in building_pop["fields"]}
    missing_tokens = set(BUILDING_POP_FIELD_TOKENS) - set(field_by_token)
    if missing_tokens:
        raise ValueError(f"BuildingPop field sentinels missing: {sorted(missing_tokens)}")

    layouts = popup_layouts(scene, popup_components())
    buildings = []
    for building_id in TARGET_IDS:
        row = rows[building_id]
        capability, popup, confidence = CAPABILITY_BINDINGS[building_id]
        buildings.append({
            "buildingId": building_id,
            "identity": {
                "en": row["localized"]["en"]["title"],
                "vi": row["localized"]["vi"]["title"],
                "confidence": "confirmed-serialized-row",
            },
            "capability": {
                "id": capability,
                "descriptionEn": row["localized"]["en"]["description"],
                "descriptionVi": row["localized"]["vi"]["description"],
                "upgradeEffectEn": row["localized"]["en"]["subDescription"],
                "levelLabelsEn": row["localized"]["en"]["descriptionText"],
                "confidence": "confirmed-serialized-row",
            },
            "progression": {
                "maxBuild": row["maxBuild"],
                "maxLevel": row["maxLevel"],
                "requiredTownHallLevels": row["possibleBuild"],
                "requiredGold": row["requiredGold"],
                "requiredMaterials": row["requiredMaterials"],
                "firstValues": row["firstValues"],
                "secondValues": row["secondValues"],
                "thirdValues": row["thirdValues"],
                "confidence": "confirmed-serialized-row",
            },
            "economyTables": {
                "tables": [
                    {"name": name, "rowCount": len(economy[name])}
                    for name in ECONOMY_TABLE_BINDINGS[building_id]
                ],
                "bindingConfidence": "correlated-table-family" if ECONOMY_TABLE_BINDINGS[building_id] else "not-applicable",
            },
            "ui": {
                "managementPopup": "BuildingPop",
                "capabilityPopup": popup,
                "layoutRef": popup,
                "bindingConfidence": confidence,
                "nativeCallSiteStatus": "unresolved",
            },
        })

    output = {
        "schemaVersion": 1,
        "contractType": "building-capability-evidence",
        "runtimeCompatibility": "not-claimed",
        "policy": "No missing value or native call-site binding is inferred.",
        "sources": [digest(path) for path in (SERIALIZED, IL2CPP, UI, ECONOMY, SCENE, MONOSCRIPTS, LEVEL1)],
        "buildingPopControllerEvidence": {
            "typeIndex": building_pop["typeIndex"],
            "token": building_pop["token"],
            "fieldReferences": [field_by_token[token] for token in BUILDING_POP_FIELD_TOKENS],
            "interpretationConfidence": "field-name-and-type-fragments-only",
        },
        "popupLayouts": layouts,
        "buildings": buildings,
        "unboundRelatedUi": [{
            "popup": "TradeWagonExchangePop",
            "reason": "Original controller and layout are confirmed, but no target building ID call-site is proven.",
        }],
        "unresolved": [
            "Native click-handler call sites that bind each building ID to a capability popup.",
            "TraderInfomation_global payload schema and exact request pricing rules.",
            "Hunter autonomous shop purchase/equip decision method bodies.",
            "Recipe-to-building filtering beyond the exact serialized buildingId columns already decoded elsewhere.",
        ],
        "sentinels": {
            "targetBuildingIds": list(TARGET_IDS),
            "buildingPopTypeIndex": 388,
            "buildingPopToken": 33554816,
            "serializedBuildingCount": len(serialized["buildings"]),
            "uiControllerCount": len(ui["controllers"]),
        },
    }
    OUT.write_text(json.dumps(output, indent=2, ensure_ascii=True) + "\n")
    print(f"Wrote {len(buildings)} building capabilities and {len(layouts)} popup layouts -> {OUT}")


if __name__ == "__main__":
    main()
