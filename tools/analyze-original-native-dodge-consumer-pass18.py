#!/usr/bin/env python3
"""Normalize the recovered native CalcDodge producer and consumer chains."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "reverse-engineering/evidence/original-native-dodge-consumer-pass18.json"
SOURCES = {
    "consumers": (
        ROOT / "reverse-engineering/evidence/original-native-dodge-consumers-api35-pass18.json",
        "9b683797a87b810e582e71653e33cbffcf879b05095367420535de642900570a",
    ),
    "callers": (
        ROOT / "reverse-engineering/evidence/original-native-dodge-callers-api35-pass18.json",
        "0ddc2326bc3d1754889291f588149a84c6132568648c11413eb9a937defaad65",
    ),
    "producers": (
        ROOT / "reverse-engineering/evidence/original-native-dodge-status-producers-api35-pass18.json",
        "1f2a9f20cb1159e57579f2135d247106c68d751768abd6db4e3a106d170a6d05",
    ),
    "hunterSchema": (
        ROOT / "reverse-engineering/evidence/original-native-dodge-hunter-schema-api35-pass18.json",
        "64492ee80126e65ee20e781921ad04dfe706793fb8ac1b2a03c44f2f97ff57d4",
    ),
    "modeSchema": (
        ROOT / "reverse-engineering/evidence/original-native-dodge-mode-schema-api35-pass18.json",
        "068afe5d52b81198b13e641b5aed3ff78ac289e8146d8f55869c0af53c3f1e13",
    ),
    "statusSchema": (
        ROOT / "reverse-engineering/evidence/status-data-runtime-schema-android-api35-v1.json",
        "6f6c2394ffaffb5a85fca0239c459434ad98ac7e70a54d1b27f54fe6ae0adda0",
    ),
    "hunterDataSchema": (
        ROOT / "reverse-engineering/evidence/hunter-info-runtime-schema-android-api35-v1.json",
        "6cc2faa575ed87567ed2262f2910a372596421d5d3b92264288639faa47da678",
    ),
}

RANGE_INT = "#0x5a76240"
OBSCURED_INT_DECODE = "#0x245672c"
OBSCURED_BOOL_DECODE = "#0x2456188"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_sources() -> dict[str, dict[str, Any]]:
    result = {}
    for name, (path, expected) in SOURCES.items():
        actual = sha256(path)
        if actual != expected:
            raise ValueError(f"source changed: {path} ({actual})")
        result[name] = json.loads(path.read_text())
    return result


def methods(document: dict[str, Any]) -> list[dict[str, Any]]:
    return document["record"]["payload"]["methods"]


def find_method(
    document: dict[str, Any], class_name: str, method_name: str, parameter_count: int
) -> dict[str, Any]:
    matches = [
        row
        for row in methods(document)
        if row["className"] == class_name
        and row["methodName"] == method_name
        and row["parameterCount"] == parameter_count
    ]
    if len(matches) != 1:
        raise ValueError(f"expected one {class_name}.{method_name}/{parameter_count}, got {len(matches)}")
    return matches[0]


def decode(method: dict[str, Any]) -> tuple[bytes, list[Any]]:
    candidate = method["candidates"][0]
    raw = bytes.fromhex(candidate["codeHex"])
    if candidate.get("codeTruncated") or len(raw) != candidate["nativeSizeBytes"]:
        raise ValueError(f"incomplete body: {method['className']}.{method['methodName']}")
    decoder = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    return raw, list(decoder.disasm(raw, int(candidate["moduleOffset"], 16)))


def require(method: dict[str, Any], anchors: list[tuple[int, str, str]]) -> None:
    _, instructions = decode(method)
    observed = {(row.address, row.mnemonic, row.op_str) for row in instructions}
    missing = [anchor for anchor in anchors if anchor not in observed]
    if missing:
        raise ValueError(f"native anchors changed for {method['className']}.{method['methodName']}: {missing}")


def descriptor(method: dict[str, Any]) -> dict[str, Any]:
    raw, _ = decode(method)
    candidate = method["candidates"][0]
    return {
        "className": method["className"],
        "methodName": method["methodName"],
        "parameterCount": method["parameterCount"],
        "token": method["token"],
        "moduleOffset": candidate["moduleOffset"],
        "nativeSizeBytes": len(raw),
        "bodySha256": hashlib.sha256(raw).hexdigest(),
    }


def schema_fields(document: dict[str, Any], class_name: str) -> dict[int, dict[str, Any]]:
    pending: list[Any] = [document]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            if value.get("name") == class_name and isinstance(value.get("fields"), list):
                return {field["offset"]: field for field in value["fields"]}
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)
    raise ValueError(f"schema class not found: {class_name}")


def build() -> dict[str, Any]:
    data = load_sources()
    consumer_specs = [
        ("HunterCtrl", 0),
        ("RaidHunterCtrl", 0),
        ("WorldBossHunterCtrl", 0),
        ("FallenPastureHunterCtrl", 2),
        ("GuildBattleHunterCtrl", 2),
        ("PvPHunterCtrl", 2),
    ]
    consumers = {
        name: find_method(data["consumers"], name, "DGPHLIIAEFL", parameter_count)
        for name, parameter_count in consumer_specs
    }

    require(
        consumers["HunterCtrl"],
        [
            (0x344D64C, "bl", "#0x3411e7c"),
            (0x344D678, "mov", "w1, #0x64"),
            (0x344D680, "bl", RANGE_INT),
            (0x344D694, "ldp", "x8, x1, [x0, #0xc0]"),
            (0x344D6A4, "ldr", "x8, [x20, #0x728]"),
            (0x344D6BC, "add", "w8, w0, w23"),
            (0x344D6C4, "b.lt", "#0x344d700"),
            (0x344D6CC, "mov", "w1, #0x3e8"),
            (0x344D6E8, "add", "x8, x0, #0x6c4"),
            (0x344D6FC, "b.ge", "#0x344d714"),
            (0x344D754, "mov", "w1, #3"),
        ],
    )
    require(
        consumers["RaidHunterCtrl"],
        [
            (0x2507714, "mov", "w1, #0x64"),
            (0x250771C, "bl", RANGE_INT),
            (0x2507730, "ldp", "x8, x1, [x0, #0xc0]"),
            (0x2507740, "ldr", "w8, [x20, #0x298]"),
            (0x2507744, "add", "w8, w8, w0"),
            (0x250774C, "b.lt", "#0x2507788"),
            (0x2507754, "mov", "w1, #0x3e8"),
            (0x2507770, "add", "x8, x0, #0x6c4"),
        ],
    )
    require(
        consumers["WorldBossHunterCtrl"],
        [
            (0x2CC24C8, "ldp", "x0, x1, [x8, #0xc0]"),
            (0x2CC24D8, "ldr", "q0, [x20, #0x6f0]"),
            (0x2CC24E4, "ldr", "w24, [x20, #0x2e0]"),
            (0x2CC24FC, "ldp", "x0, x1, [x23]"),
            (0x2CC2530, "sub", "w9, w10, w0"),
            (0x2CC2538, "sub", "w8, w9, w8"),
            (0x2CC253C, "bic", "w22, w8, w8, asr #31"),
            (0x2CC2520, "mov", "w1, #0x65"),
            (0x2CC2540, "bl", RANGE_INT),
            (0x2CC2550, "mov", "w1, #0x3e8"),
            (0x2CC2564, "add", "x8, x8, #0x6c4"),
        ],
    )
    require(
        consumers["FallenPastureHunterCtrl"],
        [
            (0x29DA404, "ldr", "x8, [x19, #0x9a8]"),
            (0x29DA40C, "ldp", "x0, x1, [x8, #0xc0]"),
            (0x29DA418, "ldr", "x8, [x19, #0x3e8]"),
            (0x29DA424, "ldr", "w25, [x19, #0x390]"),
            (0x29DA43C, "sub", "w0, w8, w0"),
            (0x29DA450, "mov", "w1, #0x64"),
            (0x29DA47C, "mov", "w1, #0x3e8"),
        ],
    )
    for class_name, status_offset in (("GuildBattleHunterCtrl", "0x430"), ("PvPHunterCtrl", "0x950")):
        method = consumers[class_name]
        _, instructions = decode(method)
        text = {(row.mnemonic, row.op_str) for row in instructions}
        required = {
            ("ldr", f"x8, [x22, #{status_offset}]") if class_name.startswith("Guild") else ("ldr", f"x8, [x19, #{status_offset}]"),
            ("mov", "w1, #0x64"),
            ("mov", "w1, #0x3e8"),
            ("ldr", "x0, [x8, #0x630]"),
        }
        if not required.issubset(text):
            raise ValueError(f"{class_name} dodge shape changed: {required - text}")

    callers = {
        ("HunterCtrl", "Damaged"): (0x3465570, "#0x344d63c", 0x3465580),
        ("HunterCtrl", "EvilDeBuffAction"): (0x3454D74, "#0x344d63c", 0x3454D84),
        ("RaidHunterCtrl", "Damaged"): (0x2503520, "#0x25076c8", 0x2503530),
        ("WorldBossHunterCtrl", "ForcedDamaged"): (0x2CC0EB8, "#0x2cc247c", 0x2CC0EC8),
        ("WorldBossHunterCtrl", "Damaged"): (0x2CDD79C, "#0x2cc247c", 0x2CDD7AC),
        ("GuildBattleHunterCtrl", "Damaged"): (0x2656878, "#0x266a158", 0x265688C),
        ("GuildBattleHunterCtrl", "ForcedDamaged"): (0x2668D94, "#0x266a158", 0x2668DA4),
        ("GuildBattleHunterCtrl", "FCFGHENGPME"): (0x266C754, "#0x266a158", 0x266C764),
    }
    caller_descriptors = []
    for (class_name, method_name), (call_site, target, branch_site) in callers.items():
        parameter_count = {
            ("HunterCtrl", "Damaged"): 2,
            ("HunterCtrl", "EvilDeBuffAction"): 3,
            ("RaidHunterCtrl", "Damaged"): 3,
            ("WorldBossHunterCtrl", "ForcedDamaged"): 2,
            ("WorldBossHunterCtrl", "Damaged"): 2,
            ("GuildBattleHunterCtrl", "Damaged"): 7,
            ("GuildBattleHunterCtrl", "ForcedDamaged"): 5,
            ("GuildBattleHunterCtrl", "FCFGHENGPME"): 5,
        }[(class_name, method_name)]
        method = find_method(data["callers"], class_name, method_name, parameter_count)
        require(
            method,
            [
                (call_site, "bl", target),
                (call_site + 0xC, "bl", OBSCURED_BOOL_DECODE),
                (branch_site, "tbnz", next(row.op_str for row in decode(method)[1] if row.address == branch_site)),
            ],
        )
        caller_descriptors.append(descriptor(method))

    evil_debuff = find_method(data["callers"], "HunterCtrl", "EvilDeBuffAction", 3)
    require(
        evil_debuff,
        [
            (0x3454D48, "cmp", "w0, #0x33"),
            (0x3454D4C, "b.ne", "#0x3454e0c"),
            (0x3454D74, "bl", "#0x344d63c"),
        ],
    )
    buff_setting = find_method(data["callers"], "HunterCtrl", "BuffSetting", 4)
    require(
        buff_setting,
        [
            (0x34600C8, "cmp", "w21, #5"),
            (0x34600CC, "b.eq", "#0x3461004"),
            (0x34610B0, "ldr", "w0, [x20, #0x1c]"),
            (0x34610B8, "bl", "#0x2457964"),
            (0x34610BC, "str", "x0, [x19, #0x728]"),
            (0x34610C0, "str", "x1, [x19, #0x730]"),
        ],
    )

    producer = find_method(data["producers"], "StatusData", "CEOBAMNDIIL", 0)
    require(
        producer,
        [
            (0x2D67DCC, "add", "x8, x0, #0x264"),
            (0x2D67DD4, "add", "x20, x19, #0x32c"),
            (0x2D67DF4, "add", "x8, x19, #0x3d8"),
            (0x2D67E18, "add", "x8, x19, #0x7cc"),
            (0x2D67E3C, "ldr", "x8, [x19, #0x760]"),
            (0x2D67E54, "ldr", "w9, [x8, #0xd0]"),
            (0x2D67E58, "ldr", "q0, [x8, #0xc0]"),
            (0x2D67E70, "add", "w8, w21, w20"),
            (0x2D67E80, "fadd", "s1, s8, s1"),
            (0x2D67E84, "fadd", "s1, s1, s9"),
            (0x2D67E88, "fadd", "s0, s1, s0"),
            (0x2D67EB8, "fcmp", "s0, #0.0"),
            (0x2D67EC0, "movi", "d0, #0000000000000000"),
            (0x2D67FC4, "stp", "x0, x1, [x19, #0xc0]"),
        ],
    )

    status_fields = schema_fields(data["statusSchema"], "StatusData")
    hunter_fields = schema_fields(data["hunterDataSchema"], "HunterData")
    expected_status = {
        192: "<CalcDodge>k__BackingField",
        488: "<Dodge>k__BackingField",
        812: "OptionDodge",
        984: "PersonalDodge",
        1888: "<GUP_Property>k__BackingField",
        1996: "RankDodge",
    }
    for offset, name in expected_status.items():
        if status_fields[offset]["name"] != name:
            raise ValueError(f"StatusData field changed at {offset}")
    if hunter_fields[612]["name"] != "<dodge>k__BackingField":
        raise ValueError("HunterData dodge field changed")

    return {
        "schemaVersion": 1,
        "contractType": "original-native-dodge-consumer-pass18",
        "runtimeCompatibility": "evidence-backed-core-with-rebuild-total-evasion-projection",
        "sources": [
            {"name": name, "path": path.relative_to(ROOT).as_posix(), "sha256": expected}
            for name, (path, expected) in SOURCES.items()
        ],
        "normalHunter": {
            "method": descriptor(consumers["HunterCtrl"]),
            "formula": "if IsMezeState then false; otherwise primaryRoll[0,100) < wrapping_i32(CalcDodge + effectType5Bonus), else petRoll[0,1000) < RidingPetDodge",
            "comparison": "signed exclusive less-than",
            "rngConsumption": "meze consumes no roll; primary success skips the pet roll",
            "thresholdClamping": "none in this method",
            "presentationOnSuccess": {"nativeType": 3, "identity": "Evade"},
            "fields": {
                "calcDodge": "StatusData.<CalcDodge>k__BackingField@0xC0",
                "effectType5Bonus": "HunterCtrl.EFPJBDACDNH@0x728",
                "ridingPetDodge": "StatusData.RidingPetDodge@0x6C4",
            },
            "callers": [
                "HunterCtrl.Damaged/2 exits the damage routine when true",
                "HunterCtrl.EvilDeBuffAction/3 applies this only on the internal value-51 branch and exits that debuff path when true",
            ],
        },
        "producer": {
            "method": descriptor(producer),
            "rawDodge": "HunterData.dodge + OptionDodge + PersonalDodge + RankDodge + GUP_Property[8]",
            "storage": "StatusData.Dodge receives rawDodge, is replaced with 0 when Dodge <= 0, then StatusData.CalcDodge receives banker-round-to-i32(Dodge)",
            "inputFields": {
                "HunterData.dodge": 612,
                "StatusData.OptionDodge": 812,
                "StatusData.PersonalDodge": 984,
                "StatusData.RankDodge": 1996,
                "StatusData.GUP_Property": 1888,
                "StatusData.Dodge": 488,
                "StatusData.CalcDodge": 192,
            },
            "commonWrappers": ["PCJKOECDDJO", "NFGEIMNPBNM", "KCKPLIMMKFP", "GHLJIGIKKDI", "FMJMKNBDIEP overloads"],
            "alternateProducerBoundary": "PFIONPOHHJK uses GUP_Property[7] and unresolved runtime constants; its only direct wrapper is IEBHFGJHELH(String,Boolean), so it is not treated as the common village formula.",
        },
        "modeVariants": {
            "raid": {
                "method": descriptor(consumers["RaidHunterCtrl"]),
                "formula": "same normal flow with plain RaidHunterCtrl.EFPJBDACDNH@0x298",
                "caller": "RaidHunterCtrl.Damaged/3",
            },
            "worldBoss": {
                "method": descriptor(consumers["WorldBossHunterCtrl"]),
                "formula": "threshold=max(EFPJBDACDNH + CalcDodge - decode(NDJAJDFKPFF) - trunc(PANOJKHNLEM), 0); primaryRoll[0,101) < threshold; same pet fallback",
                "callers": ["WorldBossHunterCtrl.ForcedDamaged/2", "WorldBossHunterCtrl.Damaged/2"],
                "unresolvedFields": ["PANOJKHNLEM public semantics/writers", "NDJAJDFKPFF public semantics/writers"],
            },
            "fallenPasture": {
                "method": descriptor(consumers["FallenPastureHunterCtrl"]),
                "formula": "threshold=CalcDodge + plain field@0x390 - decode(field@0x3E8); primary [0,100), same pet fallback",
                "callerStatus": "no direct BL caller found in the captured Assembly-CSharp method index; likely dispatch remains unresolved",
            },
            "guildBattle": {
                "method": descriptor(consumers["GuildBattleHunterCtrl"]),
                "formula": "threshold=PvPStatusData field@0xA8 + EFPJBDACDNH@0x2B0 - HDPCCGIDEIL@0x4A0, with an optional opponent-owned subtraction; primary [0,100), pet fallback uses PvPStatusData@0x630",
                "callers": ["Damaged/7", "ForcedDamaged/5", "FCFGHENGPME/5"],
            },
            "pvp": {
                "method": descriptor(consumers["PvPHunterCtrl"]),
                "formula": "same structural PvP threshold using StatusData@0x950, plain field@0x390, own subtraction@0x3E8, optional opponent subtraction, and pet fallback@0x630",
                "callerStatus": "no direct BL caller found in the captured Assembly-CSharp method index; likely dispatch remains unresolved",
            },
        },
        "effectType5Writer": {
            "method": descriptor(buff_setting),
            "proof": "BuffSetting effectType==5 converts the branch payload at +0x1C to ObscuredInt and stores it in HunterCtrl@0x728",
            "publicName": None,
        },
        "callerMethods": caller_descriptors,
        "integrationBoundary": {
            "coreStatus": "normal Hunter resolver and contribution calculator preserve the recovered threshold arithmetic",
            "liveStatus": "the rebuild treats profile.evasion_rate_bps as total evasion, rounds it to CalcDodge, and uses a deterministic uniform roll with the recovered bounds; missing named sources, effect type 5, and riding-pet dodge are zero",
            "unresolved": [
                "public name of effect type 5",
                "public name of EvilDeBuffAction internal value 51",
                "World Boss subtractor writers/semantics",
                "PvP/Guild opponent nested-field semantics",
                "Fallen Pasture and PvP indirect caller dispatch",
                "UnityEngine.Random internal PRNG sequence",
            ],
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    result = build()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=True) + "\n")
    print(f"Wrote {args.output}")


if __name__ == "__main__":
    main()
