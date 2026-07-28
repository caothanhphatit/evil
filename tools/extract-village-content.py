#!/usr/bin/env python3
"""Extract scene-bound village sprites from immutable Unity bundles.

UnityPy is intentionally used here instead of a hand-maintained PNG copy so a
future asset refresh can reproduce the exact browser content package.
"""

from pathlib import Path
import hashlib
import json
import UnityPy
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "game-assets/source/unity-assets/bin/Data"
OUT = ROOT / "game-assets/normalized/village"

FOREGROUND = {
    "ground": ("fc6112344ba24e846ab93206222bc5ce", 1, "skin_botton_00"),
    "gate": ("69d52ffa47a4c624da3b4f6b9e3ba220", 1, "skin_gate_00"),
    "wallA": ("b13a1ee42a18e194cbf1f8a074eb05e2", 1, "skin_wallA_00"),
    "wallB": ("44f0ab482bf1a8b4c8839576a308a16b", 1, "skin_wallB_00"),
    "wallC": ("509a760d786690d468890bf130819324", 1, "skin_wallC_00"),
    "wallD": ("119508b99aa4a4a49b5966c5738f62f1", 1, "skin_wallD_00"),
    "wallE": ("e64a839e6c603d34e9a89b00ffbbae53", 1, "skin_wallE_00"),
    "bridgeA": ("bf7d60d575a7bb24fa3ba6753c8abaff", 1, "skin_bridgeA_00"),
    "bridgeB": ("ff81645f1f305f74ca58acd9196ddc6f", 1, "skin_bridgeB_00"),
    "bridgeC": ("c1bc3dd9f6b3ac342a7f8771421dbb0d", 1, "skin_bridgeC_00"),
}

NPC_BUNDLES = {
    "farm_npc_1": ("2ed56cd26b560684c8009cb9d7e5cf41", "img_farm_npc_1_"),
    "farm_npc_2": ("c22dc728206506d458a90434973e7b51", "img_farm_npc_2_"),
    "fallen_pasture_npc": ("91499e849527b97488223e41557f71c5", "fallen_pasture_npc_"),
}

# These three scene-bound bundles are referenced by sign_01, sign_02, and
# sign_03 in level1. Each bundle contains the exact I/II/III visual states.
SIGNBOARD_BUNDLES = {
    "map_new01": ("a3915f675ec081c418a8ca3ca9931e8f", "area_sign_1_"),
    "background_08": ("a51b9fd00b7358340a58882091ffc38a", "area_sign_2_"),
    "background_11": ("4eaad883811ba9640b4f0ab70471bf17", "area_sign_3_"),
}


def digest(path: Path) -> dict:
    data = path.read_bytes()
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def save_with_transparent_matte(image: Image.Image, output: Path) -> None:
    """Unity Sprite exports may carry a white matte; restore transparent pixels for web compositing."""
    rgba = image.convert("RGBA")
    pixels = rgba.load()
    for y in range(rgba.height):
        for x in range(rgba.width):
            r, g, b, a = pixels[x, y]
            if a and r >= 248 and g >= 248 and b >= 248:
                pixels[x, y] = (r, g, b, 0)
    rgba.save(output, format="PNG", optimize=False)


def extract_sprite(bundle: str, path_id: int, name: str, output: Path) -> dict:
    env = UnityPy.load(str(DATA / bundle))
    obj = next((item for item in env.objects if item.path_id == path_id), None)
    if obj is None or obj.type.name != "Sprite":
        raise RuntimeError(f"Sprite {name} ({bundle}:{path_id}) was not found")
    sprite = obj.read()
    if getattr(sprite, "m_Name", None) != name:
        raise RuntimeError(f"Sprite name mismatch: expected {name}, got {getattr(sprite, 'm_Name', None)}")
    image = sprite.image
    output.parent.mkdir(parents=True, exist_ok=True)
    save_with_transparent_matte(image, output)
    rect = sprite.m_Rect
    texture_offset = sprite.m_RD.textureRectOffset
    pivot_x = rect.width * sprite.m_Pivot.x - texture_offset.x
    pivot_y_from_bottom = rect.height * sprite.m_Pivot.y - texture_offset.y
    return {
        "sourceRect": {"width": float(rect.width), "height": float(rect.height)},
        "textureRectOffset": {"x": float(texture_offset.x), "y": float(texture_offset.y)},
        "anchor": {
            "x": pivot_x / image.width,
            "y": (image.height - pivot_y_from_bottom) / image.height,
        },
        "pixelsPerUnit": float(sprite.m_PixelsToUnits),
    }


