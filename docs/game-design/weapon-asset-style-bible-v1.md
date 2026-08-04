# Weapon Asset Style Bible (v1)

This document defines the rebuild weapon-icon style and integration boundary.
The original exported icons remain immutable evidence. New assets must live in
a separate rebuild content release.

## Verified source characteristics

The currently integrated weapon icons were audited from
`apps/web/public/content/releases/evil-hunter-1.411/gear-icons/weapon-*.png`.

- File format: RGBA PNG.
- Delivery canvas: `96 x 96` pixels.
- Logical artwork canvas: `24 x 24` pixels.
- Every audited icon is an exact nearest-neighbor `4x` enlargement; every
  logical pixel occupies a uniform `4 x 4` delivery block.
- Alpha values in the current sample are `0`, `235`, and `255`.
- Individual icons use approximately 12-41 RGBA colors including transparency.
- Visible artwork normally occupies 13-20 logical pixels per axis, leaving
  generous transparent padding.
- Client presentation uses `object-fit: contain` and
  `image-rendering: pixelated` at sizes from roughly 38 to 104 CSS pixels.

## Visual language

- Compact mobile-game pixel art, not painterly concept art and not
  photorealistic rendering.
- Hard pixel clusters with no resampling blur or subpixel geometry.
- Dark outline or darkest-material edge establishes the silhouette.
- Two to four material-value steps are preferred over smooth gradients.
- Highlights are small, deliberate clusters; emissive accents remain restrained
  so the weapon reads against both dark blue and brown inventory frames.
- No baked rarity frame, star, text, character, hand, floor, cast shadow,
  reflection, or background.
- Detail must survive display at `38 x 38`; silhouette carries identity before
  engraving or ornament.

## Class grammar

### Berserker

- Sword or axe arranged diagonally, usually head/blade toward upper-left and
  grip toward lower-right.
- Broad blade mass and visibly heavy head.
- Higher tiers increase silhouette complexity, not just glow intensity.

### Paladin

- Hammer, maul, or mace with a clearly separated head and shaft.
- Strong geometric head shapes; holy/celestial tiers use balanced forms rather
  than thin ornamental filigree.

### Ranger

- Bow is nearly vertical with a mild diagonal lean.
- String, grip, upper limb, and lower limb must remain readable at logical
  resolution.
- Preserve open negative space inside the bow instead of filling it with VFX.

### Sorcerer

- Staff is diagonal from lower-right to upper-left.
- Magical focus occupies only a compact area near the head.
- Alternate focus silhouettes across orb, fork, ring, eye, crystal, and crown
  forms; avoid repeating one crystal-on-a-stick template.

### Dark Knight

- Spear/glaive silhouette follows the diagonal staff grammar but uses a longer,
  sharper head and darker shaft.
- The blade, neck, shaft, and counterweight must remain separable at icon size.

## Generation workflow

Image generation is a concept stage, not the final export stage:

1. Build one reference contact sheet from the immutable source icons.
2. Generate a class concept sheet using the source sheet only as style and
   silhouette reference.
3. Select one concept and reconstruct it on a `24 x 24` logical pixel grid.
4. Reduce palette and hand-correct pixel clusters, outline continuity, and
   transparent padding.
5. Enlarge to `96 x 96` using nearest-neighbor only.
6. Validate dimensions, alpha, exact `4 x 4` block consistency, bounds, palette,
   and readability at all current UI sizes.
7. Save under a rebuild release; never overwrite package-derived icons.

Directly resizing an AI-generated illustration into the delivery canvas is not
allowed because it produces antialiasing, noisy clusters, weak silhouettes, and
icons that do not integrate with `image-rendering: pixelated`.

## Integration contract

Each weapon has one authoritative `24 x 24` logical master. The `96 x 96`
inventory icon and the Spine attachment are generated from that same master as
defined in `weapon-visual-family-contract-v1.md`.

Proposed release root:

```text
apps/web/public/content/releases/evil-hunter-rebuild-weapons-v1/gear-icons/
```

Filename examples:

```text
wp_berserker_000.png
wp_paladin_300.png
wp_ranger_700.png
```

Required checks for each final asset:

- exactly `96 x 96` RGBA PNG;
- transparent canvas with no opaque corner pixels;
- exact `4x` nearest-neighbor block structure;
- logical visible bounds no larger than `20 x 20` unless an explicit review
  approves the silhouette;
- no localized text embedded in the bitmap;
- stable ID-based filename matching the weapon catalog;
- previewed at `38`, `46`, `85`, and `104` CSS pixels on existing inventory
  backgrounds.

## Initial generation batch

Do not generate all 40 final icons at once. The first approval batch should be
one weapon per class at level 300, because that tier has enough material and
energy detail to reveal style drift without the visual excess of endgame gear.
After those five pass pixel and UI validation, generate the remaining seven
bases per class using the accepted class silhouette language.
