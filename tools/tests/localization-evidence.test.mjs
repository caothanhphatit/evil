import assert from "node:assert/strict";
import { promises as fs } from "node:fs";
import path from "node:path";
import test from "node:test";

const repoRoot = path.resolve(import.meta.dirname, "../..");
test("localization evidence has aligned locale keys", async () => {
  const evidence = JSON.parse(await fs.readFile(path.join(repoRoot, "reverse-engineering/evidence/localization-evidence-v1.json"), "utf8"));
  const ids = evidence.sharedTable.entries.map((entry) => entry.id).sort((left, right) => left - right);
  assert.equal(new Set(ids).size, 3);
  for (const locale of Object.values(evidence.locales)) assert.deepEqual(locale.entries.map((entry) => entry.id).sort((left, right) => left - right), ids);
  assert.match(evidence.gaps.join(" "), /not the complete in-game localization corpus/);
});
