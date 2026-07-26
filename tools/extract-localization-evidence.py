#!/usr/bin/env python3
"""Extract the small, source-confirmed Unity Localization table evidence."""

import argparse
import hashlib
import json
from pathlib import Path

import UnityPy


LOCALE_BUNDLES = {
    "en": "localization-string-tables-english(en)_assets_all.bundle",
    "ja": "localization-string-tables-japanese(ja)_assets_all.bundle",
    "zh": "localization-string-tables-chinese(simplified)(zh)_assets_all.bundle",
    "zh-TW": "localization-string-tables-chinese(traditional)(zh-tw)_assets_all.bundle",
}
SHARED_BUNDLE = "localization-assets-shared_assets_all.bundle"


def digest(path):
    payload = path.read_bytes()
    return {"path": str(path), "bytes": len(payload), "sha256": hashlib.sha256(payload).hexdigest()}


def first_mono(path):
    environment = UnityPy.load(str(path))
    return next(obj.read() for obj in environment.objects if obj.type.name == "MonoBehaviour")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="game-assets/extracted/addressables_bundles")
    parser.add_argument("--output", default="reverse-engineering/evidence/localization-evidence-v1.json")
    args = parser.parse_args()
    root = Path(args.root)
    shared_path = root / SHARED_BUNDLE
    shared = first_mono(shared_path)
    entries = {int(entry.m_Id): entry.m_Key for entry in shared.m_Entries}
    if not entries:
        raise RuntimeError("Shared localization table has no entries")

    locales = {}
    for locale, filename in LOCALE_BUNDLES.items():
        path = root / filename
        table = first_mono(path)
        rows = []
        for entry in table.m_TableData:
            entry_id = int(entry.m_Id)
            if entry_id not in entries:
                raise RuntimeError(f"{locale} contains an unknown localization ID {entry_id}")
            rows.append({"id": entry_id, "key": entries[entry_id], "localized": entry.m_Localized})
        if len({row["id"] for row in rows}) != len(rows):
            raise RuntimeError(f"{locale} contains duplicate localization IDs")
        if set(row["id"] for row in rows) != set(entries):
            raise RuntimeError(f"{locale} does not cover the shared table")
        locales[locale] = {"source": digest(path), "tableName": table.m_Name, "localeCode": table.m_LocaleId.m_Code, "entries": rows}

    output = {
        "schemaVersion": 1,
        "manifestType": "unity-localization-evidence",
        "runtimeCompatibility": "evidence-only",
        "sharedTable": {"source": digest(shared_path), "tableName": shared.m_TableCollectionName, "guid": shared.m_TableCollectionNameGuidString,
                        "entries": [{"id": entry_id, "key": entries[entry_id]} for entry_id in sorted(entries)]},
        "locales": locales,
        "coverage": {"sharedKeys": len(entries), "locales": len(locales), "localeKeys": {locale: len(data["entries"]) for locale, data in locales.items()}},
        "gaps": ["Only the recovered IOS_DisplayNameTable is present in these bundles; this is not the complete in-game localization corpus.", "Font fallback, glyph coverage, rich-text behavior, and runtime table selection remain unvalidated."],
    }
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(output, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"Extracted localization evidence: {len(entries)} keys across {len(locales)} locales -> {output_path}")


if __name__ == "__main__":
    main()
