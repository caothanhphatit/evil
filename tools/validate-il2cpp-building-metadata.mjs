#!/usr/bin/env node

import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const evidencePath = path.join(repoRoot, "reverse-engineering/evidence/il2cpp-building-metadata-v1.json");
const evidence = JSON.parse(await fs.readFile(evidencePath, "utf8"));

const expectedSources = {
  metadata: {
    sha256: "ebbadaf6d94d838037b33bd77d60d861d84cc309b1a148383b3800d5e294b63e",
  },
  binary: {
    sha256: "7ca72697375e87703f03867ccfcf8d7ddec1e5cca8b9cd9de4736b666cd8390f",
  },
};
const requiredCleanNames = [
  "AdminBuildData",
  "AdminTradeWagonData",
  "BuildingData",
  "GearCreatePop",
  "RequestPop",
];

if (
  evidence.schemaVersion !== 1
  || evidence.contractType !== "il2cpp-building-metadata-evidence"
  || evidence.runtimeCompatibility !== "evidence-only"
) {
  throw new Error("Invalid IL2CPP building metadata evidence identity");
}
if (evidence.sources?.metadataVersion !== 39 || evidence.sources?.unityVersion !== "6000.3.9f1") {
  throw new Error("Unexpected IL2CPP or Unity version");
}
if (evidence.assemblyCSharpImage?.typeCount !== 2043) {
  throw new Error(`Unexpected Assembly-CSharp type count: ${evidence.assemblyCSharpImage?.typeCount}`);
}

for (const [sourceName, expected] of Object.entries(expectedSources)) {
  const sourceRecord = evidence.sources?.[sourceName];
  if (sourceRecord?.sha256 !== expected.sha256) {
    throw new Error(`Unexpected recorded ${sourceName} checksum`);
  }
  const sourceBytes = await fs.readFile(path.join(repoRoot, sourceRecord.path));
  const actualHash = createHash("sha256").update(sourceBytes).digest("hex");
  if (sourceBytes.length !== sourceRecord.bytes || actualHash !== expected.sha256) {
    throw new Error(`${sourceName} source does not match the evidence record`);
  }
}

const buildingPop = evidence.candidateTypes.find((candidate) => candidate.name?.value === "BuildingPop");
if (!buildingPop) {
  throw new Error("BuildingPop is missing from the extracted candidate types");
}
if (!buildingPop.methods.some((method) => method.token === 0x060021aa)) {
  throw new Error("BuildingPop method tokens do not match the metadata v39 record layout");
}
const cleanNames = new Set(evidence.cleanNameCatalog);
for (const name of requiredCleanNames) {
  if (!cleanNames.has(name)) throw new Error(`Clean name catalog is missing ${name}`);
}

console.log(
  `Validated IL2CPP building metadata: ${evidence.assemblyCSharpImage.typeCount} Assembly-CSharp types, ${evidence.candidateTypes.length} building candidates.`,
);