OUT.mkdir(parents=True, exist_ok=True)
metadata = {
    "schemaVersion": 2,
    "stage": "normalized-evidence",
    "sourceRoot": "game-assets/source/unity-assets/bin/Data",
    "transformation": {
        "id": "unitypy-sprite-export-transparent-matte-v1",
        "approximation": True,
        "note": "Near-white pixels are made transparent for web compositing; visual validation remains required.",
    },
    "foreground": [],
    "npcs": {},
    "signboards": {},
}
for key, (bundle, path_id, name) in FOREGROUND.items():
    target = OUT / "foreground" / f"{key}.png"
    render = extract_sprite(bundle, path_id, name, target)
    metadata["foreground"].append({"id": key, "source": bundle, "pathId": path_id, "sprite": name, "file": target.relative_to(ROOT).as_posix(), **render, **digest(target)})

for role, (bundle, prefix) in NPC_BUNDLES.items():
    env = UnityPy.load(str(DATA / bundle))
    frames = []
    for obj in env.objects:
        if obj.type.name != "Sprite":
            continue
        sprite = obj.read()
        name = getattr(sprite, "m_Name", "")
        if not name.startswith(prefix):
            continue
        suffix = name[len(prefix):]
        if not suffix.isdigit():
            continue
        target = OUT / "npcs" / role / f"{name}.png"
        target.parent.mkdir(parents=True, exist_ok=True)
        save_with_transparent_matte(sprite.image, target)
        frames.append({"frame": int(suffix), "name": name, "source": bundle, "pathId": obj.path_id, "file": target.relative_to(ROOT).as_posix(), **digest(target)})
    if not frames:
        raise RuntimeError(f"No sprites with prefix {prefix} in {bundle}")
    metadata["npcs"][role] = sorted(frames, key=lambda frame: frame["frame"])

for region_id, (bundle, prefix) in SIGNBOARD_BUNDLES.items():
    env = UnityPy.load(str(DATA / bundle))
    states = []
    for obj in env.objects:
        if obj.type.name != "Sprite":
            continue
        sprite = obj.read()
        name = getattr(sprite, "m_Name", "")
        if not name.startswith(prefix):
            continue
        suffix = name[len(prefix):]
        if not suffix.isdigit():
            continue
        density_level = int(suffix) + 1
        target = OUT / "signboards" / region_id / f"density-{density_level}.png"
        target.parent.mkdir(parents=True, exist_ok=True)
        save_with_transparent_matte(sprite.image, target)
        states.append({
            "densityLevel": density_level,
            "name": name,
            "source": bundle,
            "pathId": obj.path_id,
            "file": target.relative_to(ROOT).as_posix(),
            **digest(target),
        })
    states.sort(key=lambda state: state["densityLevel"])
    if [state["densityLevel"] for state in states] != [1, 2, 3]:
        raise RuntimeError(f"Density sign states are incomplete for {region_id}")
    metadata["signboards"][region_id] = states

(OUT / "manifest.json").write_text(json.dumps(metadata, indent=2) + "\n")
print(
    f"Extracted {len(metadata['foreground'])} foreground sprites, "
    f"{sum(len(v) for v in metadata['npcs'].values())} NPC frames, and "
    f"{sum(len(v) for v in metadata['signboards'].values())} signboard states"
)
