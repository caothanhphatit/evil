import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const evidencePath = path.join(repoRoot, "reverse-engineering/evidence/localization-evidence-v1.json");
const evidence = JSON.parse(await fs.readFile(evidencePath, "utf8"));
if (evidence.schemaVersion !== 1 || evidence.manifestType !== "unity-localization-evidence" || evidence.runtimeCompatibility !== "evidence-only") {
  throw new Error("Invalid localization evidence identity");
}
const sharedIds = new Set(evidence.sharedTable.entries.map((entry) => entry.id));
if (sharedIds.size !== evidence.coverage.sharedKeys || sharedIds.size === 0) throw new Error("Shared localization key coverage is invalid");
const sharedById = new Map(evidence.sharedTable.entries.map((entry) => [entry.id, entry.key]));
for (const [locale, table] of Object.entries(evidence.locales)) {
  const ids = table.entries.map((entry) => entry.id);
  if (new Set(ids).size !== ids.length || ids.length !== sharedIds.size) throw new Error(`Locale coverage is invalid for ${locale}`);
  for (const entry of table.entries) if (!sharedIds.has(entry.id) || sharedById.get(entry.id) !== entry.key) throw new Error(`Locale key mismatch for ${locale}/${entry.id}`);
  const source = await fs.readFile(path.join(repoRoot, table.source.path));
  if (source.length !== table.source.bytes || createHash("sha256").update(source).digest("hex") !== table.source.sha256) throw new Error(`Locale source checksum mismatch: ${locale}`);
}
const shared = await fs.readFile(path.join(repoRoot, evidence.sharedTable.source.path));
if (shared.length !== evidence.sharedTable.source.bytes || createHash("sha256").update(shared).digest("hex") !== evidence.sharedTable.source.sha256) throw new Error("Shared localization source checksum mismatch");
if (evidence.coverage.locales !== Object.keys(evidence.locales).length || !evidence.gaps.length) throw new Error("Localization coverage metadata is stale");
console.log(`Validated localization evidence: ${sharedIds.size} shared keys across ${Object.keys(evidence.locales).length} locales; corpus remains explicitly partial.`);
