# Visual Consistency Guide

This is the baseline contract for image generation. Family-specific decisions
belong in an asset brief and may refine, but must not silently contradict, this
guide.

## 1. Evidence Before Style

- Cite package evidence, migration documents, or user-provided screenshots.
- Do not infer identity, rarity, mechanics, icon meaning, or asset mapping from
  filename order or visual resemblance alone.
- Mark unknown details as `unresolved`; do not hide them with a plausible fill.
- Keep generated work visibly and operationally separate from recovered assets.

## 2. Lock A Family Before Scaling

Do not generate a large set from a single prompt. First approve a small anchor
set, then lock these fields for the whole family:

- intended in-game use and viewing size;
- canvas dimensions and safe area;
- camera angle, projection, crop, and subject scale;
- silhouette language and shape complexity;
- outline color and thickness, if any;
- palette and material treatment;
- lighting direction, softness, and shadow policy;
- detail density and texture/noise level;
- background and transparency policy.

Values remain `TBD` until supported by references and accepted by review.

## 3. Readability Rules

- Judge assets at their actual game size, not only zoomed in.
- Preserve a clear primary silhouette and one dominant focal area.
- Avoid tiny decorative detail that collapses into noise after downscaling.
- Keep adjacent gameplay categories distinguishable by shape before color.
- Do not bake labels, numbers, rarity frames, or UI chrome into an asset unless
  the brief explicitly requires them.

## 4. Composition Rules

- Use one consistent subject scale and visual center within a family.
- Keep required padding clear on every edge.
- Do not crop functional parts such as weapon tips, feet, handles, or effect
  extents unless the approved reference does so.
- Use transparent backgrounds for isolated sprites and icons; use scene
  backgrounds only when the asset itself is a complete scene or tile.

## 5. Color And Light

- Derive palettes from cited, approved references; do not invent a global game
  palette before a reference board is reviewed.
- Record colors in sRGB hex values once locked.
- Maintain value contrast in grayscale and check common color-vision failures.
- Keep light direction and shadow softness consistent within each family.
- Avoid uncontrolled bloom, color grading, chromatic aberration, and glossy 3D
  highlights unless they are part of the approved family language.

## 6. Output Quality

- Master raster output: lossless PNG, sRGB, with alpha when needed.
- Preserve the original generated master; derivatives are reproducible outputs.
- Do not repeatedly resize or recompress a working master.
- Remove stray pixels, accidental halos, clipped alpha, signatures, watermarks,
  prompt text, and generation artifacts before approval.
- Pixel-art families must use integer scaling and nearest-neighbor review.
