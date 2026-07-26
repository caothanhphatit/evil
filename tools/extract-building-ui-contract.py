#!/usr/bin/env python3
"""Build an evidence-only contract for the recovered building UI surface."""

from __future__ import annotations

import csv
import hashlib
import json
import re
import struct
from pathlib import Path

import UnityPy

from scene_evidence_lib import mono_behaviour_header


ROOT = Path(__file__).resolve().parents[1]
JOINED = ROOT / "game-assets/extracted/joined_unity_files"
DATA = ROOT / "game-assets/source/unity-assets/bin/Data"
INVENTORY = ROOT / "game-assets/extracted/exported/metadata/inventory.json"
MONOSCRIPTS = ROOT / "reverse-engineering/evidence/monoscripts.csv"
SCENE = ROOT / "reverse-engineering/evidence/level1-scene-evidence-v2.json"
OUT = ROOT / "reverse-engineering/evidence/building-ui-contract-v1.json"

BUILD_PATTERN = re.compile(
    r"build|building|townhall|pasture|revive", re.IGNORECASE
)
POPUP_TARGETS = {
    "BuildingPop": {"panel": "Background", "controller": "BuildingPop", "role": "multi-mode-building-management"},
    "RequestPop": {"panel": "Background", "controller": "RequestPop", "role": "bounty-request"},
    "GearCreatePop": {"panel": "Background", "controller": "GearCreatePop", "role": "gear-crafting"},
    "ConsumCreatePop": {"panel": "Background", "controller": "ConsumCreatePop", "role": "potion-crafting"},
    "ProductCreatePop": {"panel": "Background", "controller": "ProductCreatePop", "role": "building-product-quantity-and-conversion"},
    "TradeWagonExchangePop": {"panel": "Border", "controller": "TradeWagonExchangePop", "role": "trade-wagon-exchange"},
}
LOCAL_UI_PATH_ID = 9082
LOCAL_UI_LOCALES = ("ko", "ja", "en", "zh-TW", "zh-CN", "ru", "fr", "es", "pt", "it", "de", "th", "vi", "id")
POPUP_LOCALIZATION_PREFIXES = ("buildpop_", "requestpop_", "gearcreatepop_", "consumcreatepop_", "productcreatepop_", "tradewagon")


def digest(path: Path) -> dict:
    payload = path.read_bytes()
    return {"path": path.relative_to(ROOT).as_posix(), "bytes": len(payload), "sha256": hashlib.sha256(payload).hexdigest()}


def load_scripts() -> dict[int, dict]:
    with MONOSCRIPTS.open(newline="") as handle:
        return {
            int(row["path_id"]): {
                "pathId": int(row["path_id"]),
                "name": row["name"],
                "class": row["class"],
                "namespace": row["namespace"],
                "assembly": row["assembly"],
            }
            for row in csv.DictReader(handle)
        }


def serialized_file(environment):
    return next(item for item in environment.files.values() if hasattr(item, "objects"))


def component_scripts(environment, scripts: dict[int, dict]) -> dict[int, dict]:
    result = {}
    for reader in environment.objects:
        if reader.type.name != "MonoBehaviour":
            continue
        try:
            header = mono_behaviour_header(reader.get_raw_data())
        except Exception:
            continue
        script = scripts.get(header["scriptPathId"])
        result[reader.path_id] = {
            "gameObjectPathId": header["gameObjectPathId"],
            "enabled": header["enabled"],
            "script": script or {
                "pathId": header["scriptPathId"],
                "name": None,
                "class": None,
                "namespace": None,
                "assembly": None,
            },
            "payloadStatus": "header-only",
        }
    return result


