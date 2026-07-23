#!/usr/bin/env python3
"""Reproducibly export embedded Unity Font payloads from sharedassets0."""

from pathlib import Path

import UnityPy


SOURCE = Path("game-assets/extracted/joined_unity_files/sharedassets0.assets")
OUTPUT = Path("game-assets/extracted/exported/fonts")
FONT_PATH_IDS = (197, 198)


environment = UnityPy.load(str(SOURCE))
objects = {object_reader.path_id: object_reader for object_reader in environment.objects}
OUTPUT.mkdir(parents=True, exist_ok=True)

for path_id in FONT_PATH_IDS:
    font = objects[path_id].read()
    payload = bytes(font.m_FontData)
    if payload[:4] not in (b"\x00\x01\x00\x00", b"OTTO"):
        raise ValueError(f"Font {font.m_Name} has an unsupported sfnt signature")
    safe_name = font.m_Name.replace("/", "_")
    output = OUTPUT / f"{safe_name}__{path_id}.ttf"
    output.write_bytes(payload)
    print(f"Extracted {font.m_Name} ({len(payload)} bytes) to {output}")
