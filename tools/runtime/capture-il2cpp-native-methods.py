#!/usr/bin/env python3
"""Capture bounded native code prefixes for selected IL2CPP methods."""

from __future__ import annotations

import argparse
import json
import subprocess
import threading
import typing
from datetime import datetime, timezone
from pathlib import Path

import typing_extensions

# Apple's system Python 3.9 lacks typing.NotRequired, while current Frida's
# runtime annotations import it from typing.
for compatibility_name in ("NotRequired", "Required", "ParamSpec"):
    if not hasattr(typing, compatibility_name):
        setattr(typing, compatibility_name, getattr(typing_extensions, compatibility_name))

import frida


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SCRIPT = ROOT / "tools/runtime/il2cpp-native-method-dump.js"
DEFAULT_PACKAGE = "com.superplanet.evilhunter"
DEFAULT_ACTIVITY = "com.google.firebase.MessagingUnityPlayerActivity"


def run_adb(adb: str, *args: str, check: bool = True) -> str:
    result = subprocess.run([adb, *args], check=check, capture_output=True, text=True)
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


def wait_for_pid(adb: str, package: str, attempts: int = 60) -> int:
    for _ in range(attempts):
        value = run_adb(adb, "shell", "pidof", package, check=False)
        if value:
            return int(value.split()[0])
        threading.Event().wait(0.1)
    raise RuntimeError(f"Timed out waiting for process {package}")


def parse_method(value: str) -> dict[str, object]:
    parts = value.split(":")
    if len(parts) not in {2, 3} or not parts[0] or not parts[1]:
        raise argparse.ArgumentTypeError("method must be Class:Method or Class:Method:ParameterCount")
    method: dict[str, object] = {"className": parts[0], "methodName": parts[1]}
    if len(parts) == 3:
        try:
            method["parameterCount"] = int(parts[2])
        except ValueError as error:
            raise argparse.ArgumentTypeError("parameter count must be an integer") from error
    return method


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--adb", default="adb")
    parser.add_argument("--package", default=DEFAULT_PACKAGE)
    parser.add_argument("--activity", default=DEFAULT_ACTIVITY)
    parser.add_argument("--script", type=Path, default=DEFAULT_SCRIPT)
    parser.add_argument("--target-assembly", default="Assembly-CSharp.dll")
    parser.add_argument("--method", action="append", default=[], type=parse_method)
    parser.add_argument(
        "--class-all-methods",
        action="append",
        default=[],
        help="Capture every managed method pointer for this class",
    )
    parser.add_argument(
        "--module-offset",
        action="append",
        default=[],
        type=lambda value: int(value, 0),
        help="Resolve every managed method whose live libil2cpp module offset matches this value",
    )
    parser.add_argument("--code-bytes", type=int, default=192)
    parser.add_argument("--exact-boundaries", action="store_true")
    parser.add_argument(
        "--include-method-index",
        action="store_true",
        help="Record a compact Assembly-CSharp method-to-module-offset index",
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--action", required=True)
    parser.add_argument("--pid", type=int)
    parser.add_argument("--launch", action="store_true")
    parser.add_argument("--attach-delay", type=float, default=0.25)
    parser.add_argument("--timeout", type=float, default=12.0)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if not args.method and not args.module_offset and not args.class_all_methods and not args.include_method_index:
        raise ValueError("at least one --method, --module-offset, or --class-all-methods is required")
    if args.code_bytes < 0 or args.code_bytes > 65536:
        raise ValueError("--code-bytes must be between 0 and 65536")
    script_path = args.script.resolve()
    if not script_path.is_file():
        raise FileNotFoundError(script_path)

    if args.launch:
        run_adb(args.adb, "shell", "am", "force-stop", args.package)
        run_adb(args.adb, "shell", "am", "start", "-n", f"{args.package}/{args.activity}")

    pid = args.pid or wait_for_pid(args.adb, args.package)
    if args.attach_delay > 0:
        threading.Event().wait(args.attach_delay)

    device = frida.get_usb_device(timeout=5)
    session = device.attach(pid)
    completed = threading.Event()
    captured: dict[str, object] = {}

    def on_message(message: dict[str, object], data: bytes | None) -> None:
        del data
        if message.get("type") == "send":
            record = message.get("payload")
            if isinstance(record, dict) and record.get("kind") in {
                "il2cpp-native-methods",
                "il2cpp-native-methods-error",
            }:
                captured["record"] = record
                completed.set()
        elif message.get("type") == "error":
            captured["fridaError"] = message
            completed.set()

    configured = (
        f"globalThis.IL2CPP_NATIVE_TARGET_ASSEMBLY = {json.dumps(args.target_assembly)};\n"
        f"globalThis.IL2CPP_NATIVE_TARGET_METHODS = {json.dumps(args.method)};\n"
        f"globalThis.IL2CPP_NATIVE_TARGET_CLASSES = {json.dumps(args.class_all_methods)};\n"
        f"globalThis.IL2CPP_NATIVE_TARGET_MODULE_OFFSETS = {json.dumps(args.module_offset)};\n"
        f"globalThis.IL2CPP_NATIVE_CODE_BYTE_LIMIT = {args.code_bytes};\n"
        f"globalThis.IL2CPP_NATIVE_EXACT_BOUNDARIES = {json.dumps(args.exact_boundaries)};\n"
        f"globalThis.IL2CPP_NATIVE_INCLUDE_METHOD_INDEX = {json.dumps(args.include_method_index)};\n"
    )
    script = session.create_script(configured + script_path.read_text())
    script.on("message", on_message)
    script.load()
    if not completed.wait(args.timeout):
        raise TimeoutError(f"No native-method record received within {args.timeout:g}s")

    record = captured.get("record")
    if not isinstance(record, dict):
        raise RuntimeError(json.dumps(captured, ensure_ascii=True))

    evidence = {
        "schemaVersion": 1,
        "contractType": "il2cpp-native-method-evidence",
        "runtimeCompatibility": "evidence-only",
        "capture": {
            "capturedAtUtc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
            "packageId": args.package,
            **package_version(args.adb, args.package),
            "deviceAbi": run_adb(args.adb, "shell", "getprop", "ro.product.cpu.abi"),
            "deviceModel": run_adb(args.adb, "shell", "getprop", "ro.product.model"),
            "androidRelease": run_adb(args.adb, "shell", "getprop", "ro.build.version.release"),
            "androidApi": run_adb(args.adb, "shell", "getprop", "ro.build.version.sdk"),
            "fridaClientVersion": frida.__version__,
            "fridaServerVersion": run_adb(
                args.adb, "shell", "/data/local/tmp/frida-server", "--version"
            ),
            "pid": pid,
            "action": args.action,
            "script": script_path.relative_to(ROOT).as_posix(),
            "requestedMethods": args.method,
            "requestedClasses": args.class_all_methods,
            "requestedModuleOffsets": [f"0x{value:x}" for value in args.module_offset],
            "requestedTargetAssembly": args.target_assembly,
            "codeByteLimit": args.code_bytes,
            "exactBoundaries": args.exact_boundaries,
            "includeMethodIndex": args.include_method_index,
        },
        "record": record,
        "limitations": [
            "MethodInfo pointer fields are version-sensitive; both leading candidates are retained.",
            "Without --exact-boundaries, a bounded native prefix may not contain every branch or inlined dependency.",
            "Disassembly and behavioral tests are required before porting a formula to Rust.",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote {record.get('kind')} evidence to {args.output}")


if __name__ == "__main__":
    main()
