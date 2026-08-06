import type { BuildingSystemSnapshot, HunterRosterMemberSnapshot, ShopDisplayItemSnapshot, ShopRecipeSnapshot } from "../generated/protocol";

export interface ShopBuyerOption {
  hunterId: number;
  displayName: string;
  classFamily: string;
  gold: number;
  available: boolean;
}

export interface ShopPurchaseProjection {
  recipe: ShopRecipeSnapshot;
  displayItem: ShopDisplayItemSnapshot | null;
  buyers: ShopBuyerOption[];
  selectedBuyer: ShopBuyerOption | null;
  canPurchase: boolean;
  blocker: "buyer_required" | "buyer_unavailable" | "incompatible_weapon" | "insufficient_gold" | "out_of_stock" | "price_unresolved" | null;
}

export function shopDisplayItemMatchesHunter(
  displayItem: ShopDisplayItemSnapshot | null | undefined,
  hunter: Pick<HunterRosterMemberSnapshot, "class_family"> | null | undefined,
): boolean {
  if (!displayItem || !hunter) return false;
  return displayItem.gear_kind !== "weapon" || displayItem.visual_family === hunter.class_family;
}

export function projectShopPurchase(
  system: BuildingSystemSnapshot,
  hunters: readonly HunterRosterMemberSnapshot[],
  shopId: string,
  productId: string,
  gearInstanceId: string | null,
  selectedHunterId: number | null,
): ShopPurchaseProjection | null {
  const recipe = system.recipes.find((candidate) => candidate.shop_id === shopId && candidate.id === productId);
  if (!recipe) return null;
  const displayItem = gearInstanceId === null
    ? null
    : system.display_items.find((candidate) => candidate.shop_id === shopId
      && candidate.product_id === productId
      && candidate.gear_instance_id === gearInstanceId) ?? null;
  const buyers = hunters.map((hunter) => ({
    hunterId: hunter.hunter_id,
    displayName: hunter.display_name,
    classFamily: hunter.class_family,
    gold: hunter.gold,
    available: hunter.hunt.status === "idle",
  }));
  const selectedBuyer = buyers.find((buyer) => buyer.hunterId === selectedHunterId) ?? null;
  const price = displayItem?.sale_price ?? recipe.sale_price;
  const isGear = recipe.id.startsWith("recipe:weapon:")
    || recipe.id.startsWith("recipe:armor:")
    || recipe.id.startsWith("recipe:gloves:")
    || recipe.id.startsWith("recipe:boots:")
    || recipe.id.startsWith("recipe:ring:")
    || recipe.id.startsWith("recipe:necklace:")
    || recipe.id.startsWith("recipe:belt:");
  const blocker = recipe.stock < 1 || (isGear && !displayItem)
    ? "out_of_stock"
    : price < 1
      ? "price_unresolved"
      : !selectedBuyer
        ? "buyer_required"
        : !selectedBuyer.available
          ? "buyer_unavailable"
          : displayItem?.gear_kind === "weapon" && !shopDisplayItemMatchesHunter(displayItem, { class_family: selectedBuyer.classFamily })
            ? "incompatible_weapon"
          : selectedBuyer.gold < price
            ? "insufficient_gold"
            : null;
  return { recipe, displayItem, buyers, selectedBuyer, canPurchase: blocker === null, blocker };
}
