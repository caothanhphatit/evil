#!/usr/bin/env python3
"""Extract exact serialized town-building geometry without inventing placement.

The level1 scene contains one fixed ReviveBuilding anchor. Other buildings use
the shared Building prefab and runtime-selected build_* controllers, so generic
prefab geometry and per-building sprite geometry are deliberately kept apart.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import UnityPy


ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "game-assets/source/unity-assets/bin/Data"
ASSET_EVIDENCE = ROOT / "reverse-engineering/evidence/building-asset-evidence-v1.json"
SCENE_EVIDENCE = ROOT / "reverse-engineering/evidence/level1-scene-evidence-v2.json"
TABLE_EVIDENCE = ROOT / "reverse-engineering/evidence/serialized-building-tables-v1.json"
OUT = ROOT / "reverse-engineering/evidence/building-town-geometry-v1.json"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def vector2(value) -> dict[str, float]:
    return {"x": float(value.x), "y": float(value.y)}


def rect(value) -> dict[str, float]:
    return {
        "x": float(value.x),
        "y": float(value.y),
        "width": float(value.width),
        "height": float(value.height),
    }


def main() -> None:
    asset_evidence = json.loads(ASSET_EVIDENCE.read_text(encoding="utf-8"))
    scene_evidence = json.loads(SCENE_EVIDENCE.read_text(encoding="utf-8"))
    tables = json.loads(TABLE_EVIDENCE.read_text(encoding="utf-8"))
    table_by_id = {item["index"]: item for item in tables["buildings"]}

    environments: dict[str, UnityPy.Environment] = {}
    buildings = []
    for binding in asset_evidence["buildingVisualBindings"]:
        frame = binding["spriteFrames"][0]
        bundle = frame["sourceBundle"]
        source = DATA / bundle
        environment = environments.setdefault(bundle, UnityPy.load(str(source)))
        sprite_reader = next(
            (
                item
                for item in environment.objects
                if item.path_id == frame["pathId"] and item.type.name == "Sprite"
            ),
            None,
        )
        if sprite_reader is None:
            raise ValueError(f"missing Sprite {bundle}:{frame['pathId']}")
        sprite = sprite_reader.read()
        image = sprite.image
        alpha = image.getchannel("A") if "A" in image.getbands() else None
        opaque_bounds = alpha.getbbox() if alpha is not None else None
        building = table_by_id[binding["sourceBuildIndex"]]
        buildings.append(
            {
                "sourceBuildIndex": binding["sourceBuildIndex"],
                "sourceBuildKey": binding["sourceBuildKey"],
                "serializedGridSize": building["size"],
                "serializedMovable": building["movable"],
                "baseSprite": {
                    "name": sprite.m_Name,
                    "sourceBundle": bundle,
                    "sourceBundleSha256": sha256(source),
                    "pathId": sprite_reader.path_id,
                    "rect": rect(sprite.m_Rect),
                    "pivot": vector2(sprite.m_Pivot),
                    "offset": vector2(sprite.m_Offset),
                    "pixelsToUnits": float(sprite.m_PixelsToUnits),
                    "renderTextureRect": rect(sprite.m_RD.textureRect),
                    "renderTextureRectOffset": vector2(sprite.m_RD.textureRectOffset),
                    "decodedImagePixels": {"width": image.width, "height": image.height},
                    "opaquePixelBounds": (
                        {
                            "left": opaque_bounds[0],
                            "top": opaque_bounds[1],
                            "right": opaque_bounds[2],
                            "bottom": opaque_bounds[3],
                        }
                        if opaque_bounds is not None
                        else None
                    ),
                    "physicsShapeCount": len(sprite.m_PhysicsShape),
                },
                "townPosition": None,
                "sorting": None,
                "collider": None,
                "resolution": "exact-visual-geometry-placement-unresolved",
            }
        )

    generic = asset_evidence["genericBuildingPrefab"]
    output = {
        "schemaVersion": 1,
        "manifestType": "building-town-geometry-evidence",
        "runtimeCompatibility": "evidence-only",
        "source": {
            "buildingAssetEvidence": ASSET_EVIDENCE.relative_to(ROOT).as_posix(),
            "buildingAssetEvidenceSha256": sha256(ASSET_EVIDENCE),
            "sceneEvidence": SCENE_EVIDENCE.relative_to(ROOT).as_posix(),
            "sceneEvidenceSha256": sha256(SCENE_EVIDENCE),
            "serializedBuildingTables": TABLE_EVIDENCE.relative_to(ROOT).as_posix(),
            "serializedBuildingTablesSha256": sha256(TABLE_EVIDENCE),
        },
        "placementFinding": {
            "status": "runtime-or-save-owned",
            "serializedPerBuildingPositions": 0,
            "exactSerializedAnchors": len(asset_evidence["sceneBindings"]),
            "note": (
                "level1 serializes only the ReviveBuilding anchor. No build_* sprite renderer or "
                "per-ID prefab instance is serialized at a town coordinate."
            ),
        },
        "genericBuildingTemplate": {
            "scope": generic["scope"],
            "gameObjectPathId": generic["gameObjectPathId"],
            "controllerClass": generic["controllerClass"],
            "collider": generic["collider"],
            "visualChild": generic["visualChild"],
            "bindingToIndividualBuildIds": None,
        },
        "serializedAnchors": asset_evidence["sceneBindings"],
        "buildings": sorted(buildings, key=lambda item: item["sourceBuildIndex"]),
        "gaps": [
            "Town coordinates for individual build IDs are runtime/save state and are not present in level1.",
            "The shared prefab collider and sorting values cannot be promoted to per-ID bindings without runtime evidence.",
            "Sprite opaque bounds describe source pixels, not navigation or collision geometry.",
        ],
    }
    OUT.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"Wrote {len(buildings)} exact build sprite geometries, "
        f"{len(output['serializedAnchors'])} scene anchor(s) -> {OUT}"
    )


if __name__ == "__main__":
    main()
