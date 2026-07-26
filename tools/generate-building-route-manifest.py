#!/usr/bin/env python3
"""Generate the decoded building-to-popup migration scope from recovered evidence."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REGISTRY_PATH = ROOT / "packages/content/releases/evil-hunter-1.411/building-registry.json"
UI_PATH = ROOT / "reverse-engineering/evidence/building-ui-contract-v1.json"
HIERARCHY_PATH = ROOT / "reverse-engineering/evidence/level1-scene-hierarchy.json"
OUTPUT_PATH = ROOT / "reverse-engineering/evidence/building-route-manifest-v1.json"


def value(field):
    return field.get("value") if isinstance(field, dict) else None


def child_names(game_objects, root_path_id):
    children = {}
    for obj in game_objects:
        parent = obj.get("transform", {}).get("parentGameObjectPathId")
        children.setdefault(parent, []).append(obj)

    rows = []

    def visit(parent, prefix=""):
        for obj in sorted(children.get(parent, []), key=lambda entry: entry["pathId"]):
            path = f"{prefix}/{obj['name']}" if prefix else obj["name"]
            transform = obj.get("transform", {})
            rows.append({
                "path": path,
                "pathId": obj["pathId"],
                "size": transform.get("sizeDelta"),
                "position": transform.get("anchoredPosition"),
            })
            visit(obj["pathId"], path)

    visit(root_path_id)
    return rows


ROUTES = {
    "build_0": ("placement-road", "confirmed"),
    "build_1": ("town-hall-management", "strongly-inferred"),
    "build_2": ("automatic-revival", "confirmed"),
    "build_3": ("trading-post-purchase", "strongly-inferred"),
    "build_4": ("bounty-quest-list", "confirmed"),
    "build_5": ("event-stand", "tentative"),
    "build_6": ("event-stand", "tentative"),
    "build_7": ("weapon-display-sale", "confirmed"),
    "build_8": ("armor-display-sale", "confirmed"),
    "build_9": ("inn-product-service", "strongly-inferred"),
    "build_10": ("gear-crafting", "confirmed"),
    "build_11": ("potion-display-sale", "confirmed"),
    "build_12": ("infirmary-product-service", "confirmed"),
    "build_13": ("restaurant-product-service", "confirmed"),
    "build_14": ("potion-crafting", "confirmed"),
    "build_15": ("gear-enhancement", "tentative"),
    "build_16": ("dungeon-entry", "strongly-inferred"),
    "build_17": ("conversion-cube", "tentative"),
    "build_18": ("mana-cube", "tentative"),
    "build_19": ("tavern-product-service", "strongly-inferred"),
    "build_20": ("accessory-display-sale", "confirmed"),
    "build_21": ("accessory-crafting", "confirmed"),
    "build_22": ("house-management", "tentative"),
    "build_23": ("academy", "tentative"),
    "build_24": ("bank", "tentative"),
    "build_25": ("library", "tentative"),
    "build_26": ("fountain", "tentative"),
    "build_27": ("courage-headstone", "tentative"),
    "build_28": ("training-ground", "tentative"),
}


def main():
    registry = json.loads(REGISTRY_PATH.read_text())
    ui = json.loads(UI_PATH.read_text())
    hierarchy = json.loads(HIERARCHY_PATH.read_text())

    capabilities = {}
    for row in registry["catalogs"]["capabilities"]["rows"]:
        building_id = value(row.get("buildingId"))
        if building_id:
            capabilities[building_id] = {
                "kind": value(row.get("kind")),
                "popupTemplateId": value(row.get("popupTemplateId")),
                "staticDataReady": row.get("readiness", {}).get("staticDataReady", False),
                "runnable": row.get("readiness", {}).get("runnable", False),
                "blockingPaths": row.get("readiness", {}).get("blockingPaths", []),
            }

    popup_templates = {}
    for popup in ui["popupTemplates"]:
        popup_templates[popup["name"]] = {
            "controller": popup["rootControllerClass"],
            "dimensions": popup["panelDimensions"],
            "panelSprite": popup["panelSprite"].get("name"),
            "hierarchyNodeCount": len(popup["hierarchy"]),
            "labelCount": len(popup["labels"]),
            "spriteBindingCount": len(popup["spriteBindings"]),
            "confidence": popup["bindingConfidence"],
            "semanticRole": popup["semanticRole"],
        }

    product_root = next(obj for obj in hierarchy["gameObjects"] if obj.get("name") == "ProductCreatePop")
    product_nodes = child_names(hierarchy["gameObjects"], product_root["pathId"])
    popup_templates["ProductCreatePop"]["criticalNodes"] = [row for row in product_nodes if row["path"].split("/")[-1] in {
            "GridBackground", "TwoButtonGroup", "CloseButton", "UpgradeButton", "Icon",
            "MinusButton", "PlusButton", "Title", "ChangeBorder", "ButtonBorder",
            "HaveMetCount", "SelectMetBorder", "NewVer", "BottomBorder", "GridTitle", "GridBorder",
        }]

    buildings = []
    for row in registry["buildings"]["rows"]:
        building_id = value(row.get("buildId"))
        display_names = value(row.get("displayName")) or {}
        route_id, confidence = ROUTES.get(building_id, ("decoration-or-event", "strongly-inferred"))
        capability = capabilities.get(building_id)
        product_ids = [value(entry.get("id")) for entry in row.get("productIds", {}).get("rows", [])]
        popup_chain = []
        if capability and capability["popupTemplateId"]:
            popup_chain.append(capability["popupTemplateId"].split(":", 1)[1])
        if building_id == "build_3":
            popup_chain = ["BuildingPop"]
        elif building_id in {"build_9", "build_12", "build_13", "build_19"}:
            popup_chain = ["BuildingPop", "ProductCreatePop"]

        blockers = []
        if capability:
            blockers.extend(capability["blockingPaths"])
        if not popup_chain and route_id not in {"placement-road", "decoration-or-event", "event-stand"}:
            blockers.append("popup-template-binding")
        if route_id in {"inn-product-service", "tavern-product-service"}:
            blockers.append("original-runtime-screenshot")

        buildings.append({
            "buildingId": building_id,
            "displayName": display_names.get("en", building_id),
            "routeId": route_id,
            "routeConfidence": confidence,
            "popupChain": popup_chain,
            "capability": capability,
            "productCount": len(product_ids),
            "productIdSample": product_ids[:12],
            "migrationReady": bool(popup_chain) and confidence == "confirmed" and not (capability and capability["blockingPaths"]),
            "blockers": sorted(set(blockers)),
        })

    priority_routes = [
        {
            "routeId": "trading-post-purchase",
            "buildingIds": ["build_3"],
            "popupChain": ["BuildingPop"],
            "decodedWidgets": ["TextTab", "ratingTab", "MoneyChange", "CreatePossible", "RequestStateButton", "GridBorder", "GridSecondBorder"],
            "dataContract": ["material stock", "hunter stock", "requested quantity", "unit price", "town gold", "upgrade level"],
            "status": "layout-and-data-decoded-dispatch-strongly-inferred",
        },
        {
            "routeId": "gear-crafting",
            "buildingIds": ["build_10"],
            "popupChain": ["GearCreatePop"],
            "decodedWidgets": ["GearBorder", "GearBackground", "MainPropertyGroup", "SubPropertyGroup", "required materials", "CreateButton", "CloseButton"],
            "dataContract": ["gear type", "rating", "properties", "material costs", "quantity", "shop stock", "hunter sale price"],
            "status": "template-and-recipe-tables-decoded-native-dispatch-unresolved",
        },
        {
            "routeId": "potion-shop-and-crafting",
            "buildingIds": ["build_11", "build_14"],
            "popupChainByBuilding": {"build_11": ["BuildingPop"], "build_14": ["ConsumCreatePop"]},
            "decodedWidgets": ["potion display grid", "hunter tab", "ConsumCreatePop product frame", "Required Materials", "cooldown", "effect", "CreateButton"],
            "dataContract": ["display stock", "sale price", "consumable recipe", "level", "cooldown", "effect", "material costs"],
            "status": "route-split-confirmed-native-dispatch-unresolved",
        },
        {
            "routeId": "building-product-service",
            "buildingIds": ["build_9", "build_12", "build_13", "build_19"],
            "popupChain": ["BuildingPop", "ProductCreatePop"],
            "decodedWidgets": ["Production/Hunters tabs", "capacity", "product list", "Produce", "quantity -/+", "quantity steps", "conversion material grid", "Produce", "Close"],
            "dataContract": ["product unlock level", "capacity by building level", "stock", "service effect", "duration", "fee", "conversion options"],
            "statusByBuilding": {
                "build_9": "serialized-products-and-layout-decoded-runtime-capture-missing",
                "build_12": "confirmed-by-user-runtime-capture",
                "build_13": "confirmed-by-user-runtime-capture",
                "build_19": "serialized-products-and-layout-decoded-runtime-capture-missing",
            },
        },
    ]

    screenshot_coverage = {
        "build_1": ["photo_6275977522440770467_y.jpg", "photo_6275977522440770468_y.jpg"],
        "build_3": ["photo_6275977522440770365_y.jpg", "photo_6275977522440770470_y.jpg"],
        "build_4": [
            "photo_6275977522440770472_y.jpg", "photo_6275977522440770473_y.jpg",
            "photo_6275977522440770474_y.jpg", "photo_6275977522440770475_y.jpg",
        ],
        "build_7": ["photo_6275977522440770476_y.jpg"],
        "build_8": ["photo_6275977522440770471_y.jpg"],
        "build_9": ["photo_6275977522440770466_y.jpg"],
        "build_10": [
            "photo_6275977522440770477_y.jpg", "photo_6275977522440770478_y.jpg",
            "photo_6275977522440770479_y.jpg", "photo_6275977522440770480_y.jpg",
        ],
        "build_12": ["photo_6275977522440770367_y.jpg", "photo_6275977522440770469_y.jpg"],
        "build_13": ["photo_6275977522440770366_y.jpg"],
        "build_16": ["photo_6275977522440770481_y.jpg"],
        "build_19": ["photo_6275977522440770485_y.jpg"],
        "build_22": ["photo_6275977522440770483_y.jpg"],
        "build_23": ["photo_6275977522440770482_y.jpg"],
        "build_53": ["photo_6275977522440770484_y.jpg"],
        "product-create": ["photo_6275977522440770368_y.jpg", "photo_6275977522440770369_y.jpg"],
        "world-map": ["photo_6275977522440770364_y.jpg"],
    }

    manifest = {
        "schemaVersion": 1,
        "manifestType": "building-route-migration-scope",
        "generatedFrom": [str(REGISTRY_PATH.relative_to(ROOT)), str(UI_PATH.relative_to(ROOT)), str(HIERARCHY_PATH.relative_to(ROOT)), "screenshot/*.jpg"],
        "policy": "A route is migration-ready only when popup hierarchy, data contract, asset binding, actions, and building dispatch are all resolved.",
        "popupTemplates": popup_templates,
        "priorityRoutes": priority_routes,
        "screenshotCoverage": screenshot_coverage,
        "buildings": buildings,
        "summary": {
            "buildingCount": len(buildings),
            "priorityBuildingCount": 8,
            "confirmedRouteCount": sum(row["routeConfidence"] == "confirmed" for row in buildings),
            "unresolvedPopupBindingCount": sum("popup-template-binding" in row["blockers"] for row in buildings),
        },
    }
    OUTPUT_PATH.write_text(json.dumps(manifest, ensure_ascii=True, indent=2) + "\n")


if __name__ == "__main__":
    main()