def hierarchy(environment, scripts: dict[int, dict]) -> list[dict]:
    objects = {item.path_id: item for item in environment.objects}
    game_objects = {}
    transforms_by_go = {}
    transform_to_go = {}
    mono = component_scripts(environment, scripts)
    for reader in environment.objects:
        if reader.type.name == "GameObject":
            try:
                game_objects[reader.path_id] = reader.read()
            except Exception:
                pass
        elif reader.type.name in ("Transform", "RectTransform"):
            try:
                data = reader.read()
                transforms_by_go[data.m_GameObject.path_id] = data
                transform_to_go[reader.path_id] = data.m_GameObject.path_id
            except Exception:
                pass

    def object_path(game_object_id: int) -> str:
        parts = []
        current = game_object_id
        for _ in range(128):
            game_object = game_objects.get(current)
            if not game_object:
                break
            parts.append(f"{game_object.m_Name}[{current}]")
            transform = transforms_by_go.get(current)
            if not transform or transform.m_Father.file_id != 0 or transform.m_Father.path_id == 0:
                break
            current = transform_to_go.get(transform.m_Father.path_id, 0)
            if not current:
                break
        return "/".join(reversed(parts))

    records = []
    for path_id, game_object in game_objects.items():
        path = object_path(path_id)
        own_match = BUILD_PATTERN.search(game_object.m_Name)
        if not own_match and not BUILD_PATTERN.search(path):
            continue
        transform = transforms_by_go.get(path_id)
        components = []
        for pair in game_object.m_Component:
            component = objects.get(pair.component.path_id)
            if not component:
                continue
            entry = {"pathId": component.path_id, "type": component.type.name}
            if component.type.name == "MonoBehaviour" and component.path_id in mono:
                entry.update(mono[component.path_id])
            elif component.type.name == "Animator":
                try:
                    animator = component.read()
                    entry["controller"] = {
                        "fileId": animator.m_Controller.file_id,
                        "pathId": animator.m_Controller.path_id,
                    }
                except Exception:
                    entry["controller"] = None
            elif component.type.name == "SpriteRenderer":
                try:
                    renderer = component.read()
                    entry["sprite"] = {
                        "fileId": renderer.m_Sprite.file_id,
                        "pathId": renderer.m_Sprite.path_id,
                    }
                except Exception:
                    entry["sprite"] = None
            components.append(entry)
        record = {
            "pathId": path_id,
            "name": game_object.m_Name,
            "path": path,
            "selectionReason": "name-match" if own_match else "descendant-of-building-object",
            "identityConfidence": "confirmed",
            "semanticRoleConfidence": "name-only" if own_match else "confirmed-hierarchy-only",
            "active": bool(game_object.m_IsActive),
            "components": components,
        }
        if transform:
            record["transform"] = {
                "type": type(transform).__name__,
                "pathId": transform.object_reader.path_id,
                "localPosition": {"x": transform.m_LocalPosition.x, "y": transform.m_LocalPosition.y, "z": transform.m_LocalPosition.z},
                "localScale": {"x": transform.m_LocalScale.x, "y": transform.m_LocalScale.y, "z": transform.m_LocalScale.z},
                "parentTransformPathId": transform.m_Father.path_id,
                "childTransformPathIds": [item.path_id for item in transform.m_Children],
            }
        records.append(record)
    return sorted(records, key=lambda item: (item["name"].lower(), item["pathId"]))


def resolve_pointer(pointer, local_objects, externals, external_cache) -> dict:
    if pointer.path_id == 0:
        return {"fileId": pointer.file_id, "pathId": 0, "status": "null"}
    if pointer.file_id == 0:
        reader = local_objects.get(pointer.path_id)
        data = reader.read() if reader else None
        return {
            "fileId": 0,
            "pathId": pointer.path_id,
            "type": reader.type.name if reader else None,
            "name": getattr(data, "m_Name", None),
            "source": "sharedassets1.assets",
            "status": "resolved" if reader else "unresolved",
        }
    bundle = externals.get(pointer.file_id)
    source = DATA / bundle if bundle else None
    if source and not source.exists() and (JOINED / bundle).exists():
        source = JOINED / bundle
    if not source or not source.exists():
        return {"fileId": pointer.file_id, "pathId": pointer.path_id, "source": bundle, "status": "unresolved"}
    if bundle not in external_cache:
        external_cache[bundle] = UnityPy.load(str(source))
    reader = next((item for item in external_cache[bundle].objects if item.path_id == pointer.path_id), None)
    data = reader.read() if reader else None
    return {
        "fileId": pointer.file_id,
        "pathId": pointer.path_id,
        "type": reader.type.name if reader else None,
        "name": getattr(data, "m_Name", None),
        "source": bundle,
        "sourceSha256": digest(source)["sha256"],
        "status": "resolved" if reader else "unresolved",
    }


