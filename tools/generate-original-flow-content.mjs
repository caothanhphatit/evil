import { promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  buildOriginalFlowRelease,
  ORIGINAL_FLOW_RELEASE,
  serializeOriginalFlowRelease
} from "./original-flow-content-lib.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const release = await buildOriginalFlowRelease(repoRoot);
await fs.mkdir(path.dirname(path.join(repoRoot, ORIGINAL_FLOW_RELEASE)), { recursive: true });
await fs.writeFile(path.join(repoRoot, ORIGINAL_FLOW_RELEASE), serializeOriginalFlowRelease(release));
console.log(`Generated ${ORIGINAL_FLOW_RELEASE} (${release.assets.length} evidence assets, ${release.releaseGate.blockingBindingIds.length} blockers)`);
