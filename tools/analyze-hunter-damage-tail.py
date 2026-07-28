#!/usr/bin/env python3
"""Normalize Hunter pre-armor modifiers, armor selection, and damage routing."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import math
import struct
import zipfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = ROOT / "reverse-engineering/evidence/original-native-hunter-damage-tail-v3.json"
FULL_HUNTER = ROOT / "reverse-engineering/evidence/original-native-hunter-damage-intake-api35-v1.json"
COMBAT_METHODS = ROOT / "reverse-engineering/evidence/original-native-combat-formula-exact-boundaries-api35-v1.json"
STATIC_FACTORS = ROOT / "reverse-engineering/evidence/original-runtime-combat-static-factors-api35-v1.json"
STATUS_SCHEMA = ROOT / "reverse-engineering/evidence/status-data-runtime-schema-android-api35-v1.json"
HUNTER_SCHEMA = ROOT / "reverse-engineering/evidence/hunter-info-runtime-schema-android-api35-v1.json"
SHIELD_SCHEMA = ROOT / "reverse-engineering/evidence/shield-data-runtime-schema-android-api35-v1.json"
HUNTER_CTRL_SCHEMA = ROOT / "reverse-engineering/evidence/evil-ai-drop-runtime-schema-android-api35-v1.json"
CONSTANT_SCHEMA = ROOT / "reverse-engineering/evidence/constant-data-runtime-schema-api35-v1.json"
CONSTANT_CCTOR = ROOT / "reverse-engineering/evidence/original-native-constant-data-cctor-api35-v1.json"
ACTK_FLOAT_SCHEMA = ROOT / "reverse-engineering/evidence/actk-obscured-float-runtime-schema-api35-v1.json"
ACTK_FLOAT_METHODS = ROOT / "reverse-engineering/evidence/actk-obscured-float-native-methods-api35-v1.json"
ACTK_BYTE4_METHODS = ROOT / "reverse-engineering/evidence/actk-byte4-native-methods-api35-v1.json"
XAPK = ROOT / "game-assets/source/Evil+Hunter+Tycoon_1.411_APKPure.xapk"


PRE_ARMOR_STAGES = [
    (1, 0x3466318, 0x3466364, "proportional_add", "24e81179eb56243622427bb10e358c1d67fbd0d634c5c69f307cdac39fa8ea06", "HunterCtrl.HKFBHKIJNJG@0x614"),
    (2, 0x3466400, 0x3466440, "proportional_subtract", "81f5c4e40b90c267da7351bf0e2f87682fa73131548dea78b71685ea4937aada", "HunterCtrl.PGDMKPKELMM@0x698"),
    (3, 0x34664DC, 0x3466528, "summed_proportional_subtract", "91e0e8913ed54aa33c0075d0d5a35e291a038d9584f8d31511cd218dc77928c0", "StatusData.GearArmorUpgrade@0x254 + StatusData.CostumeImmuneUp@0x464"),
    (4, 0x3466600, 0x3466640, "negative_percent_point_add", "9daa272ea92453f13086167e108910b95c137a10e8973f538826ae0ddc5bbba1", "StatusData.GearProperty row 56"),
    (5, 0x3466724, 0x3466764, "negative_percent_point_add", "82ca1a75ddb300cba3f63c16e8247205397b42e62953b121cedd736d6318e9b7", "StatusData.GearProperty row 57"),
    (6, 0x3466880, 0x34668C0, "fixed_scale", "68e2b58c5687094fb39d9996ac26db20e3a7644a109b739d8165ff57c2d148f0", "ConstantData.IMMUNE_VALUE@0x100"),
    (7, 0x34669A0, 0x34669E0, "negative_percent_point_add", "7338602347e483893503ced719f47cdb23e2cb443fe3665aae4e4d6d57793bb1", "runtime collection entry 80"),
    (8, 0x3466B28, 0x3466B68, "negative_percent_point_add", "9f0996695f632f269afb97638effd2242fe5ab8b5af46d10cb88193c8ffef508", "runtime indexed collection entry"),
    (9, 0x3466C20, 0x3466C60, "one_minus_percent_scale", "51c152d6e4237a52a0764ed271bdf6e9de2d6e3c71012c6249f0f85a10c1dac4", "ConstantData.SOUL_ABSORPTION_DECREASE_VALUE@0x2894"),
    (10, 0x3466CEC, 0x3466D2C, "negative_percent_point_add", "b6ddcd0a3fac7dec6cc4bc553d9b5f335741f1f5df89c06bd9d7e8ff154ed03f", "HunterCtrl.JECJDDBJAPE@0x628"),
    (11, 0x3466DB8, 0x3466DF8, "negative_percent_point_add", "42a354e12a8fb1a607365065889ee44b856581b2b208f5d54de121f2a970c214", "HunterCtrl.FIMICNDLECJ@0x7d4"),
    (12, 0x3466EA8, 0x3466EE8, "fixed_scale", "c8f019ee0ea9ec28fd5a25aaca67abf3ab3f534a6afe6ce1a323a6798cc7553d", "ConstantData.EXECUTOR_DAMAGE_DECREASE_VALUE@0x13c"),
    (13, 0x34670A8, 0x34670E8, "proportional_subtract", "1f572731420f29afb882663ef6b9c239151d62699bbf609799eb83be5b03a0d7", "runtime effect value captured at stack 0x1570"),
    (14, 0x34675FC, 0x346763C, "proportional_subtract", "42759cc3d9b184ca33c9982ee3d0c4d796356c91f1a8a1d0c8dc183ee9e87102", "runtime effect value captured at stack 0x1550"),
    (15, 0x3467724, 0x3467764, "negative_percent_point_add", "7ba9be0542298a57f62abc57678b15a30a242d35a208d3324e5737d885fd5ae5", "HunterCtrl runtime factor"),
    (16, 0x346796C, 0x34679AC, "negative_basis_point_add", "92a847da19fe43a435d556c366afb59927ca3d5fba985ad0063b4795aa447626", "runtime dictionary value"),
    (17, 0x3467A40, 0x3467A80, "proportional_subtract", "1ed304d3b623c2e1765a4f7fb82bca6947868f4d07df9572109b9c24664def0e", "StatusData.RidingPetImmuneUp@0x680"),
    (18, 0x3467B5C, 0x3467B9C, "negative_percent_point_add", "fe27af782a3714bd04b9f63a988952b45872190eae6faf80f16889dc43e50a3f", "StatusData.RidingPetGearProperty row 9"),
    (19, 0x3467CFC, 0x3467D3C, "negative_percent_point_add", "1ab9a11718b084eafeb9cf3355f9aed3c22b49a97a29cc1d8fcbb16267731e89", "runtime array entry 13"),
    (20, 0x3467E10, 0x3467E50, "proportional_subtract", "b73d535560e4775a9a27ea47e3e6c5554346532f6010916623c4cfcff7314b3e", "ConstantData.DRAGON_FURY_BUFF_HANS_OF_GOD_HIT_DECREASE_VALUE@0x2f54"),
    (21, 0x3467F50, 0x3467F90, "proportional_subtract", "8713511111e902b80cff77f603716f9fe0b9710d4f5de5fa2490f7c2f299eed2", "ConstantData.FROZEN_HEART_DAMAGE_DECREASE_VALUE@0x3484"),
    (22, 0x3468024, 0x3468064, "proportional_subtract", "43f14ee43d15cebcc71cbfd52a80652730e735aa1d5aeb0cd9b3f49c6e184bca", "StatusData.HeroicJobTraitImmuneUp@0x630"),
    (23, 0x34681B8, 0x34681F8, "negative_percent_point_add", "c6dd7457858f7f834b62922e2c64c3f9d5c2cf7a726035909f3bea2907c89a25", "conditional class/trait aggregate"),
    (24, 0x3468278, 0x34682B8, "direct_product_subtract", "5c7aefc049a7f6331f64838942844f5b847a7316a39a59bb70d2654de760b7ee", "HunterCtrl.FFDOBPJLLFM@0x7e8"),
    (25, 0x3468418, 0x3468458, "percent_product_subtract", "9e1841ceeea1a691698bd2486b689abea61d18eed51a8e1df36fb260fa20286e", "conditional class/trait aggregate"),
    (26, 0x34684D4, 0x3468514, "direct_product_subtract", "6eb453e555b8ad70a3f50832fe6bb26d590d18f136eeb37b78334c9b16424c5c", "HunterCtrl.AAFECOPNGJA@0x87c"),
    (27, 0x3468670, 0x34686B0, "percent_product_subtract", "aa0ccc02efedbcfd8052786ddd08aa13b7a2d678021a159264e10631448cf6f2", "conditional class/trait aggregate"),
    (28, 0x34687F4, 0x3468834, "percent_product_subtract", "bf9f4633f37c825cd688e74c38cae69e1f5489737eb0f53e71d5ac5aaf4a3dea", "conditional class/trait aggregate"),
    (29, 0x34688EC, 0x346892C, "percent_product_subtract", "d0110754be638413d726bcff06f882cb131f843274ab57064d79e85dd8752dd9", "HunterCtrl.AEJMHHJPCCG@0x9b4"),
    (30, 0x34689E4, 0x3468A24, "percent_product_subtract", "81b4ecc4a9ca9f929ec8337329d88e32330187c80554530c523792d60e6dc6e7", "HunterCtrl.GAMLIJMBJIH@0x9c8"),
    (31, 0x3468ACC, 0x3468B0C, "direct_product_subtract", "bea378caa91cc78dc2fce6c2c7f80a8044436b355dc369040af355766b70aa6d", "HunterCtrl.CMGHCGEGLEN@0x9f0"),
    (32, 0x3468C70, 0x3468CB0, "percent_product_subtract", "7729f434f1b74444790c8c15fdd249e8a1c9cae585b11674a0fb532332908415", "runtime gear/aura count"),
]


FORMULAS = {
    "proportional_add": "A' = truncTowardZero(float32(A) + float32(factor * float32(A)))",
    "proportional_subtract": "A' = truncTowardZero(float32(A) - float32(factor * float32(A)))",
    "summed_proportional_subtract": "A' = truncTowardZero(float32(A) - float32((factor1 + factor2) * float32(A)))",
    "negative_percent_point_add": "A' = truncTowardZero(float32(A) + float32(rawValue * count * -0.01f))",
    "negative_basis_point_add": "A' = truncTowardZero(float32(A) + float32(rawValue * count * -0.0001f))",
    "fixed_scale": "A' = truncTowardZero(float32(A) * factor)",
    "one_minus_percent_scale": "A' = truncTowardZero(float32(A) * (1.0f + rawPercent * -0.01f))",
    "direct_product_subtract": "A' = A - truncTowardZero(factor * float32(A))",
    "percent_product_subtract": "A' = A - truncTowardZero(composite * float32(A) * 0.01f)",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text())


def method(payload: dict[str, Any], class_name: str, method_name: str) -> tuple[dict[str, Any], bytes]:
    record = next(
        row
        for row in payload["record"]["payload"]["methods"]
        if row["className"] == class_name and row["methodName"] == method_name
    )
    candidate = record["candidates"][0]
    body = bytes.fromhex(candidate["codeHex"])
    if candidate.get("codeTruncated", False) or (
        "nativeSizeBytes" in candidate and len(body) != candidate["nativeSizeBytes"]
    ):
        raise ValueError(f"incomplete body: {class_name}.{method_name}")
    return record, body


def class_fields(payload: dict[str, Any], class_name: str) -> dict[str, dict[str, Any]]:
    cls = next(row for row in payload["record"]["payload"]["classes"] if row["name"] == class_name)
    return {field["name"]: field for field in cls["fields"]}


def package_float_constants() -> list[float]:
    with zipfile.ZipFile(XAPK) as outer:
        apk_bytes = outer.read("config.arm64_v8a.apk")
    with zipfile.ZipFile(io.BytesIO(apk_bytes)) as apk:
        lib = apk.read("lib/arm64-v8a/libil2cpp.so")
    offsets = [0xD2A50C, 0xD2A414, 0xD2B814, 0xD2AAB8]
    return [struct.unpack_from("<f", lib, offset)[0] for offset in offsets]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


def build() -> dict[str, Any]:
    full_hunter = load(FULL_HUNTER)
    combat = load(COMBAT_METHODS)
    static = load(STATIC_FACTORS)
    status_schema = load(STATUS_SCHEMA)
    hunter_schema = load(HUNTER_SCHEMA)
    shield_schema = load(SHIELD_SCHEMA)
    hunter_ctrl_schema = load(HUNTER_CTRL_SCHEMA)
    constant_schema = load(CONSTANT_SCHEMA)
    constant_cctor = load(CONSTANT_CCTOR)
    actk_float_schema = load(ACTK_FLOAT_SCHEMA)
    actk_float_methods = load(ACTK_FLOAT_METHODS)
    actk_byte4_methods = load(ACTK_BYTE4_METHODS)

    damaged_record, damaged_body = method(full_hunter, "HunterCtrl", "Damaged")
    hit_record, hit_body = method(combat, "HunterCtrl", "HitDamageProcess")
    if hashlib.sha256(damaged_body).hexdigest() != "5be5aa13cc8593c863421e6fb16d267473dc7fbdcf2d71b6e27e16496ce540f6":
        raise ValueError("HunterCtrl.Damaged body changed")
    if hashlib.sha256(hit_body).hexdigest() != "a0632289d593e15dc6584f9a21958ebc5318d142167b27507bf83424ed5c90db":
        raise ValueError("HunterCtrl.HitDamageProcess body changed")

    status_fields = class_fields(status_schema, "StatusData")
    hunter_fields = class_fields(hunter_schema, "HunterData")
    shield_fields = class_fields(shield_schema, "ShieldData")
    hunter_ctrl_fields = class_fields(hunter_ctrl_schema, "HunterCtrl")
    constant_fields = class_fields(constant_schema, "ConstantData")
    obscured_float_fields = class_fields(actk_float_schema, "ObscuredFloat")
    required_offsets = {
        status_fields["<CalcArmor>k__BackingField"]["offset"]: 72,
        hunter_fields["<feel>k__BackingField"]["offset"]: 392,
        hunter_fields["<nowFeel>k__BackingField"]["offset"]: 412,
        hunter_fields["<nowHp>k__BackingField"]["offset"]: 360,
        hunter_fields["<mShieldDataDic>k__BackingField"]["offset"]: 1488,
        shield_fields["MaxShield"]["offset"]: 16,
        shield_fields["CurrentShield"]["offset"]: 48,
    }
    if any(actual != expected for actual, expected in required_offsets.items()):
        raise ValueError("runtime field offsets changed")
    if hunter_ctrl_fields["LIEGAADKDHD"]["offset"] != 1888:
        raise ValueError("HunterCtrl accumulator offset changed")
    if constant_fields["DEFALUT_DAMAGE_DECREASE_VALUE"]["offset"] != 276:
        raise ValueError("ConstantData damage-decrease offset changed")
    expected_actk_offsets = {
        "currentCryptoKey": 16,
        "hiddenValue": 20,
        "hiddenValueOldByte4": 24,
        "fakeValue": 28,
        "fakeValueActive": 32,
        "inited": 33,
    }
    if any(obscured_float_fields[name]["offset"] != offset for name, offset in expected_actk_offsets.items()):
        raise ValueError("ACTk ObscuredFloat layout changed")

    thresholds = package_float_constants()
    expected_thresholds = [0.8, 0.6, 0.4, 0.2]
    if not all(math.isclose(actual, expected, rel_tol=1e-6) for actual, expected in zip(thresholds, expected_thresholds)):
        raise ValueError(f"feel thresholds changed: {thresholds}")
    armor_factors = static["armorFactorArray"]["values"]
    expected_factors = [1.2, 1.1, 1.0, 0.9, 0.8]
    if not all(math.isclose(actual, expected, rel_tol=1e-6) for actual, expected in zip(armor_factors, expected_factors)):
        raise ValueError(f"armor factors changed: {armor_factors}")

    final_factor = static["selectedFinalFactor"]["decodedValue"]
    if not math.isclose(final_factor, 0.75, rel_tol=0, abs_tol=1e-7):
        raise ValueError(f"default damage-decrease factor changed: {final_factor}")

    cctor_record, cctor_body = method(constant_cctor, "ConstantData", ".cctor")
    cctor_base = int(cctor_record["candidates"][0]["moduleOffset"], 16)
    writer_start = 0x330CB14
    writer_end = 0x330CB84
    writer = cctor_body[writer_start - cctor_base:writer_end - cctor_base]
    writer_sha = hashlib.sha256(writer).hexdigest()
    if writer_sha != "a7b4c6dcdebc25405e788902c7cbab8aad3f2631d2f78d0f102b0256ee057707":
        raise ValueError("ConstantData damage-decrease writer changed")

    implicit_candidates = [
        row for row in actk_float_methods["record"]["payload"]["methods"]
        if row["className"] == "ObscuredFloat" and row["methodName"] == "op_Implicit"
        and row["returnType"] == "System.Single"
    ]
    if len(implicit_candidates) != 1:
        raise ValueError("ACTk float decode overload changed")
    implicit_record = implicit_candidates[0]
    implicit_body = bytes.fromhex(implicit_record["candidates"][0]["codeHex"])
    unshuffle_record, unshuffle_body = method(actk_byte4_methods, "ACTkByte4", "UnShuffle")
    if implicit_record["candidates"][0]["moduleOffset"] != "0x245668c":
        raise ValueError("ACTk float decode helper moved")
    if unshuffle_record["candidates"][0]["moduleOffset"] != "0x2486c24":
        raise ValueError("ACTk byte unshuffle helper moved")

    damaged_base = int(damaged_record["candidates"][0]["moduleOffset"], 16)
    pre_armor_stages = []
    for order, start, end, family, expected_sha, source in PRE_ARMOR_STAGES:
        window = damaged_body[start - damaged_base:end - damaged_base]
        actual_sha = hashlib.sha256(window).hexdigest()
        if actual_sha != expected_sha:
            raise ValueError(f"pre-armor stage {order} changed")
        pre_armor_stages.append({
            "order": order,
            "nativeStart": f"0x{start:x}",
            "nativeEndExclusive": f"0x{end:x}",
            "windowSha256": actual_sha,
            "family": family,
            "equation": FORMULAS[family],
            "operandSource": source,
            "gateStatus": "native branch/order exact; product-facing condition remains unresolved where the source name is obfuscated",
        })

    sources = [
        FULL_HUNTER, COMBAT_METHODS, STATIC_FACTORS, STATUS_SCHEMA, HUNTER_SCHEMA,
        SHIELD_SCHEMA, HUNTER_CTRL_SCHEMA, CONSTANT_SCHEMA, CONSTANT_CCTOR,
        ACTK_FLOAT_SCHEMA, ACTK_FLOAT_METHODS, ACTK_BYTE4_METHODS, XAPK,
    ]
    return {
        "schemaVersion": 1,
        "contractType": "original-native-hunter-damage-tail-analysis-v3",
        "runtimeCompatibility": "evidence-only",
        "package": { "id": "com.superplanet.evilhunter", "versionName": "1.411", "androidApi": 35, "abi": "arm64-v8a" },
        "sources": [
            { "path": path.relative_to(ROOT).as_posix(), "sha256": sha256(path) }
            for path in sources
        ],
        "methods": [
            {
                "type": "HunterCtrl",
                "method": "Damaged",
                "token": f'0x{damaged_record["token"]:08X}',
                "moduleOffset": damaged_record["candidates"][0]["moduleOffset"],
                "nativeSizeBytes": len(damaged_body),
                "bodySha256": hashlib.sha256(damaged_body).hexdigest(),
            },
            {
                "type": "HunterCtrl",
                "method": "HitDamageProcess",
                "token": f'0x{hit_record["token"]:08X}',
                "moduleOffset": hit_record["candidates"][0]["moduleOffset"],
                "nativeSizeBytes": len(hit_body),
                "bodySha256": hashlib.sha256(hit_body).hexdigest(),
            },
            {
                "type": "ConstantData",
                "method": ".cctor",
                "token": f'0x{cctor_record["token"]:08X}',
                "moduleOffset": cctor_record["candidates"][0]["moduleOffset"],
                "capturedPrefixBytes": len(cctor_body),
                "writerWindow": {
                    "start": f"0x{writer_start:x}",
                    "endExclusive": f"0x{writer_end:x}",
                    "sha256": writer_sha,
                },
            },
        ],
        "preArmorAccumulator": {
            "owner": "HunterCtrl",
            "field": "LIEGAADKDHD",
            "offset": 1888,
            "type": hunter_ctrl_fields["LIEGAADKDHD"]["type"],
            "initialization": "A0 = truncTowardZero(float32(incomingDamage) * GameManager.RandDamage())",
            "stageCount": len(pre_armor_stages),
            "stages": pre_armor_stages,
            "nextOperation": "A33 = A32 - armorScratch",
            "orderingStatus": "all 32 optional mutation checkpoints are preserved in exact native execution order",
        },
        "armorSelector": {
            "ratio": "HunterData.nowFeel / HunterData.feel",
            "zeroDenominatorStatus": "No explicit zero-denominator branch is isolated in the selected native block; runtime HunterData invariant remains required.",
            "bands": [
                { "condition": "nowFeel / feel >= 0.8", "factor": armor_factors[0] },
                { "condition": "nowFeel / feel >= 0.6", "factor": armor_factors[1] },
                { "condition": "nowFeel / feel >= 0.4", "factor": armor_factors[2] },
                { "condition": "nowFeel / feel >= 0.2", "factor": armor_factors[3] },
                { "condition": "nowFeel / feel < 0.2", "factor": armor_factors[4] },
            ],
            "equation": "armorScratch = truncTowardZero(StatusData.CalcArmor * selectedArmorFactor)",
            "thresholdFloat32": thresholds,
            "factorRuntimeDataOffset": static["armorFactorArray"]["runtimeDataOffset"],
            "factorRawHex": static["armorFactorArray"]["managedArrayRawHex"],
        },
        "selectedFinalFactor": {
            "runtimeDataOffset": static["selectedFinalFactor"]["runtimeDataOffset"],
            "owner": "ConstantData",
            "field": "DEFALUT_DAMAGE_DECREASE_VALUE",
            "capturedValue": final_factor,
            "rawHex": static["selectedFinalFactor"]["obscuredFloatRawHex"],
            "equation": "positiveForwardedDamage = truncTowardZero(postArmor * selectedFinalFactor)",
            "actkDecode": "swap hiddenValue bytes 1 and 2 via ACTkByte4.UnShuffle, then XOR currentCryptoKey and reinterpret float32",
            "constructorValue": 0.75,
            "writer": "ConstantData..cctor -> ObscuredFloat.op_Implicit(0.75f) -> static runtimeData+0x114",
            "semanticStatus": "field name, exact value, ACTk decode and initonly writer resolved",
        },
        "shieldRouting": {
            "dictionary": { "owner": "HunterData", "field": "mShieldDataDic", "offset": 1488, "type": hunter_fields["<mShieldDataDic>k__BackingField"]["type"] },
            "shieldFields": [
                { "name": "MaxShield", "offset": 16, "type": shield_fields["MaxShield"]["type"] },
                { "name": "CurrentShield", "offset": 48, "type": shield_fields["CurrentShield"]["type"] },
            ],
            "selection": "HitDamageProcess enumerates the dictionary and uses the first yielded ShieldData entry when Count >= 1.",
            "equations": [
                "if CurrentShield < forwardedDamage: forwardedDamage -= CurrentShield; CurrentShield = 0",
                "else: CurrentShield -= forwardedDamage; forwardedDamage = 0",
                "nowHp = max(nowHp - forwardedDamage, 0)",
            ],
            "ordering": "shield routing occurs before the common nowHp subtraction",
        },
        "goldenVectors": {
            "modifierFamilies": [
                { "family": "proportional_add", "accumulator": 100, "factor": 0.2, "expected": 120 },
                { "family": "proportional_subtract", "accumulator": 100, "factor": 0.2, "expected": 80 },
                { "family": "summed_proportional_subtract", "accumulator": 100, "factor1": 0.1, "factor2": 0.2, "expected": 70 },
                { "family": "negative_percent_point_add", "accumulator": 100, "rawValue": 10, "count": 2, "expected": 99 },
                { "family": "negative_basis_point_add", "accumulator": 10000, "rawValue": 10, "count": 2, "expected": 9999 },
                { "family": "fixed_scale", "accumulator": 100, "factor": 0.75, "expected": 75 },
                { "family": "one_minus_percent_scale", "accumulator": 100, "rawPercent": 25, "expected": 75 },
                { "family": "direct_product_subtract", "accumulator": 100, "factor": 0.2, "expected": 80 },
                { "family": "percent_product_subtract", "accumulator": 100, "composite": 20, "expected": 80 },
            ],
            "armorSelector": [
                { "feel": 100.0, "nowFeel": 100.0, "factor": 1.2 },
                { "feel": 100.0, "nowFeel": 80.0, "factor": 1.2 },
                { "feel": 100.0, "nowFeel": 79.999, "factor": 1.1 },
                { "feel": 100.0, "nowFeel": 60.0, "factor": 1.1 },
                { "feel": 100.0, "nowFeel": 40.0, "factor": 1.0 },
                { "feel": 100.0, "nowFeel": 20.0, "factor": 0.9 },
                { "feel": 100.0, "nowFeel": 19.999, "factor": 0.8 },
            ],
            "shieldRouting": [
                { "currentShield": 30, "forwardedDamage": 50, "expectedShield": 0, "expectedHpDamage": 20 },
                { "currentShield": 50, "forwardedDamage": 30, "expectedShield": 20, "expectedHpDamage": 0 },
                { "currentShield": 30, "forwardedDamage": 30, "expectedShield": 0, "expectedHpDamage": 0 },
            ],
        },
        "unresolved": [
            "Dictionary ordering/ownership semantics when mShieldDataDic contains more than one entry.",
            "Product-facing names and writer semantics for the remaining obfuscated HunterCtrl/effect gates in the 32-stage pre-armor chain.",
            "Whether every optional stage is reachable in ordinary village combat versus PvP, boss, trait or late-game modes.",
        ],
    }


def main() -> None:
    args = parse_args()
    output = build()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote Hunter damage-tail v3 analysis to {args.output}")


if __name__ == "__main__":
    main()