def animation_contract(environment) -> tuple[list[dict], list[dict]]:
    file = serialized_file(environment)
    externals = {index: item.path for index, item in enumerate(file.externals, 1)}
    local_objects = {item.path_id: item for item in environment.objects}
    external_cache = {}
    clips = {}
    for reader in environment.objects:
        if reader.type.name != "AnimationClip":
            continue
        clip = reader.read()
        if not (clip.m_Name.startswith("build_") or clip.m_Name.startswith("buildSkin_")):
            continue
        mappings = [
            resolve_pointer(pointer, local_objects, externals, external_cache)
            for pointer in clip.m_ClipBindingConstant.pptrCurveMapping
        ]
        clips[reader.path_id] = {
            "clipPathId": reader.path_id,
            "name": clip.m_Name,
            "assetClass": "build-skin-variant" if clip.m_Name.startswith("buildSkin_") else ("build-run-effect" if clip.m_Name.startswith("build_run_") else "base-building"),
            "spriteSequence": mappings,
            "displayName": {"status": "unresolved", "value": None},
            "townPosition": {"status": "unresolved", "value": None},
            "bindingConfidence": "confirmed-serialized-reference",
        }
    controllers = []
    for reader in environment.objects:
        if reader.type.name != "AnimatorController":
            continue
        controller = reader.read()
        if not (controller.m_Name.startswith("build_") or controller.m_Name.startswith("buildSkin_")):
            continue
        linked = [pointer.path_id for pointer in controller.m_AnimationClips]
        controllers.append({
            "controllerPathId": reader.path_id,
            "name": controller.m_Name,
            "clipPathIds": linked,
            "clips": [clips[path_id]["name"] if path_id in clips else None for path_id in linked],
            "bindingConfidence": "confirmed-serialized-reference",
        })
    return sorted(clips.values(), key=lambda item: item["name"]), sorted(controllers, key=lambda item: item["name"])


def aligned_string(raw: bytes, offset: int) -> str | None:
    if offset + 4 > len(raw):
        return None
    length = struct.unpack_from("<i", raw, offset)[0]
    if length < 0 or offset + 4 + length > len(raw):
        return None
    try:
        return raw[offset + 4:offset + 4 + length].decode("utf-8")
    except UnicodeDecodeError:
        return None


def decoded_popup_localization(environment, serialized_keys: set[str]) -> dict:
    reader = next(item for item in environment.objects if item.path_id == LOCAL_UI_PATH_ID)
    raw = reader.get_raw_data()
    offset = 28

    def read_int32() -> int:
        nonlocal offset
        value = struct.unpack_from("<i", raw, offset)[0]
        offset += 4
        return value

    def read_string() -> str:
        nonlocal offset
        length = read_int32()
        end = offset + length
        value = raw[offset:end].decode("utf-8")
        offset = (end + 3) & ~3
        return value

    header = {
        "name": read_string(),
        "spreadsheetId": read_string(),
        "spreadsheetName": read_string(),
        "worksheetName": read_string(),
        "rowCount": read_int32(),
    }
    rows = []
    for _ in range(header["rowCount"]):
        index = read_int32()
        key = read_string()
        explanation = read_string()
        localized = {locale: read_string() for locale in LOCAL_UI_LOCALES}
        if key in serialized_keys or key.startswith(POPUP_LOCALIZATION_PREFIXES):
            rows.append({"index": index, "key": key, "explanationKo": explanation, "localized": localized})
    if offset != len(raw):
        raise ValueError(f"localUI trailing bytes: {len(raw) - offset}")
    return {
        "sourcePathId": LOCAL_UI_PATH_ID,
        "rawBytes": len(raw),
        "rawSha256": hashlib.sha256(raw).hexdigest(),
        "header": header,
        "selection": "All popup-prefix rows plus every localization key serialized under the recovered popup roots.",
        "rows": rows,
    }
