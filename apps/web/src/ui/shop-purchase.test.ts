import { describe, expect, it } from "vitest";
import type { BuildingSystemSnapshot, HunterRosterMemberSnapshot } from "../generated/protocol";
import { projectShopPurchase, shopDisplayItemMatchesHunter } from "./shop-purchase";

const recipe = { id: "recipe:weapon:0:rating:0", shop_id: "build_7", icon: "weapon.png", product_name: "Kiếm", material_costs: [], stock: 1, sale_price: 75, kind: "craft", required_level: 0, duration_ms: 0, cooldown_ms: 0, effect_value: 0, effect_kind: "", capacity: 10 };
const system = {
  evidence_label: "test",
  town_gold: 0,
  hunter_materials: 0,
  materials: 0,
  runes: 0,
  weapons: 0,
  armor: 0,
  hunter_equipment_purchases: 0,
  material_stocks: [],
  recipes: [recipe],
  display_items: [{ gear_instance_id: "00000000-0000-4000-8000-000000000001", shop_id: "build_7", product_id: recipe.id, product_name: "Kiếm", icon: "weapon.png", gear_kind: "weapon", visual_family: "H1", rating: 0, quality: 1, primary_stat: 88, option_type: 0, option_value: 0, sale_price: 75, ruleset: "web-rebuild-weapon-core-v1" }],
  definitions: [],
  states: [],
  instances: [],
} satisfies BuildingSystemSnapshot;
const hunter = { hunter_id: 1, display_name: "Sharon", class_family: "H1", gold: 100, hunt: { status: "idle" } } as HunterRosterMemberSnapshot;
const gearInstanceId = system.display_items[0].gear_instance_id;

describe("projectShopPurchase", () => {
  it("keeps the live display instance and its rolled stat attached to the purchase", () => {
    const projection = projectShopPurchase(system, [hunter], "build_7", recipe.id, gearInstanceId, 1);
    expect(projection?.displayItem?.primary_stat).toBe(88);
    expect(projection?.canPurchase).toBe(true);
  });

  it("fails closed when no buyer is selected or the selected Hunter is away", () => {
    expect(projectShopPurchase(system, [hunter], "build_7", recipe.id, gearInstanceId, null)?.blocker).toBe("buyer_required");
    const away = { ...hunter, hunt: { ...hunter.hunt, status: "hunting" as const } };
    expect(projectShopPurchase(system, [away], "build_7", recipe.id, gearInstanceId, 1)?.blocker).toBe("buyer_unavailable");
  });

  it("fails closed when a weapon does not match the selected Hunter class", () => {
    const incompatible = { ...hunter, class_family: "H2" };
    expect(projectShopPurchase(system, [incompatible], "build_7", recipe.id, gearInstanceId, 1)?.blocker).toBe("incompatible_weapon");
    expect(shopDisplayItemMatchesHunter(system.display_items[0], incompatible)).toBe(false);
    expect(shopDisplayItemMatchesHunter(system.display_items[0], hunter)).toBe(true);
  });

  it("fails closed when the selected rolled instance is no longer displayed", () => {
    expect(projectShopPurchase(system, [hunter], "build_7", recipe.id, "00000000-0000-4000-8000-000000000099", 1)?.blocker).toBe("out_of_stock");
  });
});
