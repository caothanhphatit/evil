import assert from "node:assert/strict";
import { execFile as execFileCallback } from "node:child_process";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { promisify } from "node:util";

const execFile = promisify(execFileCallback);
const repoRoot = path.resolve(import.meta.dirname, "../..");
const verifier = path.join(repoRoot, "tools/verify-asset-copy.sh");

async function commandExists(command) {
  try {
    await execFile(command, ["-v"]);
    return true;
  } catch {
    return false;
  }
}

test("asset verifier extracts base_assets.apk from an XAPK", async (context) => {
  if (!await commandExists("zip") || !await commandExists("unzip")) {
    context.skip("zip and unzip are required for the recovery fixture");
    return;
  }

  const temporaryRoot = await fs.mkdtemp(path.join(os.tmpdir(), "evil-asset-recovery-"));
  try {
    const apkRoot = path.join(temporaryRoot, "apk");
    const sourceRoot = path.join(apkRoot, "assets");
    const destinationRoot = path.join(temporaryRoot, "destination");
    await fs.mkdir(path.join(sourceRoot, "bin/Data"), { recursive: true });
    await fs.writeFile(path.join(sourceRoot, "info.txt"), "fixture-info\n");
    await fs.writeFile(path.join(sourceRoot, "bin/Data/level1.split0"), "fixture-level\n");
    await fs.cp(sourceRoot, destinationRoot, { recursive: true });

    const apk = path.join(temporaryRoot, "base_assets.apk");
    const xapk = path.join(temporaryRoot, "fixture.xapk");
    await execFile("zip", ["-q", "-r", apk, "assets"], { cwd: apkRoot });
    await execFile("zip", ["-q", xapk, "base_assets.apk"], { cwd: temporaryRoot });

    const result = await execFile("bash", [
      verifier,
      "--xapk", xapk,
      "--destination", destinationRoot,
      "--expected-count", "2"
    ]);
    assert.match(result.stdout, /Asset copy verified: 2 files match byte-for-byte/);
  } finally {
    await fs.rm(temporaryRoot, { recursive: true, force: true });
  }
});
