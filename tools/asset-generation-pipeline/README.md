# Reusable Asset Generation Pipeline

Portable profile-driven pipeline for rebuild artwork. Weapons are the first
profile; armor, helmets, monsters, buildings, UI icons, projectiles, and VFX can
use the same concept -> logical master -> delivery -> validation flow.

This folder creates asset drafts and deterministic delivery files only. It does
not change gameplay or import content into PostgreSQL.

## Contents

- `profiles/<name>/profile.json`: paths, grouping, outputs, and optional runtime binding rules.
- `profiles/<name>/catalog.json`: IDs, English/Vietnamese names, themes, logical size, and validation limits.
- `profiles/<name>/prompts.md`: generation prompt and first approval batch.
- `profiles/_template/`: copy this when adding another asset family.
- `pipeline.py`: reference-sheet, draft conversion, build, and validation commands.
- `input/<profile>/concepts/`: raw AI-generated images.
- `input/<profile>/masters/`: cleaned authoritative logical masters.
- `work/<profile>/`: generated references and temporary review files.
- `output/<profile>/`: generated delivery images, runtime masters, and bindings.

## Setup

```sh
cd tools/asset-generation-pipeline
python3 -m venv .venv
. .venv/bin/activate
pip install -r requirements.txt
```

## 1. Build the style reference

From the repository root:

```sh
python3 tools/asset-generation-pipeline/pipeline.py --profile weapons reference \
  --source apps/web/public/content/releases/evil-hunter-1.411/gear-icons
```

The result is `work/weapons/reference-sheet.png`. Supply it to the image model as a
style reference, not as an edit target.

## 2. Generate the first five concepts

Start with the level-300 batch in `profiles/weapons/prompts.md`. Save the results as:

```text
input/weapons/concepts/wp_berserker_300.png
input/weapons/concepts/wp_paladin_300.png
input/weapons/concepts/wp_ranger_300.png
input/weapons/concepts/wp_sorcerer_300.png
input/weapons/concepts/wp_dark_knight_300.png
```

Use a flat chroma-key background. If the weapon uses green, choose another key
color and pass the same value to `--chroma`.

## 3. Create a cleanup draft

```sh
python3 pipeline.py --profile weapons draft \
  --id wp_berserker_300 \
  --input input/weapons/concepts/wp_berserker_300.png \
  --chroma '#00ff00' \
  --colors 24
```

This produces `input/weapons/masters/wp_berserker_300.png`. It is only a starting
draft. Open it in Aseprite, LibreSprite, or another pixel editor and correct:

- silhouette and transparent padding;
- grip alignment;
- broken bow strings or shafts;
- noisy isolated pixels;
- outline continuity;
- palette and contrast at actual `24 x 24` size.

Do not upscale and paint the master at a different logical resolution.

## 4. Build delivery assets

```sh
python3 pipeline.py --profile weapons build --id wp_berserker_300
```

Outputs:

```text
output/weapons/gear-icons/wp_berserker_300.png       # 96x96 inventory icon
output/weapons/spine-regions/wp_berserker_300.png    # 24x24 atlas input
output/weapons/bindings/wp_berserker_300.json        # class/slot/skin binding
```

## 5. Validate

```sh
python3 pipeline.py --profile weapons validate --id wp_berserker_300
```

Run without `--id` only after all 40 masters exist:

```sh
python3 pipeline.py --profile weapons build
python3 pipeline.py --profile weapons validate
```

Validation checks dimensions, transparent corners, visible bounds, palette,
exact nearest-neighbor icon derivation, Spine-region identity, and class binding.

## Important boundary

The generated `spine-regions` files are atlas inputs. Packing them into the
Hunter atlas and appending generated skins to a rebuild Hunter bundle is a later
repository integration step. Never overwrite the original `1.411` Hunter Spine
bundle or gear icons.

## Add another asset family

```sh
python3 pipeline.py new-profile --id armor
```

Then edit `profiles/armor/profile.json`, `catalog.json`, and `prompts.md`.
Choose the family's own logical/delivery size, palette limit, visible bounds,
source reference pattern, and output folder names. The core pipeline does not
assume that every profile is a weapon or uses Spine bindings.
