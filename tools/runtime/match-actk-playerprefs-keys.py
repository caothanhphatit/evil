#!/usr/bin/env python3
"""Match semantic key candidates to ACTk PlayerPrefs without exporting values."""

from __future__ import annotations

import argparse
import json
import struct
import subprocess
import xml.etree.ElementTree as ET
from datetime import datetime, timezone
from pathlib import Path

import frida


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SCRIPT = ROOT / "tools/runtime/actk-playerprefs-key-match.js"
DEFAULT_PREFS = "/data/user/0/com.superplanet.evilhunter/shared_prefs/com.superplanet.evilhunter.v2.playerprefs.xml"
DEFAULT_CANDIDATES = {
    "UserData", "userData", "userdata", "USER_DATA", "user_data",
    "HunterData", "hunterData", "hunterdata", "HUNTER_DATA", "hunter_data",
    "HunterDataDic", "hunterDataDic", "mHunterData", "mHunterWaitData",
    "SaveData", "saveData", "savedata", "SAVE_DATA", "save_data",
    "GameData", "gameData", "gamedata", "PlayerData", "playerData",
    "AccountData", "accountData", "InventoryData", "inventoryData",
    "GearData", "gearData", "ItemData", "itemData", "SkillData", "skillData",
    "RidingPetData", "ridingPetData", "UserSave", "userSave", "save",
}
METADATA_SECTIONS = [
    "stringLiteral", "stringLiteralData", "string", "events", "properties",
    "methods", "parameterDefaultValues", "fieldDefaultValues",
    "fieldAndParameterDefaultValueData", "fieldMarshaledSizes", "parameters",
    "fields", "genericParameters", "genericParameterConstraints",
    "genericContainers", "nestedTypes", "interfaces", "vtableMethods",
    "interfaceOffsets", "typeDefinitions", "images", "assemblies", "fieldRefs",
    "referencedAssemblies", "attributeData", "attributeDataRange",
    "unresolvedVirtualCallParameterTypes", "unresolvedVirtualCallParameterRanges",
    "windowsRuntimeTypeNames", "windowsRuntimeStrings", "exportedTypeDefinitions",
]
KEYWORDS = ("save", "data", "user", "hunter", "player", "inventory", "gear", "item", "skill", "pet", "pref")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--adb", default="adb")
    parser.add_argument("--package", default="com.superplanet.evilhunter")
    parser.add_argument("--prefs", default=DEFAULT_PREFS)
    parser.add_argument("--script", type=Path, default=DEFAULT_SCRIPT)
    parser.add_argument("--candidate-file", type=Path)
    parser.add_argument("--metadata", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def metadata_candidates(path: Path) -> set[str]:
    data = path.read_bytes()
    magic, version = struct.unpack_from("<II", data)
    if magic != 0xFAB11BAF or version != 39:
        raise ValueError("Expected IL2CPP metadata v39")
    sections: dict[str, tuple[int, int, int]] = {}
    offset = 8
    for name in METADATA_SECTIONS:
        sections[name] = struct.unpack_from("<iii", data, offset)
        offset += 12
    literal_offset, literal_size, _ = sections["stringLiteral"]
    literal_data_offset, literal_data_size, _ = sections["stringLiteralData"]
    result: set[str] = set()
    for entry_offset in range(literal_offset, literal_offset + literal_size, 8):
        length, data_index = struct.unpack_from("<II", data, entry_offset)
        if length == 0 or length > 128 or data_index + length > literal_data_size:
            continue
        raw = data[literal_data_offset + data_index:literal_data_offset + data_index + length]
        try:
            value = raw.decode("utf-8")
        except UnicodeDecodeError:
            continue
        if all(character.isprintable() for character in value) and any(keyword in value.lower() for keyword in KEYWORDS):
            result.add(value)
    return result


def main() -> None:
    args = parse_args()
    candidates = set(DEFAULT_CANDIDATES)
    if args.candidate_file:
        candidates.update(
            line.strip()
            for line in args.candidate_file.read_text().splitlines()
            if line.strip()
        )
    if args.metadata:
        candidates.update(metadata_candidates(args.metadata))
    candidates = sorted(candidates)
    xml_text = subprocess.run(
        [args.adb, "shell", "su", "0", "cat", args.prefs],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    root = ET.fromstring(xml_text)
    entries = {
        node.attrib["name"]: len(node.text or "")
        for node in root
        if "name" in node.attrib
    }

    pid_text = subprocess.run(
        [args.adb, "shell", "pidof", args.package],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    pid = int(pid_text.split()[0])
    device = frida.get_usb_device(timeout=5)
    session = device.attach(pid)
    script = session.create_script(args.script.read_text())
    script.load()
    matches = script.exports_sync.match(candidates, entries)
    session.detach()

    # Recompute only match metadata; encrypted key names and stored values stay ephemeral.
    evidence = {
        "schemaVersion": 1,
        "contractType": "actk-playerprefs-semantic-key-match",
        "capturedAtUtc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "packageId": args.package,
        "candidateCount": len(candidates),
        "storedEntryCount": len(entries),
        "privacyPolicy": "Only matched plaintext candidates are emitted; encrypted keys and values are omitted.",
        "matches": sorted(matches, key=lambda match: match["candidate"]),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    print(f"Matched {len(matches)} of {len(candidates)} candidates")


if __name__ == "__main__":
    main()
