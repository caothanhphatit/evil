import { describe, expect, it } from "vitest";
import {
  clampQuantity,
  missingCraftMaterial,
  remainingSharedCapacity,
  resolveServiceMaterialId,
  serviceMaterialRequired,
} from "./shop-crafting";

const infirmaryCosts = [
  { material_id: "currency:elemental", quantity: 150, output_quantity: 1 },
  { material_id: "currency:gem", quantity: 3, output_quantity: 1 },
  { material_id: "material:1", quantity: 1, output_quantity: 1 },
  { material_id: "material:16", quantity: 1, output_quantity: 10 },
];

describe("shop crafting projection", () => {
  it("clamps keyboard quantity input to integer bounds and fails closed for invalid input", () => {
    expect(clampQuantity("12.9", 1, 100)).toBe(12);
    expect(clampQuantity("0", 1, 100)).toBe(1);
    expect(clampQuantity("9999", 1, 100)).toBe(100);
    expect(clampQuantity("", 1, 100)).toBe(1);
  });
  it("auto-selects a fundable Infirmary material instead of an unavailable premium currency", () => {
    expect(resolveServiceMaterialId(
      infirmaryCosts,
      [
        { id: "material:1", town_quantity: 1 },
        { id: "material:16", town_quantity: 1 },
      ],
      1,
      null,
    )).toBe("material:1");
  });

  it("keeps an explicit material choice and applies recovered batch output conversion", () => {
    expect(resolveServiceMaterialId(
      infirmaryCosts,
      [{ id: "material:16", town_quantity: 1 }],
      10,
      "material:16",
    )).toBe("material:16");
    expect(serviceMaterialRequired(infirmaryCosts[3], 10)).toBe(1);
    expect(serviceMaterialRequired(infirmaryCosts[3], 11)).toBe(2);
  });

  it.each(["build_9", "build_12", "build_13", "build_19"])(
    "uses the same fundable-option rule for service building %s",
    () => {
      expect(resolveServiceMaterialId(
        [
          { material_id: "currency:gem", quantity: 3, output_quantity: 1 },
          { material_id: "material:11", quantity: 1, output_quantity: 1 },
        ],
        [{ id: "material:11", town_quantity: 1 }],
        1,
        null,
      )).toBe("material:11");
    },
  );

  it("requires every Alchemist, Blacksmith, and Jeweler input", () => {
    const costs = [
      { material_id: "material:1", quantity: 2, output_quantity: 1 },
      { material_id: "material:11", quantity: 3, output_quantity: 1 },
    ];
    expect(missingCraftMaterial(costs, [
      { id: "material:1", town_quantity: 4 },
      { id: "material:11", town_quantity: 5 },
    ], 2)).toBe("material:11");
    expect(missingCraftMaterial(costs, [
      { id: "material:1", town_quantity: 4 },
      { id: "material:11", town_quantity: 6 },
    ], 2)).toBeNull();
  });

  it("uses shared destination stock when evaluating capacity", () => {
    const selected = { family: "weapon", stock: 2, capacity: 5 };
    const recipes = [
      selected,
      { family: "weapon", stock: 2, capacity: 5 },
      { family: "armor", stock: 4, capacity: 5 },
    ];
    expect(remainingSharedCapacity(recipes, selected, (candidate, current) => candidate.family === current.family)).toBe(1);
  });

  it("treats an unconfigured zero capacity as unlimited", () => {
    const selected = { stock: 31, capacity: 0 };
    expect(remainingSharedCapacity([selected], selected, () => true)).toBe(Number.MAX_SAFE_INTEGER);
  });
});
