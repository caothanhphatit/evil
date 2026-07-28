#!/usr/bin/env python3
"""Normalize the recovered Hunter attack-speed producer and timer chain."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "reverse-engineering/evidence"
DEFAULT_CAPTURE = EVIDENCE / "original-native-hunter-attack-speed-producers-api35-v1.json"
DEFAULT_SCAN = EVIDENCE / "original-native-hunter-attack-speed-field-scan-api35-v1.json"
DEFAULT_RESOLUTIONS = EVIDENCE / "original-native-hunter-attack-speed-offset-resolutions-api35-v1.json"
DEFAULT_STATUS_SCHEMA = EVIDENCE / "status-data-runtime-schema-android-api35-v1.json"
DEFAULT_HUNTER_SCHEMA = EVIDENCE / "evil-ai-drop-runtime-schema-android-api35-v1.json"
DEFAULT_MANAGER_SCHEMA = EVIDENCE / "hunter-manager-runtime-schema-android-api30-v1.json"
DEFAULT_USER_SCHEMA = EVIDENCE / "hunter-info-runtime-schema-android-api35-v1.json"
DEFAULT_OUTPUT = EVIDENCE / "original-native-hunter-attack-speed-producer-chain-v2.json"

EXPECTED_METHODS = {
    ("StatusData", "FGCEFJCHNCK", 1): ("0x2d5e1ac", 76),
    ("StatusData", "COJNMPDBOOO", 0): ("0x2d5e1f8", 880),
    ("StatusData", "set_CalcAttackSpeed", 1): ("0x2d6ed80", 20),
    ("HunterCtrl", "FixedUpdate", 0): ("0x340fcf8", 6800),
    ("HunterCtrl", "NICAFPDFNPG", 0): ("0x34195fc", 128),
    ("HunterCtrl", "BuffEndSetting", 1): ("0x344a120", 3288),
    ("HunterCtrl", "CKKBPHNBKLC", 3): ("0x3459f20", 1896),
    ("HunterCtrl", "Init", 3): ("0x345d294", 1856),
    ("HunterCtrl", "BuffSetting", 4): ("0x345f700", 12656),
    ("HunterCtrl", "Init", 2): ("0x3456e6c", 1808),
    ("HunterCtrl", ".ctor", 0): ("0x3463e68", 2056),
}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source(path: Path) -> dict[str, str]:
    return {"path": path.relative_to(ROOT).as_posix(), "sha256": digest(path)}


def method_map(capture: dict[str, Any]) -> dict[tuple[str, str, int], dict[str, Any]]:
    return {
        (method["className"], method["methodName"], method["parameterCount"]): method
        for method in capture["record"]["payload"]["methods"]
    }


def exact_body(method: dict[str, Any]) -> tuple[dict[str, Any], bytes]:
    candidate = method["candidates"][0]
    body = bytes.fromhex(candidate["codeHex"])
    if candidate["codeTruncated"] or len(body) != candidate["nativeSizeBytes"]:
        raise ValueError(f"{method['className']}.{method['methodName']} is not exact")
    return candidate, body


def require_words(body: bytes, offset: int, expected: list[int], label: str) -> None:
    actual = [
        int.from_bytes(body[index : index + 4], "little")
        for index in range(offset, offset + len(expected) * 4, 4)
    ]
    if actual != expected:
        raise ValueError(f"{label} changed at method offset 0x{offset:x}")


def fields(schema: dict[str, Any], class_name: str) -> dict[int, dict[str, Any]]:
    cls = next(row for row in schema["record"]["payload"]["classes"] if row["name"] == class_name)
    return {field["offset"]: field for field in cls["fields"]}


def attack_speed(
    weapon_speed: float,
    personal: float,
    option: float,
    rank: float,
    guild: float,
    gup_7: float,
    riding_pet: float,
) -> float:
    return weapon_speed * (1.0 + 0.01 * (personal + option + rank - guild - gup_7 - riding_pet))


def calc_attack_speed(
    attack_speed_value: float,
    personal: float,
    quicken: float,
    fury: float,
    speed_potion: float,
) -> float:
    denominator = quicken + fury + speed_potion if fury > 1.0 else quicken + speed_potion + personal
    return max(0.25, attack_speed_value / denominator)


def build(
    capture_path: Path,
    scan_path: Path,
    resolutions_path: Path,
    status_schema_path: Path,
    hunter_schema_path: Path,
    manager_schema_path: Path,
    user_schema_path: Path,
) -> dict[str, Any]:
    capture = json.loads(capture_path.read_text())
    scan = json.loads(scan_path.read_text())
    resolutions = json.loads(resolutions_path.read_text())
    status_schema = json.loads(status_schema_path.read_text())
    hunter_schema = json.loads(hunter_schema_path.read_text())
    manager_schema = json.loads(manager_schema_path.read_text())
    user_schema = json.loads(user_schema_path.read_text())
    methods = method_map(capture)
    bodies: dict[tuple[str, str, int], bytes] = {}
    candidates: dict[tuple[str, str, int], dict[str, Any]] = {}

    for key, (module_offset, native_size) in EXPECTED_METHODS.items():
        candidate, body = exact_body(methods[key])
        if candidate["moduleOffset"] != module_offset or candidate["nativeSizeBytes"] != native_size:
            raise ValueError(f"native boundary changed for {key}")
        candidates[key] = candidate
        bodies[key] = body

    producer = bodies[("StatusData", "COJNMPDBOOO", 0)]
    require_words(
        producer,
        0x178,
        [0x1E2A2921, 0x90FEFE68, 0x1E2E1009, 0xAA1F03E0, 0x1E2B2821, 0x1E2C3821, 0x1E2D3821, 0x1E203820, 0xBD4C8D01, 0x9104F3E8, 0x1E210800, 0x1E292800, 0x1E200900],
        "AttackSpeed arithmetic",
    )
    require_words(
        producer,
        0x2B8,
        [0x1E2A2900, 0xB94012A8, 0xAA1F03E1, 0xB9001008, 0x1E292808, 0x3DC002A0, 0x3D800000, 0x97DBE070, 0x1E281800],
        "CalcAttackSpeed division",
    )
    require_words(
        producer,
        0x30C,
        [0x1E2A1001, 0x1E212000, 0x54000125, 0x1E2A1000, 0x910033E8, 0xAA1F03E0, 0x97DBE4D8, 0x3CC0C3E0, 0xB9401FE8, 0x3D8002A0, 0xB90012A8, 0x3DC002A0, 0xB94012A8, 0x3C888260, 0xB9009A68],
        "CalcAttackSpeed 0.25 clamp and write",
    )

    fury_setter = bodies[("StatusData", "FGCEFJCHNCK", 1)]
    require_words(
        fury_setter,
        0x14,
        [0xBD4C8D01, 0x910033E8, 0x1E210800, 0x97DBE5AC, 0x3CC0C3E0, 0xB9401FE9, 0x911FE268, 0xAA1303E0, 0x3D800100, 0xB9080A69, 0x94000004],
        "FuryValue percent writer and producer call",
    )

    fixed_update = bodies[("HunterCtrl", "FixedUpdate", 0)]
    require_words(
        fixed_update,
        0x1A8,
        [0x91065278, 0xB941A668, 0x910643E0, 0x3DC00300, 0xAA1F03E1, 0xB901A3E8, 0x3D8067E0, 0x97C119F4, 0x1E202008, 0x54000065, 0x2F00E400, 0x14000015],
        "mAttackDelay initial decode gate",
    )
    require_words(
        fixed_update,
        0x218,
        [0xAA1F03E0, 0x1E204008, 0x9499E37B, 0x1E203900, 0x9107A3E8, 0xAA1F03E0, 0x97C11E55, 0x3DC002A0, 0xB941FBE8, 0x3D800300, 0xB9001308],
        "mAttackDelay delta-time decrement and store",
    )

    buff_setting = bodies[("HunterCtrl", "BuffSetting", 4)]
    require_words(buff_setting, 0x8D4, [0x34006B75, 0x710006BF, 0x540089C1], "BuffSetting type-zero dispatch")
    require_words(
        buff_setting,
        0x16C4,
        [0xAA1303E0, 0x97FE5ACE, 0xB400BCE0, 0xBD401E80, 0xAA1F03E1, 0x5E21D800, 0x97E3F4F4, 0xBD401E80, 0xD0FEC648, 0xAA1F03E0, 0xBD4C8D0A, 0x910183E8, 0x5E21D800, 0x1E2A0800, 0x97BFDAA0, 0x3DC01BE0, 0xB94073E9, 0x911AB268, 0x911243E0, 0xAA1F03E1, 0x3D800100, 0xB906BE69],
        "BuffSetting type-zero Fury and BCE writers",
    )

    buff_end = bodies[("HunterCtrl", "BuffEndSetting", 1)]
    require_words(buff_end, 0x2BC, [0x34001874, 0x7100069F, 0x54000E20], "BuffEndSetting type-zero dispatch")
    require_words(
        buff_end,
        0x604,
        [0xAA1303E0, 0x97FEB476, 0xB4001BC0, 0x2F00E400, 0xAA1F03E1, 0x97E44E9D, 0x1E2E1000, 0x9100F3E8, 0xAA1F03E0, 0x97C0344D, 0x3CC3C3E0, 0x911AB268, 0xB9404FE9, 0xAA1F03E0, 0x3D800100, 0x1E2E1000, 0x9100A3E8, 0xB906BE69],
        "BuffEndSetting Fury clear and BCE reset",
    )

    scan_findings = scan["findings"]
    dan_writers = [
        row
        for row in scan_findings
        if row["className"] == "HunterCtrl"
        and any(access["field"] == "DANCPPLMKIK" and access["access"] == "write" for access in row["accesses"])
    ]
    if scan["methodsScanned"] != {"StatusData": 128, "HunterCtrl": 391} or dan_writers:
        raise ValueError("class-wide DANCPPLMKIK writer scan changed")

    bce_writers = {
        (row["methodName"], row["token"])
        for row in scan_findings
        if row["className"] == "HunterCtrl"
        and any(access["field"] == "BCEBGLKCDHN" and access["access"] == "write" for access in row["accesses"])
    }
    expected_bce_writers = {
        ("BuffEndSetting", "0x06005C05"),
        ("CKKBPHNBKLC", "0x06005C4F"),
        ("Init", "0x06005C66"),
        ("BuffSetting", "0x06005C87"),
        ("Init", "0x06005C99"),
    }
    if bce_writers != expected_bce_writers:
        raise ValueError("BCEBGLKCDHN writer set changed")

    status_fields = fields(status_schema, "StatusData")
    hunter_fields = fields(hunter_schema, "HunterCtrl")
    game_fields = fields(manager_schema, "GameManager")
    user_fields = fields(user_schema, "UserData")
    expected_status = {
        0x88: "<CalcAttackSpeed>k__BackingField",
        0x198: "<AttackSpeed>k__BackingField",
        0x1AC: "<WeaponSpeed>k__BackingField",
        0x2F4: "OptionAttackSpeed",
        0x39C: "PersonalAttackSpeed",
        0x724: "RidingPetAttackSpeedUp",
        0x760: "<GUP_Property>k__BackingField",
        0x7A4: "RankAttackSpeed",
        0x7F8: "FuryValue",
        0x80C: "SpeedPotionValue",
        0x8D0: "Quicken",
    }
    for offset, name in expected_status.items():
        if status_fields[offset]["name"] != name:
            raise ValueError(f"StatusData schema changed at 0x{offset:x}")
    if hunter_fields[0x194]["name"] != "mAttackDelay" or hunter_fields[0x3D8]["name"] != "DANCPPLMKIK" or hunter_fields[0x6AC]["name"] != "BCEBGLKCDHN":
        raise ValueError("HunterCtrl attack-speed field schema changed")
    if game_fields[0x608]["name"] != "mUserData" or user_fields[0xBD8]["name"] != "<mGuildAttackSpeedUp>k__BackingField":
        raise ValueError("GameManager/UserData guild speed chain changed")

    resolved = {
        (row["className"], row["methodName"]): row
        for row in resolutions["record"]["payload"]["methods"]
    }
    game_instance = resolved[("GameManager", "getInstance")]
    if game_instance["candidates"][0]["moduleOffset"] != "0x26c0238":
        raise ValueError("GameManager.getInstance resolution changed")

    constants = {row["moduleOffset"]: row["float32"] for row in scan["runtimeConstants"]}
    if constants != {"0xd2aab8": 0.20000000298023224, "0xd2ac8c": 0.009999999776482582, "0xd2b834": 535.0}:
        raise ValueError("runtime constants changed")

    return {
        "schemaVersion": 1,
        "contractType": "original-native-hunter-attack-speed-producer-chain-evidence",
        "runtimeCompatibility": "evidence-only",
        "sources": [
            source(capture_path), source(scan_path), source(resolutions_path), source(status_schema_path), source(hunter_schema_path), source(manager_schema_path), source(user_schema_path)
        ],
        "methods": [
            {
                "type": key[0],
                "method": key[1],
                "parameterCount": key[2],
                "token": f"0x{methods[key]['token']:08X}",
                "moduleOffset": candidates[key]["moduleOffset"],
                "nativeSizeBytes": candidates[key]["nativeSizeBytes"],
                "bodySha256": hashlib.sha256(bodies[key]).hexdigest(),
            }
            for key in EXPECTED_METHODS
        ],
        "statusDataProducer": {
            "method": "StatusData.COJNMPDBOOO()",
            "attackSpeedEquation": "AttackSpeed = WeaponSpeed * (1 + 0.01 * (PersonalAttackSpeed + OptionAttackSpeed + RankAttackSpeed - UserData.mGuildAttackSpeedUp - GUP_Property[7] - RidingPetAttackSpeedUp))",
            "denominatorBranch": "denominator = FuryValue > 1 ? Quicken + FuryValue + SpeedPotionValue : Quicken + SpeedPotionValue + PersonalAttackSpeed",
            "calcAttackSpeedEquation": "CalcAttackSpeed = max(0.25, AttackSpeed / denominator)",
            "gupArrayIndex": 7,
            "guildChain": "GameManager.getInstance().mUserData.mGuildAttackSpeedUp",
            "goldenVectors": [
                {"input": [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], "attackSpeed": attack_speed(1, 0, 0, 0, 0, 0, 0)},
                {"input": [0.8, 10.0, 20.0, 5.0, 3.0, 2.0, 1.0], "attackSpeed": attack_speed(0.8, 10, 20, 5, 3, 2, 1)},
                {"input": [0.5, 0.0, 1.0, 1.0, 0.0], "calcAttackSpeed": calc_attack_speed(0.5, 0, 1, 1, 0)},
                {"input": [0.5, 4.0, 1.0, 2.0, 1.0], "calcAttackSpeed": calc_attack_speed(0.5, 4, 1, 2, 1)},
            ],
        },
        "furyAndBceChain": {
            "statusMethod": "StatusData.FGCEFJCHNCK(float)",
            "statusOperation": "FuryValue = input * 0.01; then call COJNMPDBOOO()",
            "buffSettingTypeZero": "BuffSetting(type=0, secondArgument, ...) writes FuryValue = secondArgument * 0.01 and BCEBGLKCDHN = secondArgument * 0.01",
            "buffEndTypeZero": "BuffEndSetting(0) writes FuryValue = 0, recomputes status, and resets BCEBGLKCDHN = 1.0",
            "otherExactWriters": [
                "Init(ObscuredInt, ObscuredString, Boolean) writes BCEBGLKCDHN = 1.0",
                "Init(Int32, Boolean) writes BCEBGLKCDHN = 1.0",
                "CKKBPHNBKLC(...) writes BCEBGLKCDHN from runtime float constant 535.0",
            ],
        },
        "attackDelayFsm": {
            "writerMethods": ["HuntingAttackAction", "CGAHEABLJMF", "NBOMDKMCGND"],
            "writerOperation": "raw-copy StatusData.CalcAttackSpeed into HunterCtrl.mAttackDelay",
            "readerMethod": "HunterCtrl.FixedUpdate()",
            "readerOperation": "decode mAttackDelay; if positive, subtract UnityEngine.Time.deltaTime and re-encode/store; if the decoded value is non-positive, use zero",
            "semanticStatus": "exact countdown timer confirmed; no separate direct managed reader exists in the class-wide scan",
        },
        "danFactor": {
            "field": "HunterCtrl.DANCPPLMKIK",
            "classMethodsScanned": scan["methodsScanned"]["HunterCtrl"],
            "directManagedWriters": [],
            "constructorWriter": False,
            "sourceStatus": "not resolved: likely populated outside direct managed method code, but Unity serialization/default injection is not claimed without decoding the opaque HunterCtrl prefab payload",
        },
        "classWideScan": {
            "statusDataMethods": scan["methodsScanned"]["StatusData"],
            "hunterCtrlMethods": scan["methodsScanned"]["HunterCtrl"],
            "memoryImage": scan["sources"]["memoryImage"],
            "bceWriterSet": sorted([{"method": name, "token": token} for name, token in expected_bce_writers], key=lambda row: row["token"]),
        },
        "unresolved": [
            "The engine-side or serialized source of HunterCtrl.DANCPPLMKIK; no direct writer exists in all 391 captured HunterCtrl managed bodies, including the constructor.",
            "The product-facing name and design meaning of BuffSetting type 0 and CKKBPHNBKLC's 535.0 initialization path.",
            "Exact gear-index-to-Spine weapon-skin binding remains separate and fail-closed.",
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture", type=Path, default=DEFAULT_CAPTURE)
    parser.add_argument("--scan", type=Path, default=DEFAULT_SCAN)
    parser.add_argument("--resolutions", type=Path, default=DEFAULT_RESOLUTIONS)
    parser.add_argument("--status-schema", type=Path, default=DEFAULT_STATUS_SCHEMA)
    parser.add_argument("--hunter-schema", type=Path, default=DEFAULT_HUNTER_SCHEMA)
    parser.add_argument("--manager-schema", type=Path, default=DEFAULT_MANAGER_SCHEMA)
    parser.add_argument("--user-schema", type=Path, default=DEFAULT_USER_SCHEMA)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    result = build(args.capture, args.scan, args.resolutions, args.status_schema, args.hunter_schema, args.manager_schema, args.user_schema)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote {args.output}")


if __name__ == "__main__":
    main()
