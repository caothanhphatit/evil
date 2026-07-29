# Town Concept V01 Generation Record

- Status: `superseded`
- Evidence label: `rebuild-original`
- Generator: `OpenAI Image API CLI`
- Model: `gpt-image-2`
- Quality: `high`
- Size: `2160x3840`
- Output: `design/asset-studio/work-in-progress/town-v1/town-concept--overview--day--v01.png`
- Seed: `unavailable`

## Prompt

```text
Use case: stylized-concept
Asset type: polished game environment concept for a portrait mobile town-management scene
Primary request: create an original, improved fantasy hunter town that preserves the cozy, playful management-game vibe and compact visual proportions of the supplied Evil Hunter web rebuild references, while increasing clarity, craftsmanship, material richness, and environmental cohesion
Scene/backdrop: a dense medieval-fantasy town inside a defended settlement, with compact timber-and-stone service buildings, workshops, an inn, storage structures, winding dirt and stone paths, small garden plots, trees, fences, crates, banners, lamps, and lived-in decoration; surrounding walls and a clear entrance gate; distant terrain only as subtle framing
Subject: the entire town is the subject, with a readable central circulation space and many distinct building silhouettes; a few tiny stylized NPC and hunter figures may provide scale without becoming focal characters
Style/medium: polished hand-painted 2D game environment illustration, charming stylized proportions, rounded but sturdy architecture, clean readable shapes, restrained painterly texture, premium mobile-game finish; preserve the approachable chibi fantasy-management feeling without copying an exact town layout
Composition/framing: strict portrait 9:16 canvas, high three-quarter isometric/top-down game camera, town fills the viewport, dense but navigable building placement, clear paths connecting every structure, balanced visual weight from top to bottom, no empty letterbox areas, safe outer margins for later HUD overlay, actual gameplay-like overview rather than cinematic landscape concept art
Lighting/mood: warm clear daytime, soft upper-left sunlight, gentle ambient shadows, welcoming and industrious mood, strong value separation between paths, roofs, vegetation, and interactable buildings
Color palette: warm ochre and terracotta roofs, honey-brown timber, pale weathered stone, muted natural greens, small controlled teal and red accents; cohesive saturation with no neon colors
Materials/textures: readable wood beams, shingles, plaster, stone foundations, packed earth, grass tufts, cloth banners, metal workshop details; textures remain simple enough to read after mobile downscaling
Constraints: preserve portrait 9:16 framing and the reference family's small-building proportions; make silhouettes readable at mobile size; every building must sit naturally on the ground and connect to the path network; environment only, no UI; no title; no labels; no logos; no watermark; no signatures; no photorealism; no 3D render look
Avoid: exact duplication of any existing game screenshot or proprietary town layout; giant castle focal point; empty plaza; scattered disconnected buildings; excessive bloom; dramatic fog; night lighting; cyberpunk elements; realistic human proportions; illegible micro-detail; warped architecture; floating objects; fake text; interface panels
```

## Negative Prompt

```text
UI, HUD, text, logo, watermark, signature, photorealistic, realistic people,
cinematic low angle, side view, flat orthographic map, giant castle, modern city,
sci-fi, neon, dark horror, night scene, heavy fog, excessive bloom, empty terrain,
isolated buildings, broken paths, warped roofs, floating props, unreadable clutter
```

## Execution

This prompt produced the wrong asset type: a completed hand-painted town rather
than an empty pixel-art base. Do not reuse it. The corrected generation prompt
is `town-base-empty-v02.prompt.txt`. Generation uses the model-compatible
`1152x1264` size, then center-crops deterministically to `1140x1260`.
