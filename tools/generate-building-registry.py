#!/usr/bin/env python3
"""Generate the fail-closed 1.411 building registry from recovered evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_TABLES = ROOT / "reverse-engineering/evidence/serialized-building-tables-v1.json"
DEFAULT_UI = ROOT / "reverse-engineering/evidence/building-ui-contract-v1.json"
DEFAULT_ASSETS = ROOT / "reverse-engineering/evidence/building-asset-evidence-v1.json"
DEFAULT_ECONOMY = ROOT / "reverse-engineering/evidence/core-economy-tables-v1.json"
DEFAULT_CAPABILITIES = ROOT / "reverse-engineering/evidence/building-capability-contract-v1.json"
DEFAULT_CONDITIONS = ROOT / "reverse-engineering/evidence/building-condition-evidence-v1.json"
DEFAULT_SKINS = ROOT / "reverse-engineering/evidence/building-skin-evidence-v1.json"
DEFAULT_OUTPUT = ROOT / "packages/content/releases/evil-hunter-1.411/building-registry.json"

SERIALIZED_SOURCE_ID = "serialized-building-tables-v1"
UI_SOURCE_ID = "building-ui-contract-v1"
ASSET_SOURCE_ID = "building-asset-evidence-v1"
ECONOMY_SOURCE_ID = "core-economy-tables-v1"
CAPABILITY_SOURCE_ID = "building-capability-contract-v1"
CONDITION_SOURCE_ID = "building-condition-evidence-v1"
SKIN_SOURCE_ID = "building-skin-evidence-v1"


def repository_path(path: Path) -> str:
    return path.resolve().relative_to(ROOT).as_posix()


def display_path(path: Path) -> str:
    try:
        return repository_path(path)
    except ValueError:
        return str(path.resolve())


def evidence_source(source_id: str, path: Path) -> dict[str, Any]:
    payload = path.read_bytes()
    return {
        "id": source_id,
        "path": repository_path(path),
        "bytes": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }


def ref(locator: str, method: str = "serialized-row", note: str | None = None) -> dict[str, str]:
    result = {"sourceId": SERIALIZED_SOURCE_ID, "locator": locator, "method": method}
    if note is not None:
        result["note"] = note
    return result


def economy_ref(locator: str, note: str | None = None) -> dict[str, str]:
    result = {"sourceId": ECONOMY_SOURCE_ID, "locator": locator, "method": "serialized-row"}
    if note is not None:
        result["note"] = note
    return result


def asset_ref(locator: str, note: str | None = None) -> dict[str, str]:
    result = {"sourceId": ASSET_SOURCE_ID, "locator": locator, "method": "asset-object"}
    if note is not None:
        result["note"] = note
    return result


def capability_ref(locator: str, method: str = "serialized-row", note: str | None = None) -> dict[str, str]:
    result = {"sourceId": CAPABILITY_SOURCE_ID, "locator": locator, "method": method}
    if note is not None:
        result["note"] = note
    return result


def condition_ref(locator: str, note: str | None = None) -> dict[str, str]:
    result = {"sourceId": CONDITION_SOURCE_ID, "locator": locator, "method": "localization-entry"}
    if note is not None:
        result["note"] = note
    return result


def skin_ref(locator: str, method: str = "serialized-row", note: str | None = None) -> dict[str, str]:
    result = {"sourceId": SKIN_SOURCE_ID, "locator": locator, "method": method}
    if note is not None:
        result["note"] = note
    return result


def resolved_from(value: Any, evidence: list[dict[str, str]], confidence: str = "confirmed") -> dict[str, Any]:
    return {
        "state": "resolved",
        "confidence": confidence,
        "value": value,
        "evidence": evidence,
        "requiredEvidence": None,
    }


def resolved(value: Any, locator: str, *, method: str = "serialized-row", note: str | None = None) -> dict[str, Any]:
    return {
        "state": "resolved",
        "confidence": "confirmed",
        "value": value,
        "evidence": [ref(locator, method, note)],
        "requiredEvidence": None,
    }


def unresolved(required_evidence: str, evidence: list[dict[str, str]] | None = None) -> dict[str, Any]:
    return {
        "state": "unresolved",
        "confidence": "unknown",
        "value": None,
        "evidence": evidence or [],
        "requiredEvidence": required_evidence,
    }


def resolved_binding(locator: str, note: str | None = None) -> dict[str, Any]:
    return {
        "state": "resolved",
        "confidence": "confirmed",
        "evidence": [ref(locator, note=note)],
        "requiredEvidence": None,
    }


def unresolved_binding(required_evidence: str, evidence: list[dict[str, str]] | None = None) -> dict[str, Any]:
    return {
        "state": "unresolved",
        "confidence": "unknown",
        "evidence": evidence or [],
        "requiredEvidence": required_evidence,
    }


def empty_unresolved_collection(required_evidence: str, evidence: list[dict[str, str]] | None = None) -> dict[str, Any]:
    return {"binding": unresolved_binding(required_evidence, evidence), "rows": []}


def unresolved_service_data(required_evidence: str) -> dict[str, Any]:
    return {
        "binding": unresolved_binding(required_evidence),
        "sourceType": unresolved(required_evidence),
        "requiredLevel": unresolved(required_evidence),
        "serviceTimeMs": unresolved(required_evidence),
        "effectValue": unresolved(required_evidence),
        "useMoney": unresolved(required_evidence),
        "completionCounts": unresolved(required_evidence),
        "requiredCashCount": unresolved(required_evidence),
        "cashCompletionCount": unresolved(required_evidence),
        "requiredElementalCount": unresolved(required_evidence),
        "elementalCompletionCount": unresolved(required_evidence),
    }


def amount(key: str, item_id: str, quantity: int, locator: str) -> dict[str, Any]:
    return {
        "key": key,
        "itemId": resolved(item_id, locator),
        "quantity": resolved(quantity, locator),
    }


def build_costs(building: dict[str, Any], level_index: int) -> list[dict[str, Any]]:
    building_index = building["index"]
    base = f"buildings[index={building_index}]"
    rows = [amount("gold", "currency:gold", building["requiredGold"][level_index], f"{base}.requiredGold[{level_index}]")]
    for slot, material in enumerate(building["requiredMaterials"]):
        material_id = material["ids"][level_index]
        quantity = material["quantities"][level_index]
        if material_id < 0 or quantity <= 0:
            continue
        rows.append(
            amount(
                f"material-{slot}",
                f"material:{material_id}",
                quantity,
                f"{base}.requiredMaterials[{slot}][{level_index}]",
            )
        )
    return rows


def build_conditions(
    building: dict[str, Any],
    level_index: int,
    condition_rows: dict[str, dict[str, Any]],
    evaluator: dict[str, Any],
) -> dict[str, Any]:
    building_index = building["index"]
    locator = f"buildings[index={building_index}].possibleBuild[{level_index}]"
    row_key = f"build_{building_index}:level:{level_index + 1}"
    condition_row = condition_rows[row_key]
    if condition_row["requiredTownHallLevel"] != building["possibleBuild"][level_index]:
        raise ValueError(f"condition evidence operand mismatch: {row_key}")
    condition_note = evaluator["reason"]
    condition_evidence = [
        ref(locator),
        condition_ref(f"conditionRows[key={row_key}]", note=condition_note),
        condition_ref("localizationRows[key=buildpop_9|buildtoast_0]", note=condition_note),
    ]
    return {
        "binding": resolved_from_binding(condition_evidence),
        "rows": [
            {
                "key": "possible-build",
                "kind": resolved("possibleBuild", locator),
                "subjectId": resolved_from(
                    evaluator["subjectId"], condition_evidence, evaluator["confidence"]
                ),
                "operator": resolved_from(
                    evaluator["operator"], condition_evidence, evaluator["confidence"]
                ),
                "operand": resolved(building["possibleBuild"][level_index], locator),
            }
        ],
    }


def capability_references(building_id: int, capabilities: dict[int, dict[str, Any]]) -> dict[str, Any]:
    capability = capabilities.get(building_id)
    if capability is None:
        return empty_unresolved_collection(
            "Decode the building controller dispatch and bind this building to an exact capability."
        )
    locator = f"buildings[buildingId={building_id}].capability.id"
    capability_id = f"capability:{capability['capability']['id']}"
    return {
        "binding": resolved_from_binding([capability_ref(locator)]),
        "rows": [
            {
                "key": capability_id.replace(":", "-"),
                "id": resolved_from(capability_id, [capability_ref(locator)]),
            }
        ],
    }


def make_level(
    building: dict[str, Any],
    level_index: int,
    products: list[dict[str, Any]],
    capabilities: dict[int, dict[str, Any]],
    condition_rows: dict[str, dict[str, Any]],
    evaluator: dict[str, Any],
) -> dict[str, Any]:
    building_index = building["index"]
    level = level_index + 1
    locator = f"buildings[index={building_index}]"
    return {
        "key": f"level-{level}",
        "level": resolved(level, f"{locator}.maxLevel"),
        "conditions": build_conditions(building, level_index, condition_rows, evaluator),
        "upgradeCosts": {
            "binding": resolved_binding(f"{locator}.requiredGold[{level_index}]"),
            "rows": build_costs(building, level_index),
        },
        "upgradeDurationMs": unresolved(
            "AdminBuildData contains no construction duration; decode the native construction timer or capture an original runtime trace."
        ),
        "inventoryCapacity": unresolved(
            "Decode the native consumer of AdminBuildData entryCounts/firstValues before assigning an inventory meaning."
        ),
        "productionSlots": unresolved(
            "Decode the native consumer of AdminBuildData entryCounts/firstValues before assigning a production-slot meaning."
        ),
        "capabilityIds": capability_references(building_index, capabilities),
        "productIds": product_references(building_index, level, products),
    }


def product_references(building_id: int, level: int | None, products: list[dict[str, Any]]) -> dict[str, Any]:
    matching = [
        product for product in products
        if product["buildingId"] == building_id and (level is None or product["level"] <= level - 1)
    ]
    locator = f"products[buildingId={building_id}]"
    return {
        "binding": resolved_binding(locator, "References are joined by the serialized product buildingId.") if matching else resolved_binding(
            f"buildings[index={building_id}]", "No product row references this building at the selected scope."
        ),
        "rows": [
            {
                "key": f"product-{product['index']}",
                "id": resolved(f"product:{product['index']}", f"products[index={product['index']}].buildingId"),
            }
            for product in matching
        ],
    }


def make_building(
    building: dict[str, Any],
    products: list[dict[str, Any]],
    economy_product_ids: dict[int, list[tuple[str, str]]],
    visual_bindings: dict[int, dict[str, Any]],
    capabilities: dict[int, dict[str, Any]],
    condition_rows: dict[str, dict[str, Any]],
    evaluator: dict[str, Any],
) -> dict[str, Any]:
    building_index = building["index"]
    build_id = f"build_{building_index}"
    locator = f"buildings[index={building_index}]"
    build_rows = []
    for level_index in range(building["maxLevel"]):
        level = level_index + 1
        build_rows.append(
            {
                "key": f"source-level-{level}",
                "sourceRowId": resolved(f"building:{building_index}:level:{level}", locator),
                "buildId": resolved(build_id, f"{locator}.index"),
                "level": resolved(level, f"{locator}.maxLevel"),
                "conditions": build_conditions(building, level_index, condition_rows, evaluator),
                "costs": {
                    "binding": resolved_binding(f"{locator}.requiredGold[{level_index}]"),
                    "rows": build_costs(building, level_index),
                },
                "durationMs": unresolved(
                    "AdminBuildData contains no construction duration; decode the native construction timer or capture an original runtime trace."
                ),
            }
        )

    visual_blocker = (
        "A serialized scene binding or original runtime trace must join this semantic build ID to its "
        "controller, popup, placement, sorting, and collider. Name similarity alone is insufficient."
    )
    visual = visual_bindings.get(building_index)
    if visual is None:
        sprite_asset_id = unresolved(
            "An exact serialized source-key join must bind this AdminBuildData index to a base build animation and sprite sequence."
        )
        visual_evidence = []
    else:
        visual_locator = f"buildingVisualBindings[sourceBuildIndex={building_index}]"
        sprite_asset_id = resolved_from(
            visual["sourceBuildKey"],
            [asset_ref(visual_locator, "Exact build_<index> join across AdminBuildData, AnimationClip, and AnimatorController.")],
        )
        visual_evidence = [asset_ref(visual_locator, "Base animation/controller/sprite binding is exact; remaining visual fields are unresolved.")]
    building_product_ids = product_references(building_index, None, products)
    for product_id, locator in economy_product_ids.get(building_index, []):
        building_product_ids["rows"].append(
            {"key": product_id.replace(":", "-"), "id": resolved_from(product_id, [economy_ref(locator)])}
        )
    return {
        "key": build_id,
        "buildId": resolved(build_id, f"{locator}.index"),
        "internalName": unresolved(
            "The serialized table has localized titles but no separate stable internal building name."
        ),
        "displayName": resolved(
            {locale: text["title"] for locale, text in sorted(building["localized"].items())},
            f"{locator}.localized.*.title",
        ),
        "category": unresolved(
            "Decode the AdminBuildData.type enum before assigning a semantic building category.",
            [ref(f"{locator}.type")],
        ),
        "sourceData": {
            "binding": resolved_binding(locator, "Raw AdminBuildData fields retain their public worksheet names without enum interpretation."),
            "sourceType": resolved(building["type"], f"{locator}.type"),
            "maxBuild": resolved(building["maxBuild"], f"{locator}.maxBuild"),
            "gridSize": resolved(building["size"], f"{locator}.size"),
            "movable": resolved(building["movable"], f"{locator}.movable"),
            "visibility": resolved(building["visibility"], f"{locator}.visibility"),
            "compatibleSkin": resolved(building["compatibleSkin"], f"{locator}.compatibleSkin"),
            "inBuildingFlag": resolved(building["inBuildingFlag"], f"{locator}.inBuildingFlag"),
            "possibleRemove": resolved(building["possibleRemove"], f"{locator}.possibleRemove"),
            "createBuild": resolved(building["createBuild"], f"{locator}.createBuild"),
            "entryCounts": resolved(building["entryCounts"], f"{locator}.entryCounts"),
            "firstValues": resolved(building["firstValues"], f"{locator}.firstValues"),
            "secondValues": resolved(building["secondValues"], f"{locator}.secondValues"),
            "thirdValues": resolved(building["thirdValues"], f"{locator}.thirdValues"),
        },
        "buildRows": {"binding": resolved_binding(locator), "rows": build_rows},
        "levels": {
            "binding": resolved_binding(f"{locator}.maxLevel"),
            "rows": [
                make_level(building, index, products, capabilities, condition_rows, evaluator)
                for index in range(building["maxLevel"])
            ],
        },
        "tradeRules": empty_unresolved_collection(
            "Decode native trade/request dispatch and bind exact item direction, unit price, limit, and conditions to this build ID."
        ),
        "productIds": building_product_ids,
        "capabilityIds": capability_references(building_index, capabilities),
        "visualBinding": {
            "binding": unresolved_binding(visual_blocker, visual_evidence),
            "spriteAssetId": sprite_asset_id,
            "controllerClass": unresolved(visual_blocker, visual_evidence),
            "popupClass": unresolved(visual_blocker, visual_evidence),
            "townPosition": unresolved(visual_blocker, visual_evidence),
            "sorting": unresolved(visual_blocker, visual_evidence),
            "collider": unresolved(visual_blocker, visual_evidence),
        },
    }


def make_capability(row: dict[str, Any]) -> dict[str, Any]:
    building_id = row["buildingId"]
    capability = row["capability"]
    capability_id = f"capability:{capability['id']}"
    base = f"buildings[buildingId={building_id}]"
    capability_locator = f"{base}.capability"
    ui_locator = f"{base}.ui"
    economy_locator = f"{base}.economyTables"
    popup_class = row["ui"]["layoutRef"]

    parameter_evidence = [capability_ref(capability_locator)]
    confidence = "confirmed"
    if row["economyTables"]["bindingConfidence"] == "correlated-table-family":
        confidence = "strongly-inferred"
        parameter_evidence.append(
            capability_ref(
                economy_locator,
                note="Table names and row counts are exact; their association with this building is a correlated table-family binding.",
            )
        )

    popup_evidence = [
        capability_ref(
            f"popupLayouts.{popup_class}",
            "ui-hierarchy",
            "The serialized popup component and dimensions are exact; the native building-to-popup call-site remains unresolved.",
        )
    ]
    runtime_blocker = (
        "Decode the native building controller call-site and parameter semantics before executing this capability."
    )
    popup_blocker = (
        "Decode the native building ID to popup dispatch before opening this template for the building."
    )
    if building_id == 3:
        popup_template_id = unresolved(
            "RequestPop is a bounty/monster request popup, not the Trading Post material-purchase UI. Decode the native Trading Post popup dispatch; generic BuildingPop purchase sections alone do not prove the template binding.",
            [capability_ref(ui_locator, "metadata-field")],
        )
        readiness_blockers = [
            "conditions.binding",
            "popupBinding",
            "popupTemplateId",
            "runtimeBinding",
        ]
        static_data_ready = False
    else:
        popup_template_id = resolved_from(f"popup-template:{popup_class}", popup_evidence)
        readiness_blockers = ["conditions.binding", "popupBinding", "runtimeBinding"]
        static_data_ready = True
    return {
        "key": capability_id.replace(":", "-"),
        "capabilityId": resolved_from(capability_id, [capability_ref(f"{capability_locator}.id")]),
        "buildingId": resolved_from(f"build_{building_id}", [capability_ref(f"{base}.buildingId")]),
        "kind": resolved_from(capability["id"], [capability_ref(f"{capability_locator}.id")]),
        "parameters": resolved_from(
            {
                "description": {"en": capability["descriptionEn"], "vi": capability["descriptionVi"]},
                "upgradeEffect": {"en": capability["upgradeEffectEn"]},
                "levelLabels": {"en": capability["levelLabelsEn"]},
                "economyTables": row["economyTables"]["tables"],
            },
            parameter_evidence,
            confidence,
        ),
        "popupTemplateId": popup_template_id,
        "popupBinding": unresolved_binding(popup_blocker, [capability_ref(ui_locator, "metadata-field")]),
        "runtimeBinding": unresolved_binding(runtime_blocker, [capability_ref(capability_locator)]),
        "conditions": empty_unresolved_collection(
            "Decode the native capability availability checks and exact progression evaluator.",
            [capability_ref(f"{base}.progression")],
        ),
        "readiness": {
            "staticDataReady": static_data_ready,
            "runnable": False,
            "blockingPaths": readiness_blockers,
            "reason": "Identity, description, progression labels, economy table evidence, and popup template are available; native dispatch and execution semantics are unresolved.",
        },
    }


def make_item(item_id: str, locator: str) -> dict[str, Any]:
    blocker = "Decode the authoritative item table row for this referenced legacy item ID."
    return {
        "key": item_id.replace(":", "-"),
        "itemId": resolved(item_id, locator),
        "internalName": unresolved(blocker),
        "displayName": unresolved(blocker),
        "itemType": unresolved(blocker),
        "stackLimit": unresolved(blocker),
        "buyPrice": None,
        "sellPrice": None,
        "directionalEconomy": None,
    }


def economy_item(
    item_id: str,
    item_type: str,
    localized: dict[str, Any],
    locator: str,
    localization_field: str = "localized",
    directional_economy: dict[str, Any] | None = None,
) -> dict[str, Any]:
    display_names = {
        locale: text["title"] if isinstance(text, dict) else text
        for locale, text in sorted(localized.items())
    }
    return {
        "key": item_id.replace(":", "-"),
        "itemId": resolved_from(item_id, [economy_ref(f"{locator}.index")]),
        "internalName": unresolved("The embedded economy row has localized names but no separate stable internal item name."),
        "displayName": resolved_from(display_names, [economy_ref(f"{locator}.{localization_field}")]),
        "itemType": resolved_from(item_type, [economy_ref(locator)]),
        "stackLimit": unresolved("Decode the authoritative inventory stack-limit consumer for this economy item type."),
        "buyPrice": None,
        "sellPrice": None,
        "directionalEconomy": directional_economy,
    }


GEAR_TABLES = {
    "gearWeapons": ("weapon", 10),
    "gearArmor": ("armor", 10),
    "gearHelmet": ("helmet", 10),
    "gearGloves": ("gloves", 10),
    "gearBoots": ("boots", 10),
    "gearBelt": ("belt", 21),
    "gearNecklace": ("necklace", 21),
    "gearRing": ("ring", 21),
}


def economy_amount(key: str, item_id: str, quantity: int, locator: str) -> dict[str, Any]:
    return {
        "key": key,
        "itemId": resolved_from(item_id, [economy_ref(locator)]),
        "quantity": resolved_from(quantity, [economy_ref(locator)]),
    }


def recipe_product(
    product_id: str,
    output_item_id: str,
    building_id: int,
    inputs: list[dict[str, Any]],
    locator: str,
) -> dict[str, Any]:
    building_evidence = [
        economy_ref(locator, "The source economy table identifies the crafted item family."),
        ref(
            f"buildings[index={building_id}].localized.en.description",
            note="The building description explicitly names the same crafted item family.",
        ),
    ]
    return {
        "key": product_id.replace(":", "-"),
        "productId": resolved_from(product_id, [economy_ref(locator)]),
        "buildingId": resolved_from(f"build_{building_id}", building_evidence, "strongly-inferred"),
        "inputs": {"binding": resolved_from_binding([economy_ref(locator)]), "rows": inputs},
        "outputs": {
            "binding": resolved_from_binding([economy_ref(locator)]),
            "rows": [economy_amount("output", output_item_id, 1, locator)],
        },
        "durationMs": unresolved("Decode the native crafting timer for this exact recipe family."),
        "salePrice": None,
        "conditions": empty_unresolved_collection(
            "Decode the exact building-level and progression gates for this recipe variant."
        ),
        "conversionOptions": None,
        "randomOutput": None,
    }


def resolved_from_binding(evidence: list[dict[str, str]], confidence: str = "confirmed") -> dict[str, Any]:
    return {
        "state": "resolved",
        "confidence": confidence,
        "evidence": evidence,
        "requiredEvidence": None,
    }


def make_economy_catalogs(economy: dict[str, Any]) -> tuple[list[dict[str, Any]], list[dict[str, Any]], dict[int, list[tuple[str, str]]]]:
    items: list[dict[str, Any]] = []
    products: list[dict[str, Any]] = []
    product_ids: dict[int, list[tuple[str, str]]] = {}

    for row in sorted(economy["materials"], key=lambda entry: entry["index"]):
        locator = f"materials[index={row['index']}]"
        direction_note = (
            "The material table price is displayed in the Trading Post request flow where the town pays a returning hunter per unit."
        )
        directional = {
            "binding": resolved_from_binding([economy_ref(f"{locator}.price", direction_note)], "strongly-inferred"),
            "townPaysHunterGoldPerUnit": resolved_from(
                row["price"], [economy_ref(f"{locator}.price", direction_note)], "strongly-inferred"
            ),
            "hunterPaysTownGoldByTier": None,
        }
        items.append(
            economy_item(
                f"material:{row['index']}",
                "material",
                row["localizedNames"],
                locator,
                "localizedNames",
                directional,
            )
        )

    for table, (item_type, building_id) in GEAR_TABLES.items():
        for row in sorted(economy[table], key=lambda entry: entry["index"]):
            locator = f"{table}[index={row['index']}]"
            item_id = f"gear:{item_type}:{row['index']}"
            direction_note = (
                "buyMoneyByRating is the gold paid by a hunter to the town shop for the selected gear rating."
            )
            directional = {
                "binding": resolved_from_binding(
                    [economy_ref(f"{locator}.buyMoneyByRating", direction_note)], "strongly-inferred"
                ),
                "townPaysHunterGoldPerUnit": None,
                "hunterPaysTownGoldByTier": resolved_from(
                    row["buyMoneyByRating"],
                    [economy_ref(f"{locator}.buyMoneyByRating", direction_note)],
                    "strongly-inferred",
                ),
            }
            items.append(economy_item(item_id, item_type, row["localized"], locator, directional_economy=directional))
            for rating, materials in enumerate(row["craftingMaterialsByRating"]):
                recipe_id = f"recipe:{item_type}:{row['index']}:rating:{rating}"
                recipe_locator = f"{locator}.craftingMaterialsByRating[{rating}]"
                inputs = [
                    economy_amount(
                        f"input-{index}",
                        f"material:{material_id}",
                        quantity,
                        f"{recipe_locator}[{index}]",
                    )
                    for index, (material_id, quantity) in enumerate(zip(materials["ids"], materials["quantities"], strict=True))
                    if material_id >= 0 and quantity > 0
                ]
                products.append(
                    recipe_product(
                        recipe_id,
                        item_id,
                        building_id,
                        inputs,
                        recipe_locator,
                    )
                )
                product_ids.setdefault(building_id, []).append((recipe_id, recipe_locator))

    for row in sorted(economy["consumables"], key=lambda entry: entry["index"]):
        locator = f"consumables[index={row['index']}]"
        item_id = f"consumable:{row['index']}"
        direction_note = (
            "priceByLevel is the gold paid by a hunter to the town Potion Shop for the selected consumable level."
        )
        directional = {
            "binding": resolved_from_binding(
                [economy_ref(f"{locator}.priceByLevel", direction_note)], "strongly-inferred"
            ),
            "townPaysHunterGoldPerUnit": None,
            "hunterPaysTownGoldByTier": resolved_from(
                row["priceByLevel"],
                [economy_ref(f"{locator}.priceByLevel", direction_note)],
                "strongly-inferred",
            ),
        }
        items.append(
            economy_item(
                item_id,
                "consumable",
                row["localized"],
                locator,
                directional_economy=directional,
            )
        )
        for level, materials in enumerate(row["craftingMaterialsByLevel"]):
            recipe_id = f"recipe:consumable:{row['index']}:level:{level}"
            recipe_locator = f"{locator}.craftingMaterialsByLevel[{level}]"
            inputs = [
                economy_amount(f"input-{index}", f"material:{material_id}", quantity, f"{recipe_locator}[{index}]")
                for index, (material_id, quantity) in enumerate(zip(materials["ids"], materials["quantities"], strict=True))
                if material_id >= 0 and quantity > 0
            ]
            products.append(
                recipe_product(
                    recipe_id,
                    item_id,
                    14,
                    inputs,
                    recipe_locator,
                )
            )
            product_ids.setdefault(14, []).append((recipe_id, recipe_locator))

    for row in sorted(economy["runes"], key=lambda entry: entry["index"]):
        locator = f"runes[index={row['index']}]"
        items.append(economy_item(f"rune:{row['index']}", "rune", row["localized"], locator))

    for row in sorted(economy["runeCraft"], key=lambda entry: entry["index"]):
        locator = f"runeCraft[index={row['index']}]"
        product_id = f"recipe:rune-random:{row['index']}"
        products.append(
            {
                "key": product_id.replace(":", "-"),
                "productId": resolved_from(product_id, [economy_ref(locator)]),
                "buildingId": unresolved(
                    "Decode the native RuneCraft popup/controller dispatch to bind this recipe to an exact building ID.",
                    [economy_ref(locator)],
                ),
                "inputs": {
                    "binding": resolved_from_binding(
                        [economy_ref(f"{locator}.price", "RuneCraft.price is the Rune Powder input count.")],
                        "strongly-inferred",
                    ),
                    "rows": [
                        economy_amount("rune-powder", "material:189", row["price"], f"{locator}.price")
                    ],
                },
                "outputs": None,
                "durationMs": unresolved("Decode the native RuneCraft completion timer."),
                "salePrice": None,
                "conditions": empty_unresolved_collection(
                    "Decode the exact progression gate for this RuneCraft row."
                ),
                "conversionOptions": None,
                "randomOutput": {
                    "binding": resolved_from_binding(
                        [economy_ref(locator, "Localized contents identify one random rune at this exact grade.")]
                    ),
                    "itemType": resolved_from("rune", [economy_ref(f"{locator}.localized")]),
                    "grade": resolved_from(row["grade"], [economy_ref(f"{locator}.grade")]),
                    "quantity": resolved_from(1, [economy_ref(f"{locator}.localized")]),
                    "rngBinding": unresolved_binding(
                        "Decode the exact rune pool, weighting, and authoritative RNG draw for this grade.",
                        [economy_ref(f"{locator}.grade")],
                    ),
                },
            }
        )

    return items, products, product_ids


def make_product(product: dict[str, Any]) -> dict[str, Any]:
    product_index = product["index"]
    locator = f"products[index={product_index}]"
    conversion_note = (
        "AdminProductData material IDs/counts/completionCounts are parallel alternative stock conversions; "
        "cash and elemental fields add two more alternatives."
    )
    conversion_options = []
    if len(product["completionCounts"]) < len(product["requiredMaterialIds"]):
        raise ValueError(f"completionCounts shorter than material alternatives: {locator}")
    for index, (material_id, quantity) in enumerate(
        zip(product["requiredMaterialIds"], product["requiredMaterialQuantities"], strict=True)
    ):
        completion_count = product["completionCounts"][index]
        if material_id < 0 or quantity <= 0:
            continue
        option_locator = f"{locator}.requiredMaterialIds[{index}]"
        option_evidence = [ref(option_locator, note=conversion_note)]
        conversion_options.append(
            {
                "key": f"material-{index}",
                "inputKind": resolved_from("material", option_evidence, "strongly-inferred"),
                "inputId": resolved_from(f"material:{material_id}", option_evidence),
                "inputQuantity": resolved_from(quantity, option_evidence),
                "outputStockQuantity": resolved_from(
                    completion_count,
                    [ref(f"{locator}.completionCounts[{index}]", note=conversion_note)],
                    "strongly-inferred",
                ),
            }
        )
    conversion_options.extend(
        [
            {
                "key": "gem",
                "inputKind": resolved_from("gem", [ref(f"{locator}.requiredCash", note=conversion_note)], "strongly-inferred"),
                "inputId": resolved_from("currency:gem", [ref(f"{locator}.requiredCash", note=conversion_note)], "strongly-inferred"),
                "inputQuantity": resolved(product["requiredCash"], f"{locator}.requiredCash"),
                "outputStockQuantity": resolved(
                    product["cashCompletionCount"], f"{locator}.cashCompletionCount"
                ),
            },
            {
                "key": "elemental",
                "inputKind": resolved_from(
                    "elemental", [ref(f"{locator}.requiredElemental", note=conversion_note)], "strongly-inferred"
                ),
                "inputId": resolved_from(
                    "currency:elemental",
                    [ref(f"{locator}.requiredElemental", note=conversion_note)],
                    "strongly-inferred",
                ),
                "inputQuantity": resolved(product["requiredElemental"], f"{locator}.requiredElemental"),
                "outputStockQuantity": resolved(
                    product["elementalCompletionCount"], f"{locator}.elementalCompletionCount"
                ),
            },
        ]
    )
    time_note = (
        "AdminProductData float bytes match the public worksheet 'time' column; all localized "
        "descriptions identify the placeholder unit as seconds."
    )
    service_data = {
        "binding": resolved_from_binding(
            [ref(locator), ref("schemaEvidence.productWorksheetColumns", note=time_note)]
        ),
        "sourceType": resolved(product["type"], f"{locator}.type"),
        "requiredLevel": resolved(product["level"], f"{locator}.level"),
        "serviceTimeMs": resolved_from(
            round(product["timeSeconds"] * 1000),
            [ref(f"{locator}.timeSeconds"), ref("schemaEvidence.productTimeUnitEvidence", note=time_note)],
        ),
        "effectValue": resolved(product["firstValue"], f"{locator}.firstValue"),
        "useMoney": resolved(product["useMoney"], f"{locator}.useMoney"),
        "completionCounts": resolved(product["completionCounts"], f"{locator}.completionCounts"),
        "requiredCashCount": resolved(product["requiredCash"], f"{locator}.requiredCash"),
        "cashCompletionCount": resolved(product["cashCompletionCount"], f"{locator}.cashCompletionCount"),
        "requiredElementalCount": resolved(product["requiredElemental"], f"{locator}.requiredElemental"),
        "elementalCompletionCount": resolved(
            product["elementalCompletionCount"], f"{locator}.elementalCompletionCount"
        ),
    }
    return {
        "key": f"product-{product_index}",
        "productId": resolved(f"product:{product_index}", f"{locator}.index"),
        "buildingId": resolved(f"build_{product['buildingId']}", f"{locator}.buildingId"),
        "inputs": None,
        "outputs": None,
        "durationMs": resolved_from(
            round(product["timeSeconds"] * 1000),
            [ref(f"{locator}.timeSeconds"), ref("schemaEvidence.productTimeUnitEvidence", note=time_note)],
        ),
        "salePrice": None,
        "conditions": empty_unresolved_collection(
            "Decode the native interpretation of AdminProductData.level before creating an executable unlock condition.",
            [ref(f"{locator}.level")],
        ),
        "serviceData": service_data,
        "conversionOptions": {
            "binding": resolved_from_binding([ref(locator, note=conversion_note)], "strongly-inferred"),
            "rows": conversion_options,
        },
        "randomOutput": None,
    }


def make_skin(row: dict[str, Any]) -> dict[str, Any]:
    building_id = row["buildingId"]
    skin_id = row["skinId"]
    source_index = row["sourceRowIndex"]
    locator = f"rows[sourceRowIndex={source_index}]"
    costs = []
    if row["requiredGold"] > 0:
        costs.append(
            {
                "key": "gold",
                "itemId": resolved_from("currency:gold", [skin_ref(f"{locator}.requiredGold")]),
                "quantity": resolved_from(row["requiredGold"], [skin_ref(f"{locator}.requiredGold")]),
            }
        )
    for index, (material_id, quantity) in enumerate(
        zip(row["requiredMaterialIds"], row["requiredMaterialQuantities"], strict=True)
    ):
        if material_id < 0 or quantity <= 0:
            continue
        costs.append(
            {
                "key": f"material-{index}",
                "itemId": resolved_from(
                    f"material:{material_id}", [skin_ref(f"{locator}.requiredMaterialIds[{index}]")]
                ),
                "quantity": resolved_from(
                    quantity, [skin_ref(f"{locator}.requiredMaterialQuantities[{index}]")]
                ),
            }
        )

    visual = row["visualBinding"]
    visual_locator = f"{locator}.visualBinding"
    if visual["state"] == "resolved":
        visual_evidence = [
            skin_ref(
                visual_locator,
                "asset-object",
                "Exact table family rule, AnimationClip, AnimatorController, and sprite-frame chain.",
            )
        ]
        visual_binding = {
            "binding": resolved_from_binding(visual_evidence),
            "assetKey": resolved_from(visual["assetKey"], visual_evidence),
            "spritePrefix": resolved_from(visual["spritePrefix"], visual_evidence),
            "animationClipPathId": resolved_from(visual["animationClip"]["pathId"], visual_evidence),
            "animatorControllerPathId": resolved_from(
                visual["animatorController"]["pathId"], visual_evidence
            ),
            "spriteFrames": resolved_from(visual["spriteFrames"], visual_evidence),
        }
    else:
        required = visual["reason"]
        visual_evidence = [skin_ref(visual_locator, "asset-object")]
        visual_binding = {
            "binding": unresolved_binding(required, visual_evidence),
            "assetKey": unresolved(required, visual_evidence),
            "spritePrefix": unresolved(required, visual_evidence),
            "animationClipPathId": unresolved(required, visual_evidence),
            "animatorControllerPathId": unresolved(required, visual_evidence),
            "spriteFrames": unresolved(required, visual_evidence),
        }

    return {
        "key": f"build_{building_id}:skin_{skin_id}",
        "buildingId": resolved_from(f"build_{building_id}", [skin_ref(f"{locator}.buildingId")]),
        "skinId": resolved_from(skin_id, [skin_ref(f"{locator}.skinId")]),
        "family": resolved_from(row["family"], [skin_ref(f"{locator}.family")]),
        "displayName": resolved_from(row["titles"], [skin_ref(f"{locator}.titles")]),
        "costs": {
            "binding": resolved_from_binding([skin_ref(locator)]),
            "rows": costs,
        },
        "requiredLevel": resolved_from(row["requiredLevel"], [skin_ref(f"{locator}.requiredLevel")]),
        "visibility": resolved_from(row["visibility"], [skin_ref(f"{locator}.visibility")]),
        "visualBinding": visual_binding,
    }
def collect_unresolved_paths(value: Any, label: str = "") -> list[str]:
    paths: list[str] = []
    if isinstance(value, list):
        for index, entry in enumerate(value):
            paths.extend(collect_unresolved_paths(entry, f"{label}[{index}]"))
    elif isinstance(value, dict):
        if all(key in value for key in ("state", "confidence", "evidence", "requiredEvidence")):
            if value["state"] == "unresolved":
                paths.append(label)
        else:
            for key, entry in value.items():
                child = f"{label}.{key}" if label else key
                paths.extend(collect_unresolved_paths(entry, child))
    return paths


def generate(
    tables_path: Path,
    ui_path: Path,
    assets_path: Path,
    economy_path: Path,
    capabilities_path: Path,
    conditions_path: Path,
    skins_path: Path,
) -> dict[str, Any]:
    tables = json.loads(tables_path.read_text(encoding="utf-8"))
    assets = json.loads(assets_path.read_text(encoding="utf-8"))
    economy = json.loads(economy_path.read_text(encoding="utf-8"))
    capability_contract = json.loads(capabilities_path.read_text(encoding="utf-8"))
    condition_contract = json.loads(conditions_path.read_text(encoding="utf-8"))
    skin_contract = json.loads(skins_path.read_text(encoding="utf-8"))
    buildings = sorted(tables["buildings"], key=lambda row: row["index"])
    products = sorted(tables["products"], key=lambda row: row["index"])
    economy_items, economy_products, economy_product_ids = make_economy_catalogs(economy)
    items = [make_item("currency:gold", "buildings[*].requiredGold")]
    items.extend(economy_items)
    product_rows = [make_product(product) for product in products]
    for product in economy_products:
        product["serviceData"] = None
    product_rows.extend(economy_products)
    visual_bindings = {
        row["sourceBuildIndex"]: row for row in assets.get("buildingVisualBindings", [])
    }
    capabilities = {row["buildingId"]: row for row in capability_contract["buildings"]}
    condition_rows = {row["key"]: row for row in condition_contract["conditionRows"]}
    evaluator = condition_contract["evaluator"]
    capability_rows = [make_capability(row) for row in capability_contract["buildings"]]

    registry: dict[str, Any] = {
        "schemaVersion": 1,
        "contractType": "building-registry",
        "registryId": "evil-hunter-1.411.buildings-v1",
        "legacy": {"game": "Evil Hunter Tycoon", "version": "1.411", "package": "com.superplanet.evilhunter"},
        "runtimeState": "blocked",
        "evidencePolicy": {
            "semanticFields": "evidence-required-per-field",
            "unresolvedValues": "fail-closed-null-or-empty",
            "visualBinding": "separate-from-gameplay-semantics",
        },
        "evidenceSources": [
            evidence_source(SERIALIZED_SOURCE_ID, tables_path),
            evidence_source(UI_SOURCE_ID, ui_path),
            evidence_source(ASSET_SOURCE_ID, assets_path),
            evidence_source(ECONOMY_SOURCE_ID, economy_path),
            evidence_source(CAPABILITY_SOURCE_ID, capabilities_path),
            evidence_source(CONDITION_SOURCE_ID, conditions_path),
            evidence_source(SKIN_SOURCE_ID, skins_path),
        ],
        "catalogs": {
            "items": {
                "binding": resolved_from_binding(
                    [ref("buildings/products.requiredMaterialIds"), economy_ref("materials/gear/consumables/runes")]
                ),
                "rows": items,
            },
            "products": {
                "binding": resolved_from_binding([ref("products"), economy_ref("gear/consumables/runeCraft")]),
                "rows": product_rows,
            },
            "capabilities": {
                "binding": resolved_from_binding([capability_ref("buildings[*].capability")]),
                "rows": capability_rows,
            },
            "skins": {
                "binding": resolved_from_binding([skin_ref("rows")]),
                "rows": [make_skin(row) for row in skin_contract["rows"]],
            },
        },
        "buildings": {
            "binding": resolved_binding("buildings"),
            "rows": [
                make_building(
                    row,
                    products,
                    economy_product_ids,
                    visual_bindings,
                    capabilities,
                    condition_rows,
                    evaluator,
                )
                for row in buildings
            ],
        },
        "releaseGate": {"runnable": False, "blockingPaths": [], "reason": ""},
    }
    blockers = sorted(set(collect_unresolved_paths(registry["catalogs"], "catalogs") + collect_unresolved_paths(registry["buildings"], "buildings")))
    registry["releaseGate"] = {
        "runnable": False,
        "blockingPaths": blockers,
        "reason": "Registry is evidence-complete for recovered identities, localization, build rows, level costs, and product rows, but native capability, trade, item, timer, and visual bindings remain unresolved.",
    }
    return registry


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tables", type=Path, default=DEFAULT_TABLES)
    parser.add_argument("--ui", type=Path, default=DEFAULT_UI)
    parser.add_argument("--assets", type=Path, default=DEFAULT_ASSETS)
    parser.add_argument("--economy", type=Path, default=DEFAULT_ECONOMY)
    parser.add_argument("--capabilities", type=Path, default=DEFAULT_CAPABILITIES)
    parser.add_argument("--conditions", type=Path, default=DEFAULT_CONDITIONS)
    parser.add_argument("--skins", type=Path, default=DEFAULT_SKINS)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    registry = generate(
        args.tables.resolve(),
        args.ui.resolve(),
        args.assets.resolve(),
        args.economy.resolve(),
        args.capabilities.resolve(),
        args.conditions.resolve(),
        args.skins.resolve(),
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(registry, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Generated {display_path(args.output)}: buildings={len(registry['buildings']['rows'])}, products={len(registry['catalogs']['products']['rows'])}")


if __name__ == "__main__":
    main()