def popup_templates(environment, scripts: dict[int, dict], inventory: list[dict]) -> list[dict]:
    file = serialized_file(environment)
    externals = {index: item.path for index, item in enumerate(file.externals, 1)}
    objects = {item.path_id: item for item in environment.objects}
    inventory_by_pointer = {(item["source"], item["path_id"]): item for item in inventory}
    game_objects = {}
    transforms_by_go = {}
    transform_to_go = {}
    mono = component_scripts(environment, scripts)
    for reader in environment.objects:
        if reader.type.name == "GameObject":
            try:
                game_objects[reader.path_id] = reader.read()
            except Exception:
                pass
        elif reader.type.name in ("Transform", "RectTransform"):
            try:
                transform = reader.read()
                transforms_by_go[transform.m_GameObject.path_id] = transform
                transform_to_go[reader.path_id] = transform.m_GameObject.path_id
            except Exception:
                pass

    children_by_go = {path_id: [] for path_id in game_objects}
    for game_object_id, transform in transforms_by_go.items():
        if transform.m_Father.file_id == 0 and transform.m_Father.path_id:
            parent = transform_to_go.get(transform.m_Father.path_id)
            if parent in children_by_go:
                children_by_go[parent].append(game_object_id)

    def component_contract(component_id: int) -> dict:
        reader = objects[component_id]
        entry = {"pathId": component_id, "type": reader.type.name}
        if reader.type.name != "MonoBehaviour" or component_id not in mono:
            return entry
        component = mono[component_id]
        script = component["script"]
        entry.update({"script": script, "enabled": component["enabled"]})
        raw = reader.get_raw_data()
        if script["namespace"] == "UnityEngine.UI" and script["class"] == "Image" and len(raw) >= 100:
            file_id = struct.unpack_from("<i", raw, 88)[0]
            path_id = struct.unpack_from("<q", raw, 92)[0]
            if path_id == 0:
                sprite = {"fileId": file_id, "pathId": 0, "status": "null"}
            else:
                source = "level1" if file_id == 0 else externals.get(file_id)
                indexed = inventory_by_pointer.get((source, path_id))
                direct_reader = None
                direct_source = DATA / source if source else None
                if not indexed and direct_source and direct_source.exists() and direct_source.stat().st_size <= 1_000_000:
                    direct_environment = UnityPy.load(str(direct_source))
                    direct_reader = next((item for item in direct_environment.objects if item.path_id == path_id), None)
                    direct_data = direct_reader.read() if direct_reader else None
                sprite = {
                    "fileId": file_id,
                    "pathId": path_id,
                    "source": source,
                    "type": indexed.get("type") if indexed else (direct_reader.type.name if direct_reader else None),
                    "name": indexed.get("name") if indexed else getattr(direct_data, "m_Name", None),
                    "status": "resolved-inventory" if indexed else ("resolved-external-object" if direct_reader else "unresolved"),
                }
            entry["sprite"] = sprite
            entry["sprite"]["serializedOffset"] = 88
        elif script["namespace"] == "UnityEngine.UI" and script["class"] == "Text":
            entry["defaultText"] = aligned_string(raw, 144)
            entry["textSerializedOffset"] = 144
        elif script["class"] == "LocalizeTextSetter":
            entry["localizationKey"] = aligned_string(raw, 40)
            entry["localizationKeySerializedOffset"] = 40
        return entry

    templates = []
    for popup_name, target in POPUP_TARGETS.items():
        roots = [path_id for path_id, value in game_objects.items() if value.m_Name == popup_name]
        if len(roots) != 1:
            raise RuntimeError(f"Expected one {popup_name} root, found {len(roots)}")
        root_id = roots[0]
        ordered = []

        def visit(game_object_id: int, depth: int, sibling_index: int) -> None:
            game_object = game_objects[game_object_id]
            transform = transforms_by_go.get(game_object_id)
            record = {
                "pathId": game_object_id,
                "parentGameObjectPathId": None if depth == 0 else transform_to_go.get(transform.m_Father.path_id),
                "name": game_object.m_Name,
                "active": bool(game_object.m_IsActive),
                "depth": depth,
                "siblingIndex": sibling_index,
                "components": [component_contract(pair.component.path_id) for pair in game_object.m_Component if pair.component.path_id in objects],
            }
            if transform:
                record["rectTransform"] = {
                    "anchorMin": {"x": transform.m_AnchorMin.x, "y": transform.m_AnchorMin.y},
                    "anchorMax": {"x": transform.m_AnchorMax.x, "y": transform.m_AnchorMax.y},
                    "anchoredPosition": {"x": transform.m_AnchoredPosition.x, "y": transform.m_AnchoredPosition.y},
                    "sizeDelta": {"x": transform.m_SizeDelta.x, "y": transform.m_SizeDelta.y},
                    "pivot": {"x": transform.m_Pivot.x, "y": transform.m_Pivot.y},
                    "localScale": {"x": transform.m_LocalScale.x, "y": transform.m_LocalScale.y, "z": transform.m_LocalScale.z},
                }
            ordered.append(record)
            for child_index, child_id in enumerate(children_by_go.get(game_object_id, [])):
                visit(child_id, depth + 1, child_index)

        visit(root_id, 0, 0)
        nodes_by_id = {item["pathId"]: item for item in ordered}

        def node_path(game_object_id: int) -> str:
            parts = []
            current = nodes_by_id.get(game_object_id)
            while current:
                parts.append(f"{current['name']}[{current['pathId']}]")
                current = nodes_by_id.get(current["parentGameObjectPathId"])
            return "/".join(reversed(parts))

        panel = next((item for item in ordered if item["depth"] == 1 and item["name"] == target["panel"]), None)
        if not panel:
            raise RuntimeError(f"Missing {popup_name}/{target['panel']} panel")
        controllers = sorted({
            component["script"]["class"]
            for item in ordered for component in item["components"]
            if component.get("script", {}).get("assembly") == "Assembly-CSharp"
        })
        labels = [
            {"gameObjectPathId": item["pathId"], "path": node_path(item["pathId"]), **binding}
            for item in ordered
            for component in item["components"]
            for binding in [{key: component[key] for key in ("defaultText", "localizationKey") if component.get(key)}]
            if binding
        ]
        sprites = [
            {"gameObjectPathId": item["pathId"], "name": item["name"], "sprite": component["sprite"]}
            for item in ordered for component in item["components"]
            if "sprite" in component and component["sprite"].get("status") != "null"
        ]
        templates.append({
            "name": popup_name,
            "rootGameObjectPathId": root_id,
            "rootControllerClass": target["controller"] if target["controller"] in controllers else None,
            "panelGameObjectPathId": panel["pathId"],
            "panelName": panel["name"],
            "panelDimensions": panel["rectTransform"]["sizeDelta"],
            "panelSprite": next((component["sprite"] for component in panel["components"] if "sprite" in component), None),
            "controllerClasses": controllers,
            "hierarchy": ordered,
            "labels": labels,
            "spriteBindings": sprites,
            "bindingConfidence": "confirmed-serialized-scene",
            "semanticRole": {
                "value": target["role"],
                "confidence": "strongly-inferred",
                "reason": "Controller class, hierarchy names, and decoded localUI labels agree; this does not identify a building dispatch call-site.",
            },
            "buildingIdBinding": {"status": "unresolved", "value": None, "reason": "Scene hierarchy proves the popup template, not the native building-to-popup dispatch call-site."},
        })
    return templates


