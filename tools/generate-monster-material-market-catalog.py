#!/usr/bin/env python3
"""Normalize monster materials, Trading Post prices, and material consumers."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MONSTERS = ROOT / "packages/content/releases/evil-hunter-1.411/monster-runtime-catalog.json"
DEFAULT_ECONOMY = ROOT / "reverse-engineering/evidence/core-economy-tables-v1.json"
DEFAULT_BUILDINGS = ROOT / "packages/content/releases/evil-hunter-1.411/building-registry.json"
DEFAULT_OUTPUT = ROOT / "packages/content/releases/evil-hunter-1.411/monster-material-market-catalog.json"


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def source(path: Path) -> dict[str, Any]:
    payload = path.read_bytes()
    return {
        "path": path.resolve().relative_to(ROOT).as_posix(),
        "bytes": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }


def resolved(field: Any) -> Any | None:
    if not isinstance(field, dict) or field.get("state") != "resolved":
        return None
    return field.get("value")


def material_index(item_id: str | None) -> int | None:
    if not isinstance(item_id, str) or not item_id.startswith("material:"):
        return None
    try:
        return int(item_id.removeprefix("material:"))
    except ValueError:
        return None


def generate(monsters_path: Path, economy_path: Path, buildings_path: Path) -> dict[str, Any]:
    monsters = load_json(monsters_path)
    economy = load_json(economy_path)
    buildings = load_json(buildings_path)

    material_definitions = {row["index"]: row for row in economy["materials"]}
    market_items: dict[int, dict[str, Any]] = {}
    for row in buildings["catalogs"]["items"]["rows"]:
        index = material_index(resolved(row.get("itemId")))
        if index is None:
            continue
        directional = row.get("directionalEconomy") or {}
        market_items[index] = {
            "unitPriceGold": resolved(directional.get("townPaysHunterGoldPerUnit")),
            "bindingConfidence": (directional.get("binding") or {}).get("confidence"),
        }

    drop_slots: list[dict[str, Any]] = []
    droppable_indices: set[int] = set()
    for group in monsters["groups"]:
        key = group["key"]
        for monster in group["monsters"]:
            materials = monster["materials"]
            for slot, index in enumerate(materials["indices"]):
                droppable_indices.add(index)
                drop_slots.append(
                    {
                        "monsterIndex": monster["index"],
                        "area": key["area"],
                        "monsterType": key["type"],
                        "createLevel": key["createLevel"],
                        "slot": slot,
                        "materialId": f"material:{index}",
                        "quantity": materials["counts"][slot],
                        "rawPercent": materials["percentValues"][slot],
                    }
                )

    recipe_inputs: list[dict[str, Any]] = []
    unresolved_recipe_conditions: list[dict[str, Any]] = []
    for product in buildings["catalogs"]["products"]["rows"]:
        product_id = resolved(product.get("productId"))
        building_id = resolved(product.get("buildingId"))
        inputs = product.get("inputs")
        if not product_id or not isinstance(inputs, dict):
            continue
        matched = False
        for slot, input_row in enumerate(inputs.get("rows", [])):
            item_id = resolved(input_row.get("itemId"))
            index = material_index(item_id)
            quantity = resolved(input_row.get("quantity"))
            if index not in droppable_indices or quantity is None:
                continue
            matched = True
            recipe_inputs.append(
                {
                    "productId": product_id,
                    "buildingId": building_id,
                    "slot": slot,
                    "materialId": item_id,
                    "quantity": quantity,
                }
            )
        conditions = product.get("conditions") or {}
        binding = conditions.get("binding") or {}
        if matched and binding.get("state") != "resolved":
            unresolved_recipe_conditions.append(
                {
                    "productId": product_id,
                    "buildingId": building_id,
                    "requiredEvidence": binding.get("requiredEvidence"),
                }
            )

    building_costs: list[dict[str, Any]] = []
    for building in buildings["buildings"]["rows"]:
        for row_kind in ("buildRows", "levels"):
            collection = building.get(row_kind) or {}
            for row in collection.get("rows", []):
                building_id = resolved(row.get("buildId"))
                level = resolved(row.get("level"))
                for slot, cost in enumerate((row.get("costs") or {}).get("rows", [])):
                    item_id = resolved(cost.get("itemId"))
                    index = material_index(item_id)
                    quantity = resolved(cost.get("quantity"))
                    if index not in droppable_indices or quantity is None:
                        continue
                    building_costs.append(
                        {
                            "rowKind": row_kind,
                            "buildingId": building_id,
                            "level": level,
                            "slot": slot,
                            "materialId": item_id,
                            "quantity": quantity,
                        }
                    )

    materials = []
    for index in sorted(droppable_indices):
        definition = material_definitions.get(index)
        market = market_items.get(index)
        if definition is None:
            raise ValueError(f"monster material {index} has no economy definition")
        materials.append(
            {
                "materialId": f"material:{index}",
                "sourceIndex": index,
                "localizedNames": definition["localizedNames"],
                "price": definition["price"],
                "rating": definition["rating"],
                "level": definition["level"],
                "convert": definition["convert"],
                "compose": definition["compose"],
                "parentIndex": definition["parentIndex"],
                "magic": definition["magic"],
                "tradingPost": {
                    "listed": market is not None and market["unitPriceGold"] is not None,
                    "townPaysHunterGoldPerUnit": None if market is None else market["unitPriceGold"],
                    "bindingConfidence": None if market is None else market["bindingConfidence"],
                },
            }
        )

    unlisted = [row["materialId"] for row in materials if not row["tradingPost"]["listed"]]
    return {
        "schemaVersion": 1,
        "catalogId": "evil-hunter-1.411.monster-material-market-v1",
        "evidencePolicy": {
            "definitionAndPrice": "serialized package rows",
            "dropSlots": "monster catalog source-array order",
            "recipeAndBuildingLinks": "resolved registry fields only",
            "unresolvedLinks": "preserved explicitly; no fallback",
        },
        "sources": [source(monsters_path), source(economy_path), source(buildings_path)],
        "summary": {
            "droppableMaterialCount": len(materials),
            "monsterDropSlotCount": len(drop_slots),
            "recipeMaterialInputCount": len(recipe_inputs),
            "buildingMaterialCostCount": len(building_costs),
            "unresolvedRecipeConditionCount": len(unresolved_recipe_conditions),
            "unlistedDroppableMaterials": unlisted,
        },
        "materials": materials,
        "monsterDropSlots": drop_slots,
        "recipeMaterialInputs": recipe_inputs,
        "buildingMaterialCosts": building_costs,
        "unresolvedRecipeConditions": unresolved_recipe_conditions,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--monsters", type=Path, default=DEFAULT_MONSTERS)
    parser.add_argument("--economy", type=Path, default=DEFAULT_ECONOMY)
    parser.add_argument("--buildings", type=Path, default=DEFAULT_BUILDINGS)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    catalog = generate(args.monsters, args.economy, args.buildings)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
