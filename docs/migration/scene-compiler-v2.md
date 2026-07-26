# Level1 Scene Evidence Compiler v2

`tools/extract-level1-scene.py` compiles the joined Unity `level1` file into `reverse-engineering/evidence/level1-scene-evidence-v2.json`. The output conforms to `packages/content/level1-scene-evidence-v2.schema.json` and explicitly sets `runtimeCompatibility` to `not-claimed`.

## Reproducible tool environment

```bash
python3 -m venv /tmp/evil-scene-venv
/tmp/evil-scene-venv/bin/python -m pip install -r tools/requirements-scene.lock
/tmp/evil-scene-venv/bin/python tools/extract-level1-scene.py
node tools/validate-level1-scene-evidence.mjs
python3 -m unittest tools.tests.test_scene_evidence
```

The lock pins UnityPy, Pillow, and all current transitive packages. The evidence manifest records the exact UnityPy version and SHA-256 of the joined source.

## Current coverage

- 23,286 GameObjects and all 23,286 Transform/RectTransform records, including local quaternion rotation, scale, hierarchy, anchors, pivot, and size.
- 5,364 SpriteRenderers, including resolved sprite/material PPtrs, color, sorting layer/order, flips, and draw mode.
- 16 Canvases, 5 Cameras, 5,429 Animators, 78 2D colliders, 23 TextMeshes, and 4 CanvasGroups.
- 15,905 UI MonoBehaviour headers: 9,320 Image, 1,950 Button, 4,608 Text, 13 CanvasScaler, and 14 GraphicRaycaster records.

## Explicit gaps

The 15,905 UI records are header-only because the joined scene does not include the external type trees required to decode their serialized payloads safely. Each record has a matching diagnostic rather than an invented sprite, text, event, or layout binding. Animator controller graphs remain references only. The compiler does not recover runtime-mutated state, dynamic spawns, navigation, gameplay values, or interaction behavior.
