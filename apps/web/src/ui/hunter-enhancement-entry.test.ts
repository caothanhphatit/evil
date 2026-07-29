import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import {
  createOpenHunterEnhancementIntent,
  projectHunterEnhancementInteraction,
  type HunterEnhancementAuthoritySnapshot,
} from "./hunter-enhancement-entry";

const arrived: HunterEnhancementAuthoritySnapshot = {
  hunterEntityId: "village-hunter-7",
  workflow: "gear_enhancement",
  phase: "waiting_for_interaction",
  buildingId: "build_15",
  buildingInstanceId: "forge-instance-1",
};

describe("Hunter enhancement interaction entry", () => {
  it("shows interaction readiness only after the authoritative arrival phase", () => {
    expect(projectHunterEnhancementInteraction({ ...arrived, phase: "traveling" })).toEqual({
      mode: "traveling",
      hunterEntityId: "village-hunter-7",
      buildingId: "build_15",
      buildingInstanceId: "forge-instance-1",
    });
    expect(projectHunterEnhancementInteraction(arrived)).toEqual({
      mode: "ready",
      hunterEntityId: "village-hunter-7",
      buildingId: "build_15",
      buildingInstanceId: "forge-instance-1",
    });
  });

  it("fails closed for another building or an unresolved instance", () => {
    expect(projectHunterEnhancementInteraction({ ...arrived, buildingId: "build_5" })).toEqual({ mode: "hidden" });
    expect(projectHunterEnhancementInteraction({ ...arrived, buildingInstanceId: null })).toEqual({ mode: "hidden" });
    expect(projectHunterEnhancementInteraction(null)).toEqual({ mode: "hidden" });
  });

  it("opens the enhancement UI only from an explicit icon interaction", () => {
    const ready = projectHunterEnhancementInteraction(arrived);
    expect(createOpenHunterEnhancementIntent(ready)).toEqual({
      type: "open_hunter_gear_enhancement",
      hunterEntityId: "village-hunter-7",
      buildingInstanceId: "forge-instance-1",
    });
    expect(createOpenHunterEnhancementIntent({ mode: "hidden" })).toBeNull();
    expect(createOpenHunterEnhancementIntent({
      mode: "traveling",
      hunterEntityId: "village-hunter-7",
      buildingId: "build_15",
      buildingInstanceId: "forge-instance-1",
    })).toBeNull();
  });

  it("renders separate travel and arrival presentations without a success popup", async () => {
    const main = await readFile(new URL("../main.ts", import.meta.url), "utf8");
    const styles = await readFile(new URL("../styles.css", import.meta.url), "utf8");
    expect(main).toContain('state.mode === "traveling"');
    expect(main).toContain('indicator.className = "hunter-enhancement-travel-indicator"');
    expect(main).toContain('entity?.interaction_prompt_key !== "hunter_enhancement_ready"');
    expect(main).not.toContain('showPanelMessage("Đã ra lệnh cường hóa"');
    expect(styles).toContain(".hunter-enhancement-travel-indicator");
    expect(styles).toContain("pointer-events: none");
  });
});