def main() -> None:
    inventory = json.loads(INVENTORY.read_text())
    scripts = load_scripts()
    relevant_scripts = [item for item in scripts.values() if BUILD_PATTERN.search(item["name"]) or item["name"] in POPUP_TARGETS]
    relevant_inventory = [
        item for item in inventory
        if BUILD_PATTERN.search(item.get("name", ""))
    ]
    shared0 = UnityPy.load(str(JOINED / "sharedassets0.assets"))
    shared1 = UnityPy.load(str(JOINED / "sharedassets1.assets"))
    level1 = UnityPy.load(str(JOINED / "level1"))
    clips, controllers = animation_contract(shared1)
    popups = popup_templates(level1, scripts, inventory)
    popup_keys = {
        label["localizationKey"]
        for popup in popups for label in popup["labels"]
        if "localizationKey" in label
    }
    popup_localization = decoded_popup_localization(shared0, popup_keys)
    popup_localization_by_key = {row["key"]: row for row in popup_localization["rows"]}
    for popup in popups:
        for label in popup["labels"]:
            if label.get("localizationKey") in popup_localization_by_key:
                label["localized"] = popup_localization_by_key[label["localizationKey"]]["localized"]
    scene_objects = hierarchy(level1, scripts)
    prefab_objects = hierarchy(shared1, scripts) + hierarchy(shared0, scripts)

    def ui_index(records: list[dict], source: str) -> list[dict]:
        indexed = []
        for item in records:
            if item["selectionReason"] != "name-match":
                continue
            name = item["name"]
            lowered = name.lower()
            if "pop" in lowered:
                role = "popup"
            elif "rowlist" in lowered or lowered.endswith("list") or "infolist" in lowered:
                role = "list-or-row-prefab"
            elif "manager" in lowered or lowered.endswith("ctrl"):
                role = "controller-object"
            elif "button" in lowered or "tab" in lowered or "select" in lowered:
                role = "control"
            else:
                role = "building-related-object"
            indexed.append({
                "source": source,
                "pathId": item["pathId"],
                "name": name,
                "path": item["path"],
                "role": role,
                "roleConfidence": "name-only",
            })
        return indexed

    ui_objects = ui_index(scene_objects, "level1") + ui_index(prefab_objects, "sharedassets")
    output = {
        "schemaVersion": 1,
        "contractType": "building-ui-evidence",
        "runtimeCompatibility": "not-claimed",
        "confidenceVocabulary": ["confirmed", "name-only", "unresolved"],
        "sources": [digest(path) for path in [JOINED / "level1", JOINED / "sharedassets0.assets", JOINED / "sharedassets1.assets", INVENTORY, MONOSCRIPTS, SCENE]],
        "buildIdPolicy": {
            "status": "unresolved",
            "note": "Animation/controller suffixes are source identifiers, not proven gameplay building IDs.",
        },
        "animationAssets": clips,
        "controllers": controllers,
        "popupTemplates": popups,
        "popupLocalization": popup_localization,
        "classes": sorted(relevant_scripts, key=lambda item: item["name"].lower()),
        "sceneObjects": scene_objects,
        "prefabObjects": sorted(prefab_objects, key=lambda item: (item["name"].lower(), item["pathId"])),
        "uiObjectIndex": sorted(ui_objects, key=lambda item: (item["role"], item["source"], item["name"].lower(), item["pathId"])),
        "inventoryObjects": sorted(relevant_inventory, key=lambda item: (item["source"], item["type"], item.get("name", ""), item["path_id"])),
        "displayNames": {
            "status": "unresolved",
            "bindings": [],
            "note": "No decoded Text/Image MonoBehaviour payload or localization table binds a displayed building name to these objects.",
        },
        "placement": {
            "confirmed": [{"sceneGameObjectPathId": 260, "name": "ReviveBuilding", "localPosition": {"x": 14.713000297546387, "y": 13.725000381469727, "z": 0.0}, "spriteBinding": None}],
            "unresolved": "All other building positions and all building-sprite-to-position bindings.",
        },
        "gaps": [
            "MonoBehaviour payloads are header-only because matching IL2CPP serialized type trees are unavailable.",
            "No recovered save or runtime trace maps controller suffixes to semantic building names.",
            "Decoded popup localization does not bind semantic building display names to scene building objects.",
            "No scene SpriteRenderer binds a build_* or buildSkin_* sprite to a town coordinate.",
            "Popup templates are exact scene objects, but building-specific popup dispatch remains unresolved without a native call-site or runtime trace.",
        ],
    }
    OUT.write_text(json.dumps(output, indent=2) + "\n")
    print(f"Wrote {len(clips)} animations, {len(controllers)} controllers, {len(scene_objects)} scene objects, {len(prefab_objects)} prefab objects -> {OUT}")


if __name__ == "__main__":
    main()
