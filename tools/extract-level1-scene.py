#!/usr/bin/env python3
"""Compile versioned, evidence-only scene metadata from Unity's level1 asset."""

import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path

import UnityPy

from scene_evidence_lib import color, mono_behaviour_header, quaternion, vector2, vector3


UI_SCRIPTS = {"Image", "Button", "Text", "CanvasScaler", "GraphicRaycaster"}
SUPPORTED_COMPONENTS = {
    "Transform", "RectTransform", "SpriteRenderer", "Canvas", "Camera", "Animator",
    "BoxCollider2D", "CircleCollider2D", "TextMesh", "CanvasGroup", "MonoBehaviour",
}


def pptr(value, external_names, objects, inventory_by_source_id):
    file_id = int(getattr(value, "file_id", getattr(value, "m_FileID", 0)) or 0)
    path_id = int(getattr(value, "path_id", getattr(value, "m_PathID", 0)) or 0)
    reference = {"fileId": file_id, "pathId": path_id}
    if path_id == 0:
        return reference
    if file_id == 0:
        reader = objects.get(path_id)
        if reader:
            reference["type"] = reader.type.name
        return reference
    source = external_names.get(file_id)
    if source:
        reference["source"] = source
        evidence = inventory_by_source_id.get((source, path_id))
        if evidence:
            reference.update({"type": evidence["type"], "name": evidence.get("name", "")})
    return reference


