import { describe, expect, it } from "vitest";
import { ORIGINAL_UI_LABEL_EVIDENCE, originalUiLabel } from "./original-ui-labels";

describe("original localUI labels", () => {
  it("projects the recovered locale and substitutes Unity placeholders", () => {
    expect(originalUiLabel("buildpop_7", "vi-VN")).toBe("Nâng Cấp");
    expect(originalUiLabel("buildpop_9", "en-US", [5])).toBe("Town Hall Lv.5 or higher required.");
  });

  it("keeps an explicit evidence locator", () => {
    expect(ORIGINAL_UI_LABEL_EVIDENCE).toMatchObject({ source: "sharedassets0.assets", pathId: 9082, worksheet: "localUI" });
  });
});
