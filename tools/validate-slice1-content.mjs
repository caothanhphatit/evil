import path from "node:path";
import { validatePublishedRelease } from "./content-pipeline-lib.mjs";

const repoRoot = path.resolve(import.meta.dirname, "..");
const manifest = await validatePublishedRelease(repoRoot);
console.log(`Validated ${manifest.totalFiles} manifest references and checksums for ${manifest.releaseId}.`);
