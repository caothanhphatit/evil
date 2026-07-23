import path from "node:path";
import { fileURLToPath } from "node:url";
import { validateOriginalFlowRelease } from "./original-flow-content-lib.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const release = await validateOriginalFlowRelease(repoRoot);
console.log(`Validated ${release.releaseId}: ${release.flows.length} ordered flows, runtime gate=${release.releaseGate.runnable}`);
