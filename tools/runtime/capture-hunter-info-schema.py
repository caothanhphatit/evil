#!/usr/bin/env python3
"""Capture the Hunter IL2CPP schema into a reproducible JSON evidence file."""

from __future__ import annotations

import argparse
import json
import subprocess
import threading
import typing
from datetime import datetime, timezone
from pathlib import Path

import typing_extensions

for compatibility_name in ("NotRequired", "Required", "ParamSpec"):
    if not hasattr(typing, compatibility_name):
        setattr(typing, compatibility_name, getattr(typing_extensions, compatibility_name))

import frida


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SCRIPT = ROOT / "tools/runtime/hunter-info-runtime-dump.js"
DEFAULT_PACKAGE = "com.superplanet.evilhunter"
DEFAULT_ACTIVITY = "com.google.firebase.MessagingUnityPlayerActivity"


def run_adb(adb: str, *args: str, check: bool = True) -> str:
    result = subprocess.run(
        [adb, *args],
        check=check,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def package_version(adb: str, package: str) -> dict[str, str | None]:
    output = run_adb(adb, "shell", "dumpsys", "package", package)
    values: dict[str, str | None] = {"versionName": None, "versionCode": None}
    for line in output.splitlines():
        stripped = line.strip()
        if stripped.startswith("versionName="):
            values["versionName"] = stripped.split("=", 1)[1]
        elif stripped.startswith("versionCode="):
            values["versionCode"] = stripped.split("=", 1)[1].split()[0]
    return values


def wait_for_pid(adb: str, package: str, attempts: int = 40) -> int:
    for _ in range(attempts):
        value = run_adb(adb, "shell", "pidof", package, check=False)
        if value:
            return int(value.split()[0])
        threading.Event().wait(0.25)
    raise RuntimeError(f"Timed out waiting for process {package}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--adb", default="adb", help="ADB executable")
    parser.add_argument("--package", default=DEFAULT_PACKAGE)
    parser.add_argument("--activity", default=DEFAULT_ACTIVITY)
    parser.add_argument("--script", type=Path, default=DEFAULT_SCRIPT)
    parser.add_argument(
        "--target-assembly",
        default="Assembly-CSharp.dll",
        help="Managed IL2CPP assembly image to inspect",
    )
    parser.add_argument(
        "--target-type",
        action="append",
        default=[],
        help="Simple type name in the target assembly; repeat to override the default targets",
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--action", required=True, help="Exact user/runtime action for this capture")
    parser.add_argument("--pid", type=int, help="Attach to an already-running process")
    parser.add_argument(
        "--attach-delay",
        type=float,
        default=4.0,
        help="Seconds to wait after resolving the process before attaching",
    )
    parser.add_argument("--timeout", type=float, default=15.0)
    parser.add_argument(
        "--launch",
        action="store_true",
        help="Force-stop and launch the package before attaching",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    script_path = args.script.resolve()
    if not script_path.is_file():
        raise FileNotFoundError(script_path)

    if args.launch:
        run_adb(args.adb, "shell", "am", "force-stop", args.package)
        component = f"{args.package}/{args.activity}"
        run_adb(args.adb, "shell", "am", "start", "-n", component)

    pid = args.pid or wait_for_pid(args.adb, args.package)
    if args.attach_delay > 0:
        threading.Event().wait(args.attach_delay)
    device = frida.get_usb_device(timeout=5)
    session = device.attach(pid)
    completed = threading.Event()
    captured: dict[str, object] = {}

    def on_message(message: dict[str, object], data: bytes | None) -> None:
        if message.get("type") == "send":
            record = message.get("payload")
            if isinstance(record, dict) and record.get("kind") in {
                "hunter-info-schema",
                "hunter-info-schema-error",
            }:
                captured["record"] = record
                completed.set()
        elif message.get("type") == "error":
            captured["fridaError"] = message
            completed.set()

    script_source = script_path.read_text()
    configured_assembly = json.dumps(args.target_assembly, ensure_ascii=True)
    script_source = (
        f"globalThis.HUNTER_SCHEMA_TARGET_ASSEMBLY = {configured_assembly};\n"
        f"{script_source}"
    )
    if args.target_type:
        configured_targets = json.dumps(args.target_type, ensure_ascii=True)
        script_source = f"globalThis.HUNTER_SCHEMA_TARGET_TYPES = {configured_targets};\n{script_source}"
    script = session.create_script(script_source)
    script.on("message", on_message)
    script.load()
    if not completed.wait(args.timeout):
        raise TimeoutError(f"No Hunter schema record received within {args.timeout:g}s")

    record = captured.get("record")
    if not isinstance(record, dict):
        raise RuntimeError(json.dumps(captured, ensure_ascii=True))

    version = package_version(args.adb, args.package)
    evidence = {
        "schemaVersion": 1,
        "contractType": "hunter-info-runtime-schema-evidence",
        "runtimeCompatibility": "evidence-only",
        "capture": {
            "capturedAtUtc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
            "packageId": args.package,
            **version,
            "deviceAbi": run_adb(args.adb, "shell", "getprop", "ro.product.cpu.abi"),
            "deviceModel": run_adb(args.adb, "shell", "getprop", "ro.product.model"),
            "androidRelease": run_adb(args.adb, "shell", "getprop", "ro.build.version.release"),
            "androidApi": run_adb(args.adb, "shell", "getprop", "ro.build.version.sdk"),
            "fridaClientVersion": frida.__version__,
            "fridaServerVersion": run_adb(
                args.adb,
                "shell",
                "/data/local/tmp/frida-server",
                "--version",
            ),
            "pid": pid,
            "action": args.action,
            "script": script_path.relative_to(ROOT).as_posix(),
            "requestedTargetTypes": args.target_type or None,
            "requestedTargetAssembly": args.target_assembly,
        },
        "record": record,
        "limitations": [
            "This capture enumerates IL2CPP schema metadata and does not read live Hunter object values.",
            "Field-name resemblance alone does not prove a UI, save, or gameplay binding.",
            "A value binding requires controlled typed before/after captures for one action at a time.",
        ],
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote {record.get('kind')} evidence to {args.output}")


if __name__ == "__main__":
    main()
