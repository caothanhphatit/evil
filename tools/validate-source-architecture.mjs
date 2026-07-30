import { readdir, readFile } from "node:fs/promises";
import { relative, resolve } from "node:path";
import {
  countSourceLines,
  evaluateLineBudget,
  forbiddenImports,
  forbiddenRustDependencies,
} from "./source-architecture-lib.mjs";

const root = resolve(import.meta.dirname, "..");

// Ceilings are ratchets for current debt; targets describe the intended module size.
const budgets = [
  ["apps/server/src/simulation/original_flow.rs", 150, 150],
  ["apps/server/src/simulation/monster_world.rs", 550, 500],
  ["apps/server/src/simulation/monster_world/hunter_tick.rs", 425, 400],
  ["apps/server/src/persistence.rs", 150, 150],
  ["apps/server/src/persistence/hunter_roster_save.rs", 350, 350],
  ["apps/server/src/buildings/mod.rs", 150, 150],
  ["apps/server/src/content/building_registry.rs", 150, 150],
  ["apps/web/src/main.ts", 50, 50],
  ["apps/web/src/app/game-application.ts", 2_700, 600],
  ["apps/web/src/game/visible-world.ts", 650, 600],
];

const failures = [];
const debt = [];

async function rustFiles(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) files.push(...await rustFiles(path));
    else if (entry.name.endsWith(".rs") && entry.name !== "tests.rs") files.push(path);
  }
  return files;
}

for (const [path, ceiling, target] of budgets) {
  const source = await readFile(resolve(root, path), "utf8");
  const result = evaluateLineBudget(path, countSourceLines(source), ceiling, target);
  if (result.exceedsCeiling) {
    failures.push(`${path}: ${result.lines} lines exceeds ratchet ceiling ${ceiling}`);
  } else if (result.exceedsTarget) {
    debt.push(`${path}: ${result.lines} lines; target <= ${target}`);
  }
}

const serverDomainFiles = [
  "apps/server/src/simulation/original_flow.rs",
  "apps/server/src/simulation/monster_world.rs",
  "apps/server/src/simulation/hunter_roster.rs",
];
for (const path of serverDomainFiles) {
  const source = await readFile(resolve(root, path), "utf8");
  const forbidden = forbiddenRustDependencies(source, ["api", "persistence", "coordination"]);
  if (forbidden.length > 0) failures.push(`${path}: domain depends on ${forbidden.join(", ")}`);
}

const webCoreFiles = [
  "apps/web/src/game/visible-world.ts",
  "apps/web/src/game/hunter-actor-presentation.ts",
  "apps/web/src/game/hunter-spine-presentation.ts",
];
for (const path of webCoreFiles) {
  const source = await readFile(resolve(root, path), "utf8");
  const forbidden = forbiddenImports(source, ["/ui/", "../ui", "/net/", "../net"]);
  if (forbidden.length > 0) failures.push(`${path}: core layer imports ${forbidden.join(", ")}`);
}

const packageRoots = [
  "apps/server/src/simulation/original_flow",
  "apps/server/src/simulation/monster_world",
  "apps/server/src/persistence",
];
const globImports = [];
for (const packageRoot of packageRoots) {
  for (const path of await rustFiles(resolve(root, packageRoot))) {
    const source = await readFile(path, "utf8");
    if (source.includes("use super::*;")) globImports.push(relative(root, path));
  }
}
if (globImports.length > 0) {
  failures.push(`package glob imports: ${globImports.length}; explicit dependencies are required`);
}

if (debt.length > 0) {
  console.warn("Architecture debt targets:\n" + debt.map((line) => `- ${line}`).join("\n"));
}
if (failures.length > 0) {
  console.error("Architecture validation failed:\n" + failures.map((line) => `- ${line}`).join("\n"));
  process.exitCode = 1;
} else {
  console.log("Architecture dependency directions and source-size ratchets are valid.");
}
