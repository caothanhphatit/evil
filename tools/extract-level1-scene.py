#!/usr/bin/env python3
"""Extract a clean-room, read-only scene inventory from Unity's level1 asset."""

import argparse
import json
from collections import Counter
from pathlib import Path

import UnityPy


def path_id(value):
    return int(getattr(value, "path_id", 0) or 0)


def vector(value):
    if value is None:
        return None
    return {"x": float(value.x), "y": float(value.y), "z": float(getattr(value, "z", 0.0))}


def vector2(value):
    if value is None:
        return None
    return {"x": float(value.x), "y": float(value.y)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("asset", nargs="?", default="game-assets/extracted/joined_unity_files/level1")
    parser.add_argument("output", nargs="?", default="reverse-engineering/evidence/level1-scene-hierarchy.json")
    args = parser.parse_args()

    environment = UnityPy.load(args.asset)
    objects = {object_reader.path_id: object_reader for object_reader in environment.objects}
    counts = Counter(object_reader.type.name for object_reader in environment.objects)
    game_objects = {}
    transforms = {}

    for object_reader in environment.objects:
        if object_reader.type.name != "GameObject":
            continue
        try:
            data = object_reader.read()
            game_objects[object_reader.path_id] = {
                "pathId": object_reader.path_id,
                "name": data.m_Name,
                "active": bool(data.m_IsActive),
                "layer": int(data.m_Layer),
                "components": [
                    {
                        "pathId": path_id(pair.component),
                        "type": objects[path_id(pair.component)].type.name if path_id(pair.component) in objects else "External",
                    }
                    for pair in data.m_Component
                ],
            }
        except Exception as error:  # A broken optional component must not hide the scene inventory.
            game_objects[object_reader.path_id] = {"pathId": object_reader.path_id, "readError": str(error)}

    for object_reader in environment.objects:
        if object_reader.type.name not in ("Transform", "RectTransform"):
            continue
        try:
            data = object_reader.read()
            game_object_id = path_id(data.m_GameObject)
            if game_object_id not in game_objects:
                continue
            parent_transform_id = path_id(data.m_Father)
            transforms[game_object_id] = {
                "componentPathId": object_reader.path_id,
                "componentType": object_reader.type.name,
                "parentTransformPathId": parent_transform_id,
                "parentGameObjectPathId": path_id(objects[parent_transform_id].read().m_GameObject) if parent_transform_id in objects else 0,
                "localPosition": vector(data.m_LocalPosition),
                "localScale": vector(data.m_LocalScale),
                "childrenCount": len(getattr(data, "m_Children", [])),
            }
            if object_reader.type.name == "RectTransform":
                transforms[game_object_id].update({
                    "anchorMin": vector2(data.m_AnchorMin),
                    "anchorMax": vector2(data.m_AnchorMax),
                    "anchoredPosition": vector2(data.m_AnchoredPosition),
                    "sizeDelta": vector2(data.m_SizeDelta),
                    "pivot": vector2(data.m_Pivot),
                })
        except Exception:
            continue

    for game_object_id, record in game_objects.items():
        record["transform"] = transforms.get(game_object_id)

    canvas_records = []
    for object_reader in environment.objects:
        if object_reader.type.name != "Canvas":
            continue
        try:
            data = object_reader.read()
            game_object_id = path_id(data.m_GameObject)
            canvas_records.append({
                "pathId": object_reader.path_id,
                "gameObjectPathId": game_object_id,
                "name": game_objects.get(game_object_id, {}).get("name"),
                "renderMode": int(data.m_RenderMode),
                "sortingOrder": int(data.m_SortingOrder),
                "enabled": bool(data.m_Enabled),
            })
        except Exception:
            continue

    output = {
        "schemaVersion": 1,
        "source": str(Path(args.asset)),
        "objectCounts": dict(counts),
        "gameObjectCount": len(game_objects),
        "transformCount": len(transforms),
        "canvasCount": len(canvas_records),
        "canvases": canvas_records,
        "gameObjects": list(game_objects.values()),
    }
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Extracted {len(game_objects)} GameObjects, {len(transforms)} transforms and {len(canvas_records)} canvases to {output_path}")


if __name__ == "__main__":
    main()
