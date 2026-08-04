import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const repositoryRoot = resolve(import.meta.dirname, "../../../..");

describe("gear crafting popup", () => {
  it("does not expose destination capacity or disable production because of it", async () => {
    const [shell, renderer] = await Promise.all([
      readFile(resolve(repositoryRoot, "apps/web/src/app/shell.ts"), "utf8"),
      readFile(resolve(repositoryRoot, "apps/web/src/app/building-renderer.ts"), "utf8"),
    ]);

    expect(shell).not.toContain('id="gear-storage-label"');
    expect(renderer).not.toContain("context.gearStorageLabel");
    expect(renderer).not.toContain("craft.shop_stock");
    expect(renderer).toContain('context.gearCreateSubmit.disabled = context.pendingCraft !== null || context.gearPopupMode !== "craft" || !craftable;');
  });

  it("plays a visible processing state after an accepted craft result", async () => {
    const [source, feedback, styles] = await Promise.all([
      readFile(resolve(repositoryRoot, "apps/web/src/app/game-application.ts"), "utf8"),
      readFile(resolve(repositoryRoot, "apps/web/src/app/intent-feedback.ts"), "utf8"),
      readFile(resolve(repositoryRoot, "apps/web/src/styles.css"), "utf8"),
    ]);
    expect(feedback).toContain('result.intent === "craft_shop_item"');
    expect(feedback).toContain('const stillShowingRequest = !popup.hidden && context.selectedRecipe?.id === pending.recipeId;');
    expect(feedback).toContain('popup.classList.add("crafting")');
    expect(source).toContain('canCraft: () => !buildingContext.pendingCraft && Boolean(buildingContext.selectedBuildingInstanceId && buildingContext.selectedRecipe)');
    expect(styles).toContain("@keyframes craft-frame-pulse");
  });
});
