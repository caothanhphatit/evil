#!/usr/bin/env python3
"""Export first-frame building sprites from recovered Unity animation clips.

The sprites are source-confirmed, but their town positions remain unresolved.
The generated manifest therefore labels every output as a migration candidate.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import UnityPy

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "game-assets/source/unity-assets/bin/Data"
EVIDENCE = ROOT / "reverse-engineering/evidence/building-asset-evidence-v1.json"
OUT_DIR = ROOT / "apps/web/public/content/releases/visible-world-v1/village/buildings"
OUT_MANIFEST = ROOT / "reverse-engineering/evidence/building-runtime-assets-v1.json"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    evidence = json.loads(EVIDENCE.read_text())
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    loaded: dict[str, UnityPy.Environment] = {}
    outputs = []
    candidates = [
        clip for clip in evidence["buildingAnimationAssets"]
        if clip["assetClass"] == "building-skin" and clip["spriteFrames"]
    ]
    selected_ids = {f"build_{index}" for index in range(1, 29)}
    available_ids = {clip["animationClip"] for clip in candidates}
    missing_ids = sorted(selected_ids - available_ids)
    if missing_ids:
        raise ValueError(f"missing source-confirmed core building animations: {missing_ids}")
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for stale in OUT_DIR.glob("*.png"):
        if stale.stem not in selected_ids:
            stale.unlink()
    seen: set[str] = set()
    for clip in candidates:
        if clip["animationClip"] not in selected_ids:
            continue
        frame = next((item for item in clip["spriteFrames"] if item["name"] not in seen), None)
        if frame is None:
            continue
        seen.add(frame["name"])
        bundle = frame["sourceBundle"]
        source = DATA / bundle
        if not source.exists():
            continue
        env = loaded.setdefault(bundle, UnityPy.load(str(source)))
        sprite = next((obj for obj in env.objects if obj.path_id == frame["pathId"]), None)
        if sprite is None or sprite.type.name != "Sprite":
            continue
        output = OUT_DIR / f"{clip['animationClip']}.png"
        sprite.read().image.save(output)
        outputs.append({
            "id": clip["animationClip"],
            "sourceSprite": frame["name"],
            "sourceNamespace": "runtime-extracted",
            "sourcePath": f"apps/web/public/content/releases/visible-world-v1/village/buildings/{output.name}",
            "publicPath": f"/content/releases/visible-world-v1/village/buildings/{output.name}",
            "bytes": output.stat().st_size,
            "sha256": sha256(output),
            "runtimeUse": "migration-fixture-town-candidate",
            "positionConfidence": "unresolved-runtime-placement",
        })
    OUT_MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    OUT_MANIFEST.write_text(json.dumps({
        "schemaVersion": 1,
        "runtimeUse": "migration-fixture-town-candidate",
        "evidence": "reverse-engineering/evidence/building-asset-evidence-v1.json",
        "placementPolicy": "candidate-grid-only-until-runtime-telemetry-resolves-town-coordinates",
        "buildings": sorted(outputs, key=lambda item: item["id"]),
    }, indent=2) + "\n")
    print(f"Exported {len(outputs)} source-confirmed building sprites to {OUT_DIR}")


if __name__ == "__main__":
    main()
