#!/usr/bin/env python3
import argparse
import json
import re
import shutil
from pathlib import Path

from PIL import Image, ImageColor, ImageDraw

ROOT = Path(__file__).resolve().parent
PROFILES = ROOT / "profiles"


def load_profile(profile_id):
    profile_dir = PROFILES / profile_id
    profile_path = profile_dir / "profile.json"
    if not profile_path.is_file():
        raise SystemExit(f"unknown profile: {profile_id}")
    profile = json.loads(profile_path.read_text(encoding="utf-8"))
    catalog = json.loads((profile_dir / profile["catalog"]).read_text(encoding="utf-8"))
    return profile, catalog


def item(profile, catalog, item_id):
    for row in catalog[profile["itemsKey"]]:
        if row["id"] == item_id:
            return row
    raise SystemExit(f"unknown asset id: {item_id}")


def profile_root(parent, profile):
    return ROOT / parent / profile["id"]


def alpha_bbox(image):
    return image.getchannel("A").getbbox()


def remove_chroma(image, color, tolerance):
    key = ImageColor.getrgb(color)
    pixels = []
    for red, green, blue, alpha in image.convert("RGBA").getdata():
        distance = max(abs(red - key[0]), abs(green - key[1]), abs(blue - key[2]))
        pixels.append((red, green, blue, 0 if distance <= tolerance else alpha))
    result = Image.new("RGBA", image.size)
    result.putdata(pixels)
    return result