def diagnostic(reader, phase, error):
    return {
        "componentType": reader.type.name,
        "pathId": reader.path_id,
        "phase": phase,
        "errorType": type(error).__name__,
        "message": str(error),
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("asset", nargs="?", default="game-assets/extracted/joined_unity_files/level1")
    parser.add_argument("output", nargs="?", default="reverse-engineering/evidence/level1-scene-evidence-v2.json")
    parser.add_argument("--inventory", default="game-assets/extracted/exported/metadata/inventory.json")
    args = parser.parse_args()

    asset_path = Path(args.asset)
    environment = UnityPy.load(str(asset_path))
    serialized_file = next(file for file in environment.files.values() if hasattr(file, "objects"))
    objects = {reader.path_id: reader for reader in environment.objects}
    object_counts = Counter(reader.type.name for reader in environment.objects)
    external_names = {index: external.name for index, external in enumerate(serialized_file.externals, 1)}
    inventory = json.loads(Path(args.inventory).read_text(encoding="utf-8"))
    inventory_by_source_id = {(entry["source"], int(entry["path_id"])): entry for entry in inventory}
    script_names = {
        int(entry["path_id"]): entry.get("name", "")
        for entry in inventory if entry["source"] == "globalgamemanagers.assets" and entry["type"] == "MonoScript"
    }

    diagnostics = []
    game_objects = {}
    components = {key: [] for key in [
        "transforms", "spriteRenderers", "canvases", "cameras", "animators", "colliders",
        "uiBehaviours", "textMeshes", "canvasGroups",
    ]}

    for reader in environment.objects:
        if reader.type.name != "GameObject":
            continue
        try:
            data = reader.read()
            game_objects[reader.path_id] = {
                "pathId": reader.path_id,
                "name": data.m_Name,
                "active": bool(data.m_IsActive),
                "layer": int(data.m_Layer),
                "components": [{
                    "pathId": int(pair.component.path_id),
                    "type": objects[int(pair.component.path_id)].type.name if int(pair.component.path_id) in objects else "External",
                } for pair in data.m_Component],
            }
        except Exception as error:
            diagnostics.append(diagnostic(reader, "read-game-object", error))
            game_objects[reader.path_id] = {"pathId": reader.path_id, "readError": True}

    for reader in environment.objects:
        component_type = reader.type.name
        if component_type not in SUPPORTED_COMPONENTS:
            continue
        if component_type == "MonoBehaviour":
            try:
                header = mono_behaviour_header(reader.get_raw_data())
                script_name = script_names.get(header["scriptPathId"])
                if header["scriptFileId"] != 1 or script_name not in UI_SCRIPTS:
                    continue
                components["uiBehaviours"].append({
                    "pathId": reader.path_id,
                    "gameObjectPathId": header["gameObjectPathId"],
                    "enabled": header["enabled"],
                    "script": {"fileId": header["scriptFileId"], "pathId": header["scriptPathId"], "source": "globalgamemanagers.assets", "name": script_name},
                    "payloadResolution": "header-only",
                })
                diagnostics.append({
                    "componentType": script_name,
                    "pathId": reader.path_id,
                    "phase": "decode-ui-payload",
                    "errorType": "UnsupportedSerializedPayload",
                    "message": "MonoBehaviour header resolved; Image/Button/Text payload fields require matching external type trees.",
                })
            except Exception as error:
                diagnostics.append(diagnostic(reader, "read-mono-behaviour-header", error))
            continue

        try:
            data = reader.read()
            game_object_id = int(data.m_GameObject.path_id)
            base = {"pathId": reader.path_id, "gameObjectPathId": game_object_id, "enabled": bool(getattr(data, "m_Enabled", True))}
            if component_type in ("Transform", "RectTransform"):
                parent = pptr(data.m_Father, external_names, objects, inventory_by_source_id)
                record = {**base, "componentType": component_type, "parent": parent,
                          "localPosition": vector3(data.m_LocalPosition), "localRotation": quaternion(data.m_LocalRotation),
                          "localScale": vector3(data.m_LocalScale), "children": [pptr(child, external_names, objects, inventory_by_source_id) for child in data.m_Children]}
                if component_type == "RectTransform":
                    record.update({"anchorMin": vector2(data.m_AnchorMin), "anchorMax": vector2(data.m_AnchorMax),
                                   "anchoredPosition": vector2(data.m_AnchoredPosition), "sizeDelta": vector2(data.m_SizeDelta), "pivot": vector2(data.m_Pivot)})
                components["transforms"].append(record)
            elif component_type == "SpriteRenderer":
                components["spriteRenderers"].append({**base, "sprite": pptr(data.m_Sprite, external_names, objects, inventory_by_source_id),
                    "color": color(data.m_Color), "sortingLayerId": int(data.m_SortingLayerID), "sortingOrder": int(data.m_SortingOrder),
                    "flipX": bool(data.m_FlipX), "flipY": bool(data.m_FlipY), "drawMode": int(data.m_DrawMode),
                    "materials": [pptr(item, external_names, objects, inventory_by_source_id) for item in data.m_Materials]})
            elif component_type == "Canvas":
                components["canvases"].append({**base, "renderMode": int(data.m_RenderMode), "sortingLayerId": int(data.m_SortingLayerID),
                    "sortingOrder": int(data.m_SortingOrder), "overrideSorting": bool(data.m_OverrideSorting), "pixelPerfect": bool(data.m_PixelPerfect),
                    "planeDistance": float(data.m_PlaneDistance), "camera": pptr(data.m_Camera, external_names, objects, inventory_by_source_id)})
            elif component_type == "Camera":
                components["cameras"].append({**base, "clearFlags": int(data.m_ClearFlags), "backgroundColor": color(data.m_BackGroundColor),
                    "cullingMask": int(data.m_CullingMask.m_Bits), "depth": float(data.m_Depth), "viewport": {"x": float(data.m_NormalizedViewPortRect.x), "y": float(data.m_NormalizedViewPortRect.y), "width": float(data.m_NormalizedViewPortRect.width), "height": float(data.m_NormalizedViewPortRect.height)},
                    "targetDisplay": int(data.m_TargetDisplay), "targetTexture": pptr(data.m_TargetTexture, external_names, objects, inventory_by_source_id)})
            elif component_type == "Animator":
                components["animators"].append({**base, "controller": pptr(data.m_Controller, external_names, objects, inventory_by_source_id),
                    "avatar": pptr(data.m_Avatar, external_names, objects, inventory_by_source_id), "applyRootMotion": bool(data.m_ApplyRootMotion),
                    "updateMode": int(data.m_UpdateMode), "cullingMode": int(data.m_CullingMode)})
            elif component_type in ("BoxCollider2D", "CircleCollider2D"):
                record = {**base, "colliderType": component_type, "offset": vector2(data.m_Offset), "isTrigger": bool(data.m_IsTrigger),
                          "density": float(data.m_Density), "material": pptr(data.m_Material, external_names, objects, inventory_by_source_id)}
                if component_type == "BoxCollider2D": record.update({"size": vector2(data.m_Size), "edgeRadius": float(data.m_EdgeRadius)})
                else: record["radius"] = float(data.m_Radius)
                components["colliders"].append(record)
            elif component_type == "TextMesh":
                components["textMeshes"].append({**base, "text": data.m_Text, "font": pptr(data.m_Font, external_names, objects, inventory_by_source_id),
                    "fontSize": int(data.m_FontSize), "fontStyle": int(data.m_FontStyle), "color": color(data.m_Color), "anchor": int(data.m_Anchor), "alignment": int(data.m_Alignment)})
            elif component_type == "CanvasGroup":
                components["canvasGroups"].append({**base, "alpha": float(data.m_Alpha), "interactable": bool(data.m_Interactable),
                    "blocksRaycasts": bool(data.m_BlocksRaycasts), "ignoreParentGroups": bool(data.m_IgnoreParentGroups)})
        except Exception as error:
            diagnostics.append(diagnostic(reader, "decode-component", error))

    component_coverage = {name: len(records) for name, records in components.items()}
    component_coverage["uiPayloadsResolved"] = 0
    output = {
        "schemaVersion": 2,
        "manifestType": "unity-scene-evidence",
        "runtimeCompatibility": "not-claimed",
        "source": {"path": str(asset_path), "bytes": asset_path.stat().st_size, "sha256": hashlib.sha256(asset_path.read_bytes()).hexdigest(), "unityPyVersion": UnityPy.__version__},
        "externalFiles": [{"fileId": file_id, "name": name} for file_id, name in external_names.items()],
        "objectCounts": dict(sorted(object_counts.items())),
        "coverage": {"gameObjects": len(game_objects), **component_coverage, "diagnostics": len(diagnostics)},
        "gaps": ["UI Image/Button/Text payloads are header-only until external type trees are supplied.", "MonoBehaviour gameplay fields are not decoded.", "Animator controller graphs and animation clips are references only.", "Scene evidence does not establish runtime-mutated state, dynamic spawns, navigation, or gameplay rules."],
        "gameObjects": list(game_objects.values()),
        "components": components,
        "diagnostics": diagnostics,
    }
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Compiled scene evidence: {len(game_objects)} GameObjects, {sum(component_coverage.values())} component records, {len(diagnostics)} diagnostics -> {output_path}")


if __name__ == "__main__":
    main()
