#!/usr/bin/env python3
"""Prove the current native object-flow boundary for original unique gear drops."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CALLGRAPH = ROOT / "reverse-engineering/evidence/original-reward-progression-callgraph-v2.json"
DEFAULT_ARITHMETIC = ROOT / "reverse-engineering/evidence/original-reward-progression-arithmetic-v3.json"
DEFAULT_METHODS = ROOT / "reverse-engineering/evidence/original-reward-progression-methods-api35-v1.json"
DEFAULT_HELPERS = ROOT / "reverse-engineering/evidence/original-reward-progression-helpers-api35-v1.json"
DEFAULT_MATERIAL = ROOT / "reverse-engineering/evidence/original-reward-material-full-api35-v1.json"
DEFAULT_OUTPUT = ROOT / "reverse-engineering/evidence/original-reward-progression-unique-gear-boundary-v7.json"


def source(path: Path, source_id: str) -> dict[str, Any]:
    body = path.read_bytes()
    return {"id": source_id, "path": path.resolve().relative_to(ROOT).as_posix(), "bytes": len(body), "sha256": hashlib.sha256(body).hexdigest()}


def find_method(paths: list[Path], name: str) -> dict[str, Any]:
    candidates = []
    for path in paths:
        document = json.loads(path.read_text())
        candidates.extend(row for row in document["record"]["payload"]["methods"] if row["className"] == "HunterCtrl" and row["methodName"] == name)
    row = max(candidates, key=lambda value: len(value["candidates"][0]["codeHex"]))
    candidate = row["candidates"][0]
    return {"parameterTypes": row["parameterTypes"], "returnType": row["returnType"], "nativeSizeBytes": candidate["nativeSizeBytes"], "codeTruncated": candidate["codeTruncated"], "bodySha256": hashlib.sha256(bytes.fromhex(candidate["codeHex"])).hexdigest()}


def build(callgraph_path: Path, arithmetic_path: Path, methods_path: Path, helpers_path: Path, material_path: Path) -> dict[str, Any]:
    callgraph = json.loads(callgraph_path.read_text())
    arithmetic = json.loads(arithmetic_path.read_text())
    rows = {row["method"]: row for row in callgraph["methods"]}
    reward = rows["RewardMetrial"]
    if reward["knownDirectCallCounts"] != {"HunterCtrl.GHPHHEFFNKN/2": 6, "HunterCtrl.LDHAEMDJCFF/5": 17, "UnityEngine.Random.Range(Int32,Int32)": 50}:
        raise ValueError("RewardMetrial call boundary changed")
    unique = arithmetic["uniqueDropTrace"]
    if unique["uniqueLevelDirectRowAccess"] is not None or unique["adminDropUniqueGearRowAccess"] is not None or unique["poolLinkage"] is not None:
        raise ValueError("pass-3 unique boundary changed")
    methods = [methods_path, helpers_path, material_path]
    signatures = {name: find_method(methods, name) for name in ("RewardMetrial", "LDHAEMDJCFF", "GHPHHEFFNKN")}
    if signatures["RewardMetrial"]["codeTruncated"] or signatures["RewardMetrial"]["nativeSizeBytes"] != 30732:
        raise ValueError("full RewardMetrial body missing")
    if signatures["LDHAEMDJCFF"]["parameterTypes"] != ["System.Int32", "System.Int32", "UnityEngine.Vector3", "System.Boolean", "System.Boolean"]:
        raise ValueError("LDHAEMDJCFF signature changed")
    if signatures["GHPHHEFFNKN"]["parameterTypes"] != ["CodeStage.AntiCheat.ObscuredTypes.ObscuredInt", "CodeStage.AntiCheat.ObscuredTypes.ObscuredBool"]:
        raise ValueError("GHPHHEFFNKN signature changed")
    return {
        "schemaVersion": 7,
        "contractType": "original-reward-progression-unique-gear-boundary-evidence",
        "runtimeCompatibility": "evidence-only",
        "sources": [source(callgraph_path, "reward-callgraph-v2"), source(arithmetic_path, "reward-arithmetic-v3"), source(methods_path, "reward-methods"), source(helpers_path, "reward-helpers"), source(material_path, "reward-material-full")],
        "capturedMethods": signatures,
        "confirmedCallerFacts": {
            "RewardMetrialNativeBytes": 30732,
            "knownCalls": reward["knownDirectCallCounts"],
            "boundAdminEvilDataFields": [row["field"] for row in unique["confirmedAdminEvilRowAccesses"]],
            "boundUniqueLevelAccess": None,
            "boundAdminDropUniqueGearDataAccess": None,
        },
        "exactBlockingBoundary": [
            "The complete RewardMetrial object-flow binds AdminEvilData.metIdx, metCount, metPercent and type, but no AdminEvilData.uniqueLevel read.",
            "All 17 LDHAEMDJCFF calls cross a signature containing only two Int32 values, Vector3 and two Boolean values; no AdminEvilData or AdminDropUniqueGearData object crosses that boundary.",
            "GHPHHEFFNKN receives only ObscuredInt and ObscuredBool, including its one call from LDHAEMDJCFF; it cannot recover an AdminEvilData row identity from its typed arguments.",
            "Therefore the captured call chain cannot mechanically bind uniqueLevel to an AdminDropUniqueGearData row without identifying an additional singleton/static lookup and its returned object type.",
        ],
        "unresolvedOutputs": {"uniqueLevelToPool": None, "dropRangeOrder": None, "dropCutOrder": None, "gearPercentDenominator": None, "gearTypeRngOrder": None, "gearIndexRngOrder": None},
        "implementationBoundary": {"liveIntegrationAllowed": False, "arrayOrderFallbackAllowed": False, "requiredNextEvidence": "Targeted runtime/native capture of the singleton/static lookup that returns AdminDropUniqueGearData, including input key, returned row pointer/type, and the immediately following RNG comparisons."},
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--callgraph", type=Path, default=DEFAULT_CALLGRAPH)
    parser.add_argument("--arithmetic", type=Path, default=DEFAULT_ARITHMETIC)
    parser.add_argument("--methods", type=Path, default=DEFAULT_METHODS)
    parser.add_argument("--helpers", type=Path, default=DEFAULT_HELPERS)
    parser.add_argument("--material", type=Path, default=DEFAULT_MATERIAL)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    evidence = build(args.callgraph, args.arithmetic, args.methods, args.helpers, args.material)
    args.output.write_text(json.dumps(evidence, ensure_ascii=True, indent=2) + "\n")
    print(f"Wrote pass-7 unique-gear boundary evidence to {args.output}")


if __name__ == "__main__":
    main()
