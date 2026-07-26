#!/usr/bin/env python3
"""Extract source-confirmed town building asset and scene evidence.

This intentionally keeps asset identity separate from town placement. The
serialized level1 scene contains only the ReviveBuilding anchor; other build
skins are selected by runtime data and therefore have no recovered position.
"""

from __future__ import annotations

import csv
import hashlib
import json
import re
from pathlib import Path

import UnityPy

from scene_evidence_lib import mono_behaviour_header


ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "game-assets/source/unity-assets/bin/Data"
JOINED = ROOT / "game-assets/extracted/joined_unity_files/sharedassets1.assets"
SCENE = ROOT / "reverse-engineering/evidence/level1-scene-evidence-v2.json"
TABLES = ROOT / "reverse-engineering/evidence/serialized-building-tables-v1.json"
MONOSCRIPTS = ROOT / "reverse-engineering/evidence/monoscripts.csv"
OUT = ROOT / "reverse-engineering/evidence/building-asset-evidence-v1.json"

BASE_BUILD_ASSET = re.compile(r"^build_(\d+)$")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_script_names() -> dict[int, str]:
    with MONOSCRIPTS.open(newline="") as handle:
        return {int(row["path_id"]): row["class"] for row in csv.DictReader(handle)}


def main() -> None:
    scene = json.loads(SCENE.read_text())
    tables = json.loads(TABLES.read_text())
    scene_gos = {item["pathId"]: item for item in scene["gameObjects"]}
    transforms = {
        item["gameObjectPathId"]: item for item in scene["components"]["transforms"]
    }
    renderers = {
        item["gameObjectPathId"]: item
        for item in scene["components"]["spriteRenderers"]
    }
    scene_bindings = []
    for path_id, game_object in scene_gos.items():
        if game_object["name"] != "ReviveBuilding":
            continue
        transform = transforms.get(path_id)
        scene_bindings.append(
            {
                "gameObjectPath": "BuildGroup/ReviveBuilding[260]",
                "gameObjectPathId": path_id,
                "active": game_object["active"],
                "transform": transform,
                "spriteRenderer": renderers.get(path_id),
                "positionConfidence": "confirmed-anchor-no-sprite",
                "note": "Serialized anchor; its visual building skin is runtime-selected.",
            }
        )

    env = UnityPy.load(str(JOINED))
    serialized = next(iter(env.files.values()))
    externals = {index: item.path for index, item in enumerate(serialized.externals, 1)}
    objects = {item.path_id: item for item in env.objects}
    script_names = load_script_names()

    generic_prefab = None
    for item in env.objects:
        if item.type.name != "GameObject":
            continue
        game_object = item.read()
        if game_object.m_Name != "Building":
            continue
        component_readers = [objects[pair.component.path_id] for pair in game_object.m_Component]
        collider_reader = next((reader for reader in component_readers if reader.type.name == "CapsuleCollider2D"), None)
        mono_reader = next((reader for reader in component_readers if reader.type.name == "MonoBehaviour"), None)
        if collider_reader is None or mono_reader is None:
            continue
        header = mono_behaviour_header(mono_reader.get_raw_data())
        collider = collider_reader.read()
        transform = next(reader.read() for reader in component_readers if reader.type.name == "Transform")
        visual_child = None
        for child_pointer in transform.m_Children:
            child_transform = objects.get(child_pointer.path_id)
            if child_transform is None:
                continue
            child_go = objects.get(child_transform.read().m_GameObject.path_id)
            if child_go is None:
                continue
            child_data = child_go.read()
            child_components = [objects[pair.component.path_id] for pair in child_data.m_Component]
            renderer_reader = next((reader for reader in child_components if reader.type.name == "SpriteRenderer"), None)
            animator_reader = next((reader for reader in child_components if reader.type.name == "Animator"), None)
            if renderer_reader is None or animator_reader is None:
                continue
            renderer = renderer_reader.read()
            child_transform_data = child_transform.read()
            visual_child = {
                "gameObjectPathId": child_go.path_id,
                "name": child_data.m_Name,
                "localPosition": {
                    "x": child_transform_data.m_LocalPosition.x,
                    "y": child_transform_data.m_LocalPosition.y,
                    "z": child_transform_data.m_LocalPosition.z,
                },
                "spriteRendererPathId": renderer_reader.path_id,
                "sortingLayer": renderer.m_SortingLayer,
                "sortingLayerId": renderer.m_SortingLayerID,
                "sortingOrder": renderer.m_SortingOrder,
                "animatorPathId": animator_reader.path_id,
                "defaultAnimatorController": None,
            }
            break
        generic_prefab = {
            "scope": "generic-template-not-bound-to-individual-build-ids",
            "gameObjectPathId": item.path_id,
            "name": game_object.m_Name,
            "controllerClass": script_names.get(header["scriptPathId"]),
            "controllerScriptPathId": header["scriptPathId"],
            "controllerComponentPathId": mono_reader.path_id,
            "collider": {
                "type": "CapsuleCollider2D",
                "pathId": collider_reader.path_id,
                "enabled": bool(collider.m_Enabled),
                "isTrigger": bool(collider.m_IsTrigger),
                "direction": collider.m_Direction,
                "offset": {"x": collider.m_Offset.x, "y": collider.m_Offset.y},
                "size": {"x": collider.m_Size.x, "y": collider.m_Size.y},
            },
            "visualChild": visual_child,
        }
        break
    clips = []
    clips_by_name = {}
    for item in env.objects:
        if item.type.name != "AnimationClip":
            continue
        clip = item.read()
        name = getattr(clip, "m_Name", "")
        if not name.startswith("build_") or name.startswith("buildSkin"):
            continue
        sprites = []
        for pointer in clip.m_ClipBindingConstant.pptrCurveMapping:
            bundle = externals.get(pointer.file_id)
            if not bundle:
                continue
            source = DATA / bundle
            if not source.exists():
                continue
            external = UnityPy.load(str(source))
            sprite = next((obj for obj in external.objects if obj.path_id == pointer.path_id), None)
            if not sprite or sprite.type.name != "Sprite":
                continue
            sprite_data = sprite.read()
            sprites.append(
                {
                    "sourceBundle": bundle,
                    "sourceSha256": sha256(source),
                    "pathId": pointer.path_id,
                    "name": sprite_data.m_Name,
                }
            )
        record = {
            "animationClip": name,
            "animationClipPathId": item.path_id,
            "assetClass": "build-run-effect" if name.startswith("build_run_") else "building-skin",
            "spriteFrames": sprites,
            "position": None,
            "positionConfidence": "unresolved-runtime-placement",
        }
        clips.append(record)
        clips_by_name[name] = record

    controllers_by_name = {}
    for item in env.objects:
        if item.type.name != "AnimatorController":
            continue
        controller = item.read()
        match = BASE_BUILD_ASSET.fullmatch(getattr(controller, "m_Name", ""))
        if not match:
            continue
        controllers_by_name[controller.m_Name] = {
            "name": controller.m_Name,
            "pathId": item.path_id,
            "animationClipPathIds": [pointer.path_id for pointer in controller.m_AnimationClips],
        }

    # AdminBuildData.index and the base animation/controller use the same exact
    # `build_<decimal index>` source key. Only emit a binding when all three
    # serialized records agree; skin variants and run effects are excluded.
    visual_bindings = []
    for building in sorted(tables["buildings"], key=lambda value: value["index"]):
        source_index = building["index"]
        source_key = f"build_{source_index}"
        clip = clips_by_name.get(source_key)
        controller = controllers_by_name.get(source_key)
        if clip is None or controller is None:
            continue
        if controller["animationClipPathIds"] != [clip["animationClipPathId"]]:
            continue
        visual_bindings.append(
            {
                "sourceBuildIndex": source_index,
                "sourceBuildKey": source_key,
                "bindingConfidence": "confirmed-exact-serialized-key-join",
                "animationClip": {
                    "name": clip["animationClip"],
                    "pathId": clip["animationClipPathId"],
                },
                "animatorController": controller,
                "spriteFrames": clip["spriteFrames"],
                "controllerClass": None,
                "popupClass": None,
                "townPosition": None,
                "sorting": None,
                "collider": None,
                "unresolvedReason": (
                    "The exact base visual assets are joined, but no per-building prefab/runtime record "
                    "binds this source index to a controller class, popup, placement, sorting, or collider."
                ),
            }
        )

    output = {
        "schemaVersion": 1,
        "source": {
            "sceneEvidence": SCENE.relative_to(ROOT).as_posix(),
            "serializedBuildingTables": TABLES.relative_to(ROOT).as_posix(),
            "serializedBuildingTablesSha256": sha256(TABLES),
            "sharedAssets": JOINED.relative_to(ROOT).as_posix(),
            "sharedAssetsSha256": sha256(JOINED),
        },
        "runtimeCompatibility": "evidence-only",
        "genericBuildingPrefab": generic_prefab,
        "sceneBindings": scene_bindings,
        "buildingAnimationAssets": sorted(clips, key=lambda value: value["animationClip"]),
        "buildingVisualBindings": visual_bindings,
        "gaps": [
            "No serialized SpriteRenderer binds a build_* skin to a town coordinate.",
            "No per-building prefab/runtime record maps source build indices to controller classes, popups, sorting, or colliders.",
        ],
    }
    OUT.write_text(json.dumps(output, indent=2) + "\n")
    print(
        f"Wrote {len(clips)} building animations, {len(visual_bindings)} exact source-key bindings, "
        f"and {len(scene_bindings)} scene anchors to {OUT}"
    )


if __name__ == "__main__":
    main()
