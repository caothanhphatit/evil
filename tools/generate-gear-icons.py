#!/usr/bin/env python3
import json
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ATLAS = ROOT / "game-assets/extracted/exported/text/hunter.atlas__258.bin"
TEXTURE = ROOT / "game-assets/extracted/exported/textures/hunter__166.png"
ECONOMY = ROOT / "reverse-engineering/evidence/core-economy-tables-v1.json"
OUTPUT = ROOT / "apps/web/public/content/releases/evil-hunter-1.411/gear-icons"
SPRITES = ROOT / "game-assets/extracted/exported/sprites"


def parse_atlas():
    lines = ATLAS.read_text().splitlines()
    entries = {}
    index = 3
    while index < len(lines):
        name = lines[index].strip()
        index += 1
        if not name:
            continue
        attributes = {}
        while index < len(lines) and ":" in lines[index]:
            key, value = lines[index].strip().split(":", 1)
            attributes[key] = value.strip()
            index += 1
        if "bounds" in attributes:
            entries[name] = attributes
    return entries


def render_icon(texture, attributes):
    x, y, width, height = map(int, attributes["bounds"].split(","))
    region = texture.crop((x, y, x + width, y + height))
    if attributes.get("rotate") == "90":
        region = region.rotate(90, expand=True)
    offset_x, offset_y, original_width, original_height = map(
        int, attributes.get("offsets", f"0,0,{region.width},{region.height}").split(",")
    )
    canvas = Image.new("RGBA", (original_width, original_height))
    canvas.alpha_composite(region, (offset_x, original_height - offset_y - region.height))
    return canvas.resize((96, 96), Image.Resampling.NEAREST)


def source_sprite(prefix, number, variant=None):
    suffix = "" if variant is None else f"_{variant}"
    matches = list(SPRITES.glob(f"{prefix}_{number:02d}{suffix}__*.png"))
    if len(matches) != 1:
        raise RuntimeError(f"Expected one source sprite for {prefix} {number} {variant}, found {len(matches)}")
    return matches[0]


def standard_wear_mapping(index):
    if index <= 7:
        return index + 1, None
    if index <= 21:
        families = [9, 10, 11, 12, 9, 10, 9, 10, 9, 10, 11, 12, 11, 12]
        family = families[index - 8]
        occurrence = families[:index - 8].count(family)
        return family, None if occurrence == 0 else str(occurrence)
    family = 13 + ((index - 22) // 3)
    occurrence = (index - 22) % 3
    return family, None if occurrence == 0 else str(occurrence)


def belt_wear_mapping(index):
    if index <= 2:
        return index + 1, None
    if index in (3, 5, 6, 8):
        occurrence = (3, 5, 6, 8).index(index)
        return 4, None if occurrence == 0 else str(occurrence)
    if index in (4, 7, 9):
        occurrence = (4, 7, 9).index(index)
        return 5, None if occurrence == 0 else str(occurrence)
    family = 6 + ((index - 10) // 3)
    occurrence = (index - 10) % 3
    if family in (6, 7) and occurrence > 0:
        return family, f"0{occurrence}"
    return family, None if occurrence == 0 else str(occurrence)


def render_wear_icon(source):
    icon = Image.open(source).convert("RGBA")
    return icon.resize((96, 96), Image.Resampling.NEAREST)


def main():
    entries = parse_atlas()
    texture = Image.open(TEXTURE).convert("RGBA")
    economy = json.loads(ECONOMY.read_text())
    OUTPUT.mkdir(parents=True, exist_ok=True)
    generated = 0
    for job in range(5):
        job_rows = [row for row in economy["gearWeapons"] if row["job"] == job]
        mappings = []
        group_zero = sorted((row for row in job_rows if row["group"] == 0), key=lambda row: row["index"])
        group_one = sorted((row for row in job_rows if row["group"] == 1), key=lambda row: row["index"])
        group_two = sorted((row for row in job_rows if row["group"] == 2), key=lambda row: row["index"])
        mappings.extend((row, f"weapon/h{job + 1}_a_01") for row in group_zero)
        mappings.extend((row, f"weapon/h{job + 1}_a_{ordinal:02d}") for ordinal, row in enumerate(group_one, 2))
        mappings.extend((row, f"weapon/h{job + 1}_b_{ordinal:02d}") for ordinal, row in enumerate(group_two, 1))
        for row, attachment in mappings:
            attributes = entries.get(attachment)
            if attributes is None:
                raise RuntimeError(f"Missing proven weapon attachment: {attachment}")
            render_icon(texture, attributes).save(OUTPUT / f"weapon-{row['index']}.png")
            generated += 1

    wear_families = [
        ("armor", "gearArmor", "wear_armour"),
        ("gloves", "gearGloves", "wear_gloves"),
        ("boots", "gearBoots", "wear_shoes"),
        ("ring", "gearRing", "wear_ring"),
        ("necklace", "gearNecklace", "wear_necklace"),
    ]
    for kind, table, prefix in wear_families:
        for row in economy[table]:
            number, variant = standard_wear_mapping(row["index"])
            render_wear_icon(source_sprite(prefix, number, variant)).save(OUTPUT / f"{kind}-{row['index']}.png")
            generated += 1
    for row in economy["gearBelt"]:
        number, variant = belt_wear_mapping(row["index"])
        render_wear_icon(source_sprite("wear_belt", number, variant)).save(OUTPUT / f"belt-{row['index']}.png")
        generated += 1

    print(f"Generated {generated} source-bound gear icons in {OUTPUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
