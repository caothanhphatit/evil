import assert from "node:assert/strict";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { assertPublishedAsset, buildRelease, validatePublishedRelease } from "../content-pipeline-lib.mjs";

const repoRoot = path.resolve(import.meta.dirname, "../..");

test("slice 1 selection builds from pinned source and Unity inventory", async () => {
  const first = await buildRelease(repoRoot);
  const second = await buildRelease(repoRoot);
  assert.deepEqual(first.manifest, second.manifest);
  assert.equal(first.manifest.totalFiles, 19);
  assert.equal(first.manifest.contentUnits.length, 6);
  assert.ok(first.manifest.contentUnits.find((unit) => unit.id === "actor.monster.a01.level1")
    .requiredAnimations.includes("atk"));
  assert.equal(first.manifest.contentUnits.find((unit) => unit.id === "effect.candidate.blade-dance-hit").presentationOnly, true);
  assert.equal(first.manifest.contentUnits.find((unit) => unit.id === "audio.candidates.slice-001").status, "unbound-candidate");
});

test("published release resolves every manifest reference and checksum", async () => {
  const manifest = await validatePublishedRelease(repoRoot);
  assert.equal(manifest.releaseId, "slice-001-combat-v1");
});

test("checksum validation rejects a modified published asset", async () => {
  const { manifest, payloads } = await buildRelease(repoRoot);
  const asset = manifest.assets[0];
  const temporaryRoot = await fs.mkdtemp(path.join(os.tmpdir(), "evil-hunter-assets-"));
  try {
    const output = path.join(temporaryRoot, asset.outputPath);
    await fs.mkdir(path.dirname(output), { recursive: true });
    const modified = Buffer.from(payloads.get(asset.outputPath));
    modified[0] ^= 0xff;
    await fs.writeFile(output, modified);
    await assert.rejects(assertPublishedAsset(asset, temporaryRoot), /checksum differs/);
  } finally {
    await fs.rm(temporaryRoot, { recursive: true, force: true });
  }
});
