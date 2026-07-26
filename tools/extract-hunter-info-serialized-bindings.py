#!/usr/bin/env python3
"""Extract serialized Hunter-detail sprite arrays and scene references."""

from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path

import UnityPy


ROOT = Path(__file__).resolve().parents[1]
LEVEL = ROOT / "game-assets/extracted/joined_unity_files/level1"
GLOBAL = ROOT / "game-assets/extracted/joined_unity_files/globalgamemanagers.assets"
SHARED0 = ROOT / "game-assets/extracted/joined_unity_files/sharedassets0.assets"
TABLES = ROOT / "reverse-engineering/evidence/hunter-info-tables-v1.json"
OUT = ROOT / "reverse-engineering/evidence/hunter-info-serialized-bindings-v1.json"

IMAGE_MANAGER_PATH_ID = 103588
HUNTER_DETAIL_PATH_ID = 74554

ARRAYS = {
    "hunterSkillIcons": {"countOffset": 31208, "count": 50},
    "growthPropertyIcons": {"countOffset": 73324, "count": 15},
    "ridingPetPortraits": {"countOffset": 78376, "count": 21},
    "ridingPetSkillIcons": {"countOffset": 78632, "count": 3},
    "ridingPetTraitIcons": {"countOffset": 78672, "count": 6},
    "ridingPetActorThumbnails": {"countOffset": 78748, "count": 21},
    "jobTraitIcons": {"countOffset": 79364, "count": 69},
}

