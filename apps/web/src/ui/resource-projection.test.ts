import { describe, expect, it } from "vitest";
import { projectResourceBar } from "./resource-projection";

const snapshot = (screen: "boot" | "village" | "field", active: boolean, gold: number) => ({
  screen,
  migration_fixture_combat: { active, world: { gold } },
} as never);

describe("projectResourceBar", () => {
  it("exposes only authoritative fixture gold after boot", () => {
    expect(projectResourceBar(snapshot("field", true, 10))).toEqual({ gold: 10, evidenceBacked: true });
  });

  it("does not display a value before an active authoritative projection", () => {
    expect(projectResourceBar(snapshot("boot", true, 10))).toEqual({ gold: null, evidenceBacked: false });
    expect(projectResourceBar(snapshot("village", false, 10))).toEqual({ gold: 10, evidenceBacked: true });
  });
});
