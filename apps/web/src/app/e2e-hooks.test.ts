import { describe, expect, it } from "vitest";
import { isLocalE2eEnvironment } from "./e2e-hooks";

describe("game E2E hooks", () => {
  it("enables semantic hooks only for an explicit local E2E session", () => {
    expect(isLocalE2eEnvironment({ hostname: "127.0.0.1", search: "?e2e=1" })).toBe(true);
    expect(isLocalE2eEnvironment({ hostname: "localhost", search: "?debug=1&e2e=1" })).toBe(true);
    expect(isLocalE2eEnvironment({ hostname: "localhost", search: "" })).toBe(false);
    expect(isLocalE2eEnvironment({ hostname: "evil.poeviethoa.net", search: "?e2e=1" })).toBe(false);
  });
});