def contain(image, size, padding):
    bbox = alpha_bbox(image)
    if not bbox:
        raise SystemExit("input image has no visible pixels")
    cropped = image.crop(bbox)
    target = max(1, size - padding * 2)
    scale = min(target / cropped.width, target / cropped.height)
    resized = cropped.resize(
        (max(1, round(cropped.width * scale)), max(1, round(cropped.height * scale))),
        Image.Resampling.LANCZOS,
    )
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    canvas.alpha_composite(resized, ((size - resized.width) // 2, (size - resized.height) // 2))
    return canvas


def quantize_rgba(image, colors):
    alpha = image.getchannel("A").point(lambda value: 255 if value >= 128 else 0)
    rgb = Image.new("RGB", image.size, (0, 0, 0))
    rgb.paste(image.convert("RGB"), mask=alpha)
    quantized = rgb.quantize(colors=max(2, colors - 1), method=Image.Quantize.MEDIANCUT).convert("RGBA")
    quantized.putalpha(alpha)
    return quantized


def command_reference(args):
    profile, catalog = load_profile(args.profile)
    source = Path(args.source)
    cell = 144
    groups = catalog[profile["groupsKey"]]
    max_columns = max((len(group.get(profile["referenceIndicesKey"], [])) for group in groups.values()), default=1)
    sheet = Image.new("RGBA", (cell * max_columns, cell * len(groups)), (28, 25, 21, 255))
    draw = ImageDraw.Draw(sheet)
    for row_index, (group_id, config) in enumerate(groups.items()):
        for column, source_index in enumerate(config.get(profile["referenceIndicesKey"], [])):
            path = source / profile["sourcePattern"].format(index=source_index)
            if not path.is_file():
                raise SystemExit(f"missing source icon: {path}")
            icon = Image.open(path).convert("RGBA").resize((112, 112), Image.Resampling.NEAREST)
            sheet.alpha_composite(icon, (column * cell + 16, row_index * cell + 22))
            draw.text((column * cell + 5, row_index * cell + 5), f"{group_id} {source_index}", fill=(238, 226, 195, 255))
    output = Path(args.output) if args.output else profile_root("work", profile) / "reference-sheet.png"
    output.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(output)
    print(output)


def command_draft(args):
    profile, catalog = load_profile(args.profile)
    item(profile, catalog, args.id)
    image = Image.open(args.input).convert("RGBA")
    if args.chroma:
        image = remove_chroma(image, args.chroma, args.tolerance)
    logical = contain(image, catalog["logicalSize"], args.padding)
    logical = quantize_rgba(logical, args.colors)
    output = profile_root("input", profile) / "masters" / f"{args.id}.png"
    output.parent.mkdir(parents=True, exist_ok=True)
    logical.save(output)
    print(f"draft master: {output}")
    print("manual pixel cleanup and grip alignment are required before build")


def build_one(profile, catalog, item_row):
    item_id = item_row["id"]
    input_root = profile_root("input", profile)
    output_root = profile_root("output", profile)
    master_path = input_root / "masters" / f"{item_id}.png"
    if not master_path.is_file():
        raise ValueError(f"missing master: {master_path}")
    master = Image.open(master_path).convert("RGBA")
    if master.size != (catalog["logicalSize"], catalog["logicalSize"]):
        raise ValueError(f"{item_id}: master must be {catalog['logicalSize']}x{catalog['logicalSize']}")
    icon_dir = output_root / profile["deliveryFolder"]
    region_dir = output_root / profile["runtimeFolder"]
    binding_dir = output_root / profile["bindingFolder"]
    for directory in (icon_dir, region_dir, binding_dir):
        directory.mkdir(parents=True, exist_ok=True)
    master.save(region_dir / f"{item_id}.png")
    master.resize((catalog["deliverySize"], catalog["deliverySize"]), Image.Resampling.NEAREST).save(
        icon_dir / f"{item_id}.png"
    )
    group_id = item_row[profile["itemGroupKey"]]
    group_config = catalog[profile["groupsKey"]][group_id]
    binding_config = profile.get("binding", {})
    binding = {
        "assetId": item_id,
        "profile": profile["id"],
        "group": group_id,
        "region": item_id,
        "delivery": f"{profile['deliveryFolder']}/{item_id}.png",
        "localization": {"en": item_row.get("en"), "vi": item_row.get("vi")},
    }
    if binding_config:
        binding.update({
            "classFamily": group_config[binding_config["familyKey"]],
            "skinId": f"{binding_config.get('skinPrefix', '')}{item_id}",
            "slot": group_config[binding_config["slotKey"]],
            "attachment": group_config[binding_config["attachmentKey"]],
        })
    (binding_dir / f"{item_id}.json").write_text(json.dumps(binding, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def command_build(args):
    profile, catalog = load_profile(args.profile)
    rows = [item(profile, catalog, args.id)] if args.id else catalog[profile["itemsKey"]]
    errors = []
    for row in rows:
        try:
            build_one(profile, catalog, row)
            print(f"built {row['id']}")
        except ValueError as error:
            errors.append(str(error))
    if errors:
        raise SystemExit("\n".join(errors))


def validate_one(profile, catalog, row):
    item_id = row["id"]
    input_root = profile_root("input", profile)
    output_root = profile_root("output", profile)
    master_path = input_root / "masters" / f"{item_id}.png"
    icon_path = output_root / profile["deliveryFolder"] / f"{item_id}.png"
    region_path = output_root / profile["runtimeFolder"] / f"{item_id}.png"
    binding_path = output_root / profile["bindingFolder"] / f"{item_id}.json"
    errors = []
    for path in (master_path, icon_path, region_path, binding_path):
        if not path.is_file():
            errors.append(f"missing {path.relative_to(ROOT)}")
    if errors:
        return errors
    master = Image.open(master_path).convert("RGBA")
    icon = Image.open(icon_path).convert("RGBA")
    region = Image.open(region_path).convert("RGBA")
    logical_size = catalog["logicalSize"]
    delivery_size = catalog["deliverySize"]
    if master.size != (logical_size, logical_size):
        errors.append(f"master is not {logical_size}x{logical_size}")
    if icon.size != (delivery_size, delivery_size):
        errors.append(f"delivery image is not {delivery_size}x{delivery_size}")
    if region.tobytes() != master.tobytes():
        errors.append("Spine region differs from master")
    expected = master.resize((delivery_size, delivery_size), Image.Resampling.NEAREST)
    if icon.tobytes() != expected.tobytes():
        errors.append("icon is not an exact 4x nearest-neighbor master")
    last = logical_size - 1
    if any(master.getpixel(point)[3] != 0 for point in ((0, 0), (last, 0), (0, last), (last, last))):
        errors.append("master corners must be transparent")
    bbox = alpha_bbox(master)
    if not bbox:
        errors.append("master is empty")
    max_bounds = catalog.get("maxVisibleBounds", logical_size)
    if bbox and (bbox[2] - bbox[0] > max_bounds or bbox[3] - bbox[1] > max_bounds):
        errors.append(f"visible bounds exceed {max_bounds}x{max_bounds}: {bbox}")
    if len(set(master.getdata())) > catalog["paletteLimit"]:
        errors.append(f"palette exceeds {catalog['paletteLimit']} RGBA colors")
    binding = json.loads(binding_path.read_text(encoding="utf-8"))
    binding_config = profile.get("binding")
    if binding_config:
        group = catalog[profile["groupsKey"]][row[profile["itemGroupKey"]]]
        if binding.get("slot") != group[binding_config["slotKey"]] or binding.get("attachment") != group[binding_config["attachmentKey"]]:
            errors.append("binding group slot/attachment mismatch")
    return errors


def command_validate(args):
    profile, catalog = load_profile(args.profile)
    rows = [item(profile, catalog, args.id)] if args.id else catalog[profile["itemsKey"]]
    failed = False
    for row in rows:
        errors = validate_one(profile, catalog, row)
        if errors:
            failed = True
            print(f"FAIL {row['id']}: " + "; ".join(errors))
        else:
            print(f"PASS {row['id']}")
    if failed:
        raise SystemExit(1)


def command_new_profile(args):
    if not re.fullmatch(r"[a-z][a-z0-9_-]*", args.id):
        raise SystemExit("profile id must use lowercase letters, numbers, underscores, or hyphens")
    target = PROFILES / args.id
    if target.exists():
        raise SystemExit(f"profile already exists: {args.id}")
    shutil.copytree(PROFILES / "_template", target)
    profile_path = target / "profile.json"
    profile = json.loads(profile_path.read_text(encoding="utf-8"))
    profile["id"] = args.id
    profile_path.write_text(json.dumps(profile, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    for path in (
        ROOT / "input" / args.id / "concepts",
        ROOT / "input" / args.id / "masters",
        ROOT / "work" / args.id,
        ROOT / "output" / args.id,
    ):
        path.mkdir(parents=True, exist_ok=True)
        (path / ".gitkeep").touch()
    print(target)


def parser():
    result = argparse.ArgumentParser(description="Reusable Evil Hunter rebuild asset-generation pipeline")
    result.add_argument("--profile", default="weapons", help="profile directory under profiles/")
    commands = result.add_subparsers(dest="command", required=True)
    reference = commands.add_parser("reference", help="build a source-style contact sheet")
    reference.add_argument("--source", required=True, help="directory containing weapon-<index>.png source icons")
    reference.add_argument("--output")
    reference.set_defaults(func=command_reference)
    draft = commands.add_parser("draft", help="convert an AI concept into a 24x24 cleanup draft")
    draft.add_argument("--id", required=True)
    draft.add_argument("--input", required=True)
    draft.add_argument("--chroma", help="flat background color such as #00ff00")
    draft.add_argument("--tolerance", type=int, default=16)
    draft.add_argument("--padding", type=int, default=2)
    draft.add_argument("--colors", type=int, default=24)
    draft.set_defaults(func=command_draft)
    build = commands.add_parser("build", help="derive icon, Spine region, and binding from cleaned masters")
    build.add_argument("--id")
    build.set_defaults(func=command_build)
    validate = commands.add_parser("validate", help="validate generated outputs")
    validate.add_argument("--id")
    validate.set_defaults(func=command_validate)
    new_profile = commands.add_parser("new-profile", help="scaffold another reusable asset family")
    new_profile.add_argument("--id", required=True)
    new_profile.set_defaults(func=command_new_profile)
    return result


if __name__ == "__main__":
    args = parser().parse_args()
    args.func(args)
