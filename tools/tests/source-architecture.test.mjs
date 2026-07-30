import test from "node:test";
import assert from "node:assert/strict";
import {
  countSourceLines,
  evaluateLineBudget,
  forbiddenImports,
  forbiddenRustDependencies,
} from "../source-architecture-lib.mjs";

test("source line budget distinguishes target debt from a ratchet failure", () => {
  assert.equal(countSourceLines("one\ntwo\nthree"), 3);
  assert.deepEqual(evaluateLineBudget("file.ts", 8, 10, 5), {
    path: "file.ts",
    lines: 8,
    ceiling: 10,
    target: 5,
    exceedsCeiling: false,
    exceedsTarget: true,
  });
});

test("dependency checks detect inward-facing domain imports", () => {
  assert.deepEqual(forbiddenImports('import { x } from "../ui/x";', ["../ui"]), ["../ui/x"]);
  assert.deepEqual(forbiddenRustDependencies("use crate::persistence::Repo;", ["api", "persistence"]), ["persistence"]);
});