DETAIL_REFERENCES = {
    "statGroup": {"offset": 628, "pathId": 1810},
    "skillGroup": {"offset": 640, "pathId": 1338},
    "inventoryGroup": {"offset": 652, "pathId": 1671},
    "growthGroup": {"offset": 664, "pathId": 17370},
    "ridingPetGroup": {"offset": 676, "pathId": 16890},
    "basicSkillOneIcon": {"offset": 2604, "componentPathId": 76783},
    "basicSkillTwoIcon": {"offset": 2616, "componentPathId": 73998},
    "heroicSkillOneIcon": {"offset": 5252, "componentPathId": 78639},
    "heroicSkillTwoIcon": {"offset": 5264, "componentPathId": 90932},
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def pptr(data: bytes, offset: int) -> tuple[int, int]:
    return struct.unpack_from("<iq", data, offset)


def script_class(data: bytes, scripts: dict[int, object]) -> dict[str, object]:
    file_id, path_id = pptr(data, 16)
    script = scripts[path_id].read()
    return {
        "fileId": file_id,
        "pathId": path_id,
        "className": script.m_ClassName,
        "assemblyName": script.m_AssemblyName,
    }


def sprite_array(data: bytes, spec: dict[str, int], sprites: dict[int, object]) -> list[dict[str, object]]:
    count_offset = spec["countOffset"]
    count = struct.unpack_from("<i", data, count_offset)[0]
    if count != spec["count"]:
        raise ValueError(f"expected {spec['count']} entries at {count_offset}, found {count}")
    rows = []
    for position in range(count):
        offset = count_offset + 4 + position * 12
        file_id, path_id = pptr(data, offset)
        if file_id != 3 or path_id not in sprites or sprites[path_id].type.name != "Sprite":
            raise ValueError(f"invalid sprite PPtr at {offset}: file={file_id}, path={path_id}")
        rows.append({
            "position": position,
            "pptrOffset": offset,
            "fileId": file_id,
            "pathId": path_id,
            "spriteName": sprites[path_id].read().m_Name,
        })
    return rows


def image_sprite(component: object, sprites: dict[int, object]) -> dict[str, object]:
    raw = component.get_raw_data()
    file_id, path_id = pptr(raw, 88)
    if file_id != 3 or path_id not in sprites or sprites[path_id].type.name != "Sprite":
        raise ValueError(f"Image component {component.path_id} has no expected sprite at offset 88")
    return {"fileId": file_id, "pathId": path_id, "spriteName": sprites[path_id].read().m_Name}


def main() -> None:
    level = UnityPy.load(str(LEVEL))
    global_managers = UnityPy.load(str(GLOBAL))
    shared0 = UnityPy.load(str(SHARED0))
    level_objects = {obj.path_id: obj for obj in level.objects}
    scripts = {obj.path_id: obj for obj in global_managers.objects}
    sprites = {obj.path_id: obj for obj in shared0.objects}

    image_manager_raw = level_objects[IMAGE_MANAGER_PATH_ID].get_raw_data()
    detail_raw = level_objects[HUNTER_DETAIL_PATH_ID].get_raw_data()
    arrays = {key: sprite_array(image_manager_raw, spec, sprites) for key, spec in ARRAYS.items()}

    detail_refs = {}
    for key, expected in DETAIL_REFERENCES.items():
        file_id, path_id = pptr(detail_raw, expected["offset"])
        expected_path = expected.get("pathId", expected.get("componentPathId"))
        if file_id != 0 or path_id != expected_path:
            raise ValueError(f"unexpected {key} PPtr at {expected['offset']}: {file_id}/{path_id}")
        entry = {"pptrOffset": expected["offset"], "fileId": file_id, "pathId": path_id}
        if "componentPathId" in expected:
            entry["sprite"] = image_sprite(level_objects[path_id], sprites)
        else:
            entry["gameObjectName"] = level_objects[path_id].read().m_Name
        detail_refs[key] = entry

    tables = json.loads(TABLES.read_text())
    correlations = {
        "basicSkills": {
            "tableRows": len(tables["skills"]),
            "arrayPositions": [0, 9],
            "status": "partially-confirmed",
            "confirmed": [
                {
                    "tableIndex": 0,
                    "name": tables["skills"][0]["localized"]["en"]["name"],
                    "spriteName": detail_refs["basicSkillOneIcon"]["sprite"]["spriteName"],
                    "evidence": "Berserker FirstSkillGroup serialized default plus supplied original screenshot",
                },
                {
                    "tableIndex": 1,
                    "name": tables["skills"][1]["localized"]["en"]["name"],
                    "spriteName": detail_refs["basicSkillTwoIcon"]["sprite"]["spriteName"],
                    "evidence": "Berserker SecondSkillGroup serialized default plus supplied original screenshot",
                },
            ],
            "limitation": "The other class pairs match the ImageManager job-major array structurally, but protected method bodies still hide the lookup expression.",
        },
        "subJobSkills": {
            "tableRows": len(tables["subJobSkills"]),
            "arrayPositions": [10, 49],
            "status": "structural-only",
            "limitation": "The 40 icons are serialized in four class-major blocks, while QuickSheet rows are progression-major and H5 rows are appended; exact row lookup still requires the native selection expression.",
        },
        "growthProperties": {
            "tableRows": len(tables["growthProperties"]),
            "arrayPositions": [0, 14],
            "status": "serialized-position-match",
            "limitation": "Count, contiguous indices and icon suffixes align one-to-one; native array indexing is not decompiled.",
        },
        "ridingPets": {
            "tableRows": len(tables["ridingPets"]),
            "arrayPositions": [0, 20],
            "status": "serialized-position-match",
            "limitation": "Position zero and one intentionally share the first sprite; native table-index lookup is not decompiled.",
        },
        "ridingPetSkills": {
            "tableRows": len(tables["ridingPetSkills"]),
            "arrayPositions": [0, 2],
            "status": "serialized-position-match",
            "limitation": "Native table-index lookup is not decompiled.",
        },
        "ridingPetTraits": {
            "tableRows": len(tables["ridingPetTraits"]),
            "arrayPositions": [0, 5],
            "status": "serialized-position-match",
            "limitation": "Native table-index lookup is not decompiled.",
        },
        "jobTraits": {
            "tableRows": len(tables["jobTraits"]),
            "arrayPositions": [0, 68],
            "status": "serialized-position-match",
            "limitation": "The asset names encode the same all/job/branch/order grouping, but native table-index lookup is not decompiled.",
        },
    }

    output = {
        "schemaVersion": 1,
        "contractType": "hunter-info-serialized-binding-evidence",
        "runtimeCompatibility": "evidence-only",
        "sources": {
            "level1": {"path": LEVEL.relative_to(ROOT).as_posix(), "sha256": sha256(LEVEL)},
            "globalManagers": {"path": GLOBAL.relative_to(ROOT).as_posix(), "sha256": sha256(GLOBAL)},
            "sharedAssets0": {"path": SHARED0.relative_to(ROOT).as_posix(), "sha256": sha256(SHARED0)},
            "tables": {"path": TABLES.relative_to(ROOT).as_posix(), "sha256": sha256(TABLES)},
        },
        "objects": {
            "imageManager": {
                "pathId": IMAGE_MANAGER_PATH_ID,
                "byteSize": len(image_manager_raw),
                "rawSha256": hashlib.sha256(image_manager_raw).hexdigest(),
                "script": script_class(image_manager_raw, scripts),
            },
            "hunterDetailPop": {
                "pathId": HUNTER_DETAIL_PATH_ID,
                "byteSize": len(detail_raw),
                "rawSha256": hashlib.sha256(detail_raw).hexdigest(),
                "script": script_class(detail_raw, scripts),
            },
        },
        "serializedArrays": arrays,
        "hunterDetailReferences": detail_refs,
        "tableCorrelations": correlations,
        "policy": "PPtr offsets, object identities, array lengths and sprite names are exact serialized facts. A table-row binding is not marked confirmed solely because names, counts or positions look compatible.",
    }
    OUT.write_text(json.dumps(output, ensure_ascii=False, indent=2) + "\n")
    print(f"Wrote serialized Hunter info binding evidence to {OUT}")


if __name__ == "__main__":
    main()
