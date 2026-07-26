import { describe, expect, it } from "vitest";
import { ALCHEMIST_BUILDING_ID, POTION_SHOP_BUILDING_ID, isPotionBuilding } from "./potion-shop-routes";

describe("potion shop routes", () => {
  it("keeps potion retail separate from alchemist production", () => {
    expect(POTION_SHOP_BUILDING_ID).toBe("build_11");
    expect(ALCHEMIST_BUILDING_ID).toBe("build_14");
    expect(isPotionBuilding(POTION_SHOP_BUILDING_ID)).toBe(true);
    expect(isPotionBuilding(ALCHEMIST_BUILDING_ID)).toBe(true);
    expect(isPotionBuilding("build_12")).toBe(false);
  });
});
