import { describe, expect, it } from "vitest";
import { animationFor, animationLoops } from "./animation";

describe("authoritative state animation projection", () => {
  it("maps every hunter state and facing to verified clips", () => {
    expect(animationFor("hunter", "idle", "front")).toBe("hunter_stay");
    expect(animationFor("hunter", "moving", "back")).toBe("hunter_walk_back");
    expect(animationFor("hunter", "attacking", "front")).toBe("h1_hit");
    expect(animationFor("hunter", "dead", "back")).toBe("hunter_die");
    expect(animationFor("hunter", "reviving", "front")).toBe("hunter_dying");
  });

  it("maps the approved monster fixture without inventing effects or audio", () => {
    expect(animationFor("monster", "attacking", "back")).toBe("atk_b");
    expect(animationFor("monster", "dead", "front")).toBe("die");
    expect(animationLoops("attacking")).toBe(false);
    expect(animationLoops("moving")).toBe(true);
  });
});
