import { describe, expect, it } from "vitest";
import { projectEntryPresentation } from "./entry-flow";

describe("game entry flow", () => {
  it("keeps a restored world covered until the player logs in", () => {
    expect(projectEntryPresentation("login")).toEqual({
      showLogin: true,
      showLoading: false,
      renderWorld: false,
      enableGameUi: false,
    });
  });

  it("renders the world only behind the dedicated loading screen", () => {
    expect(projectEntryPresentation("loading")).toEqual({
      showLogin: false,
      showLoading: true,
      renderWorld: true,
      enableGameUi: false,
    });
  });

  it("enables gameplay only after loading completes", () => {
    expect(projectEntryPresentation("game")).toEqual({
      showLogin: false,
      showLoading: false,
      renderWorld: true,
      enableGameUi: true,
    });
  });
});
