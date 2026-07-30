import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { canSubmitGearEnhancement, enhancementTargetLevel, GEAR_ENHANCEMENT_MODES, projectGearEnhancement } from "./gear-enhancement";

const repositoryRoot = resolve(import.meta.dirname, "../../../..");

describe("gear enhancement evidence boundary", () => {
  it("uses the four target modes shown in the supplied enhancement UI", () => {
    expect(GEAR_ENHANCEMENT_MODES).toEqual(["single", "to_10", "to_15", "to_20"]);
  });

  it("keeps the +20 cap and unresolved level visible without inventing a cost", () => {
    const preview = projectGearEnhancement({
      product_id: "product:weapon:0",
      level: null,
      max_level: 20,
      instance_id: null,
      evidence_state: "unresolved",
    }, "single");
    expect(preview.maxLevel).toBe(20);
    expect(preview.currentLevel).toBeNull();
    expect(preview.costResolved).toBe(false);
    expect(preview.blockers).toContain("enhancement_probability_binding");
    expect(canSubmitGearEnhancement(preview)).toBe(false);
  });

  it("renders selection and configuration as separate source-style steps", async () => {
    const main = await Promise.all([
      readFile(resolve(repositoryRoot, "apps/web/src/app/building-renderer.ts"), "utf8"),
      readFile(resolve(repositoryRoot, "apps/web/src/app/game-application.ts"), "utf8"),
    ]).then((parts) => parts.join("\n"));
    const styles = await readFile(resolve(repositoryRoot, "apps/web/src/styles.css"), "utf8");
    expect(main).toContain("enhancement-workspace");
    expect(main).toContain("enhancement-stage");
    expect(main).toContain("enhancementView: GearEnhancementView");
    expect(main).toContain('if (context.enhancementView === "configure" && selected) shell.append(configureControls)');
    expect(main).toContain('buildingContext.enhancementView = "configure"');
    expect(main).toContain("enhancement-cost-row unresolved");
    expect(main).toContain("enhancement-inventory");
    expect(main).toContain("enhancement-processing");
    expect(main).toContain("enhancement-result");
    expect(main).toContain("ENHANCEMENT_FORGE_BUILDING_IDS.includes(context.selectedBuildingId");
    expect(styles).toContain(".building-panel.source-popup.enhancement-forge-ui > #building-panel-close");
    expect(styles).toContain(".building-panel.source-popup.enhancement-forge-ui .building-actions .source-green-button");
    expect(main).not.toContain("Unavailable until evidence capture:");
  });

  it("projects each requested target without exceeding the +20 cap", () => {
    expect(enhancementTargetLevel(7, 20, "single")).toBe(8);
    expect(enhancementTargetLevel(7, 20, "to_10")).toBe(10);
    expect(enhancementTargetLevel(12, 20, "to_10")).toBe(12);
    expect(enhancementTargetLevel(12, 20, "to_15")).toBe(15);
    expect(enhancementTargetLevel(18, 20, "to_20")).toBe(20);
    expect(enhancementTargetLevel(null, 20, "to_20")).toBeNull();
  });

  it("does not auto-select the first owned gear when the forge opens", async () => {
    const main = await readFile(resolve(repositoryRoot, "apps/web/src/app/building-renderer.ts"), "utf8");
    expect(main).toContain("selectedEnhancementGearKey = null");
    expect(main).not.toContain("selectedEnhancementGearKey = ownedRows[0]?.key");
  });

  it("does not synthesize an enhancement badge for unresolved levels", async () => {
    const main = await readFile(resolve(repositoryRoot, "apps/web/src/app/building-renderer.ts"), "utf8");
    expect(main).toContain("if (level !== null)");
    expect(main).toContain("if (owned.gear.level !== null)");
    expect(main).not.toContain('badge.textContent = "+20"');
  });
});
