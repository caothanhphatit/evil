#!/usr/bin/env python3
"""Join serialized AdminBuildSkinData rows to exact Unity visual assets."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import UnityPy


ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "game-assets/source/unity-assets/bin/Data"
SHARED = ROOT / "game-assets/extracted/joined_unity_files/sharedassets1.assets"
TABLES = ROOT / "reverse-engineering/evidence/serialized-building-tables-v1.json"
OUT = ROOT / "reverse-engineering/evidence/building-skin-evidence-v1.json"

FAMILY_RULES = {
    1: {
        "family": "middle-ages",
        "assetKeyRule": "buildSkin_1_{buildingId-1}",
        "spritePrefixRule": "bd_a_cos_{buildingId:03}_",
        "assetKey": lambda building_id: f"buildSkin_1_{building_id - 1}",
        "spritePrefix": lambda building_id: f"bd_a_cos_{building_id:03d}_",
    },
    2: {
        "family": "halloween",
        "assetKeyRule": "buildSkin_2_{buildingId}",
        "spritePrefixRule": "bd_a_halloween_{buildingId:03}_",
        "assetKey": lambda building_id: f"buildSkin_2_{building_id}",
        "spritePrefix": lambda building_id: f"bd_a_halloween_{building_id:03d}_",
    },
    3: {
        "family": "christmas",
        "assetKeyRule": "buildSkin_3_{buildingId}",
        "spritePrefixRule": "bd_a_christmas_{buildingId:03}_",
        "assetKey": lambda building_id: f"buildSkin_3_{building_id}",
        "spritePrefix": lambda building_id: f"bd_a_christmas_{building_id:03d}_",
    },
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    tables = json.loads(TABLES.read_text(encoding="utf-8"))
    environment = UnityPy.load(str(SHARED))
    serialized = next(iter(environment.files.values()))
    externals = {index: item.path for index, item in enumerate(serialized.externals, 1)}
    local_objects = {item.path_id: item for item in environment.objects}
    clips = {}
    controllers = {}
    for item in environment.objects:
        if item.type.name == "AnimationClip":
            data = item.read()
            if data.m_Name.startswith("buildSkin_"):
                clips[data.m_Name] = (item, data)
        elif item.type.name == "AnimatorController":
            data = item.read()
            if data.m_Name.startswith("buildSkin_"):
                controllers[data.m_Name] = (item, data)

    external_environments: dict[str, UnityPy.Environment] = {}
    rows = []
    expected_keys = set()
    for source_row in tables["buildingSkins"]:
        skin_id = source_row["skinId"]
        building_id = source_row["buildingId"]
        rule = FAMILY_RULES.get(skin_id)
        if rule is None:
            raise ValueError(f"unsupported serialized skin family {skin_id}")
        asset_key = rule["assetKey"](building_id)
        sprite_prefix = rule["spritePrefix"](building_id)
        expected_keys.add(asset_key)
        clip_pair = clips.get(asset_key)
        controller_pair = controllers.get(asset_key)
        visual = {
            "state": "unresolved",
            "expectedAssetKey": asset_key,
            "expectedSpritePrefix": sprite_prefix,
            "reason": "exact animation clip is absent from sharedassets1.assets",
        }
        if clip_pair is not None and controller_pair is not None:
            clip_reader, clip = clip_pair
            controller_reader, controller = controller_pair
            controller_clip_ids = [pointer.path_id for pointer in controller.m_AnimationClips]
            if controller_clip_ids == [clip_reader.path_id]:
                frames = []
                valid = True
                for pointer in clip.m_ClipBindingConstant.pptrCurveMapping:
                    if pointer.file_id == 0:
                        sprite_reader = local_objects.get(pointer.path_id)
                        source_bundle = None
                        source_hash = sha256(SHARED)
                    else:
                        source_bundle = externals.get(pointer.file_id)
                        if source_bundle is None:
                            valid = False
                            break
                        source_path = DATA / source_bundle
                        external = external_environments.setdefault(
                            source_bundle, UnityPy.load(str(source_path))
                        )
                        sprite_reader = next(
                            (item for item in external.objects if item.path_id == pointer.path_id),
                            None,
                        )
                        source_hash = sha256(source_path)
                    if sprite_reader is None or sprite_reader.type.name != "Sprite":
                        valid = False
                        break
                    sprite_name = sprite_reader.read().m_Name
                    if not sprite_name.startswith(sprite_prefix):
                        valid = False
                        break
                    frames.append(
                        {
                            "sourceBundle": source_bundle,
                            "sourceSha256": source_hash,
                            "pathId": sprite_reader.path_id,
                            "name": sprite_name,
                        }
                    )
                if valid and frames:
                    visual = {
                        "state": "resolved",
                        "bindingConfidence": "exact-table-rule-clip-controller-sprite-chain",
                        "assetKey": asset_key,
                        "animationClip": {"pathId": clip_reader.path_id, "name": clip.m_Name},
                        "animatorController": {
                            "pathId": controller_reader.path_id,
                            "name": controller.m_Name,
                            "animationClipPathIds": controller_clip_ids,
                        },
                        "spritePrefix": sprite_prefix,
                        "spriteFrames": frames,
                    }
                else:
                    visual["reason"] = "clip/controller exists but its sprite chain violates the exact family rule"
            else:
                visual["reason"] = "controller does not reference exactly the same-named animation clip"
        elif clip_pair is not None or controller_pair is not None:
            visual["reason"] = "only one member of the exact clip/controller pair exists"

        rows.append(
            {
                "sourceRowIndex": source_row["index"],
                "buildingId": building_id,
                "skinId": skin_id,
                "family": rule["family"],
                "titles": source_row["titles"],
                "requiredGold": source_row["requiredGold"],
                "requiredMaterialIds": source_row["requiredMaterialIds"],
                "requiredMaterialQuantities": source_row["requiredMaterialQuantities"],
                "requiredLevel": source_row["requiredLevel"],
                "visibility": source_row["visibility"],
                "visualBinding": visual,
            }
        )

    asset_keys = set(clips) | set(controllers)
    orphan_keys = sorted(asset_keys - expected_keys)
    resolved_count = sum(row["visualBinding"]["state"] == "resolved" for row in rows)
    output = {
        "schemaVersion": 1,
        "contractType": "building-skin-serialized-visual-evidence",
        "runtimeCompatibility": "evidence-only",
        "sources": {
            "serializedBuildingTables": TABLES.relative_to(ROOT).as_posix(),
            "serializedBuildingTablesSha256": sha256(TABLES),
            "sharedAssets": SHARED.relative_to(ROOT).as_posix(),
            "sharedAssetsSha256": sha256(SHARED),
        },
        "familyRules": [
            {
                "skinId": skin_id,
                "family": rule["family"],
                "assetKeyRule": rule["assetKeyRule"],
                "spritePrefixRule": rule["spritePrefixRule"],
                "evidence": "Every resolved row must satisfy the table-derived key, same-name clip/controller link, and sprite prefix.",
            }
            for skin_id, rule in sorted(FAMILY_RULES.items())
        ],
        "coverage": {
            "serializedRows": len(rows),
            "resolvedVisualBindings": resolved_count,
            "unresolvedVisualBindings": len(rows) - resolved_count,
            "orphanAssetKeys": len(orphan_keys),
        },
        "rows": rows,
        "orphanAssets": [
            {
                "assetKey": key,
                "animationClipPathId": clips[key][0].path_id if key in clips else None,
                "animatorControllerPathId": controllers[key][0].path_id if key in controllers else None,
                "reason": "No AdminBuildSkinData row produces this key under the exact family rules.",
            }
            for key in orphan_keys
        ],
        "policy": "Missing or orphan visual assets remain explicit; no filename-similarity fallback is permitted.",
    }
    OUT.write_text(json.dumps(output, ensure_ascii=True, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"Wrote {len(rows)} skin rows: {resolved_count} resolved, "
        f"{len(rows) - resolved_count} unresolved, {len(orphan_keys)} orphan asset(s) -> {OUT}"
    )


if __name__ == "__main__":
    main()
