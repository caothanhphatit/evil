#!/usr/bin/env python3
"""Inventory private PlayerPrefs without exporting raw keys or values."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import xml.etree.ElementTree as ET
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path


DEFAULT_PACKAGE = "com.superplanet.evilhunter"
DEFAULT_PREFS = "com.superplanet.evilhunter.v2.playerprefs.xml"


def adb_bytes(adb: str, *args: str) -> bytes:
    return subprocess.run([adb, *args], check=True, capture_output=True).stdout


def adb_text(adb: str, *args: str) -> str:
    return adb_bytes(adb, *args).decode().strip()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--adb", default="adb")
    parser.add_argument("--package", default=DEFAULT_PACKAGE)
    parser.add_argument("--prefs", default=DEFAULT_PREFS)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    path = f"/data/user/0/{args.package}/shared_prefs/{args.prefs}"
    payload = adb_bytes(args.adb, "exec-out", "cat", path)
    root = ET.fromstring(payload)
    entries = []
    for child in root:
        value = child.text or child.attrib.get("value", "") or ""
        entries.append((child.tag, len(value)))
    type_counts = Counter(entry_type for entry_type, _ in entries)
    length_buckets = Counter()
    for _, length in entries:
        if length < 32:
            bucket = "0-31"
        elif length < 1_024:
            bucket = "32-1023"
        elif length < 65_536:
            bucket = "1024-65535"
        else:
            bucket = "65536+"
        length_buckets[bucket] += 1
    version = adb_text(args.adb, "shell", "dumpsys", "package", args.package)
    version_name = next(
        (line.split("=", 1)[1] for line in version.splitlines() if line.strip().startswith("versionName=")),
        None,
    )
    evidence = {
        "schemaVersion": 1,
        "contractType": "android-private-playerprefs-inventory",
        "capturedAtUtc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "capture": {
            "packageId": args.package,
            "versionName": version_name,
            "androidApi": adb_text(args.adb, "shell", "getprop", "ro.build.version.sdk"),
            "deviceAbi": adb_text(args.adb, "shell", "getprop", "ro.product.cpu.abi"),
            "rootUid": adb_text(args.adb, "shell", "id", "-u"),
            "sourcePath": path,
            "bytes": len(payload),
            "sha256": hashlib.sha256(payload).hexdigest(),
        },
        "inventory": {
            "entryCount": len(entries),
            "entryTypes": dict(sorted(type_counts.items())),
            "valueLengthBuckets": dict(sorted(length_buckets.items())),
            "largestValueLength": max((length for _, length in entries), default=0),
        },
        "privacy": {
            "rawKeysIncluded": False,
            "rawValuesIncluded": False,
            "accountIdentifiersIncluded": False,
        },
        "findings": [
            "The rooted API35 emulator exposes a large private Unity PlayerPrefs XML unavailable through run-as.",
            "ACTk storage schema is present; encrypted key/value semantics require runtime correlation.",
            "This artifact deliberately omits raw keys, values, account identifiers, and credentials.",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote sanitized PlayerPrefs inventory to {args.output}")


if __name__ == "__main__":
    main()
