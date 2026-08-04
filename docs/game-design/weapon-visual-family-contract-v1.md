# Weapon Visual Family Contract (v1)

This document expands the weapon asset boundary beyond inventory icons. New
weapons must render consistently in inventory, crafting, the world, combat,
Hunter roster, and Hunter Info.

## Verified rendering model

The Hunter is a Spine actor. A carried weapon is a skin attachment on the
class-specific weapon slot; it is not a separate world sprite and does not need
one bitmap per animation frame.

| Class family | Spine slot | Attachment | Base skin prefix |
|---|---|---|---|
| Berserker `H1` | `weapon_01` | `sword` | `weapon_h1_` |
| Paladin `H2` | `weapon_02` | `hammer` | `weapon_h2_` |
| Ranger `H3` | `weapon_03` | `bow` | `weapon_h3_` |
| Sorcerer `H4` | `weapon_04` | `wand` | `weapon_h4_` |
| Dark Knight `H5` | `weapon_05` | `spear` | `weapon_h5_` |

The packaged attack, walk, idle, death, and revival animations move the weapon
bone and control slot visibility. Hunter roster and Hunter Info reuse the same
Spine skeleton and skin-composition path as the visible world.

The five audited starter weapons prove that the inventory icon's logical
`24 x 24` pixels are byte-for-pixel identical to the corresponding Spine atlas
attachment after reconstructing its atlas offsets. The inventory file is the
same artwork enlarged to `96 x 96` with nearest-neighbor scaling.

This exact identity is confirmed for weapon catalog indices `0`, `9`, `18`,
`27`, and `252`. It defines the rebuild pipeline; it is not an inferred mapping
for every remaining package gear index.

## Per-weapon deliverables

Each rebuild weapon requires one authoritative logical master:

```text
weapon-master/<weapon-id>.png        24 x 24 RGBA pixel art
```

Generated outputs from that master:

```text
gear-icons/<weapon-id>.png           96 x 96 nearest-neighbor icon
hunter atlas region                  trimmed logical sprite plus offsets
hunter Spine skin                    class slot + attachment + region mapping
catalog visual binding               weapon ID -> Spine skin ID
```

The same generated Spine skin is visible when the Hunter:

- stands or walks in the town/world;
- chases and attacks monsters;
- plays front/back attack clips;
- appears in the Hunter roster;
- appears in the Hunter Info tab.

No separate Hunter-tab weapon bitmap is required. The tab must render the live
equipped weapon skin rather than a fixed preview sprite.

## Optional visual assets

These are separate from the base weapon sprite and only exist when a weapon or
affix explicitly owns them:

- projectile sprite;
- trail or impact VFX;
- glow/effect attachment;
- transformation appearance;
- special proc animation/effect.

Do not bake these effects into every weapon master. Base items must remain
readable without VFX, and affix-driven visuals must be composable independently
of the base weapon.

## Generated Spine overlay

New skins should be produced as a reproducible rebuild derivative rather than
hand-editing the package bundle. The generator must:

1. pack the `24 x 24` weapon masters into a rebuild-owned atlas page;
2. append stable weapon regions and skin entries to a generated Hunter bundle;
3. preserve the package skeleton bones, slots, and animations unchanged;
4. bind each weapon skin to exactly one class-compatible slot/attachment;
5. emit a manifest connecting weapon catalog ID, icon, Spine skin, atlas region,
   source master digest, and content release.

Proposed skin IDs:

```text
rebuild_wp_berserker_000
rebuild_wp_paladin_300
rebuild_wp_ranger_700
```

## Validation matrix

Every weapon must be previewed through the actual application renderer in:

| Surface | Required check |
|---|---|
| Inventory/crafting | readable at 38, 46, 85, and 104 CSS pixels |
| World idle | correct grip, layer order, scale, and transparent padding |
| World walk | no detached or visibly orbiting weapon |
| Combat front | correct class attack clip and weapon visibility |
| Combat back | correct `_back` clip and layer order |
| Hunter roster | same equipped skin as world state |
| Hunter Info | same equipped skin, centered within actor preview |

Automated validation must also reject a weapon when its class does not match
the target Spine slot or when its declared skin/atlas/icon outputs are missing.
