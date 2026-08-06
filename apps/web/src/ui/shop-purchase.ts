import type { BuildingSystemSnapshot, HunterRosterMemberSnapshot, ShopDisplayItemSnapshot, ShopRecipeSnapshot } from "../generated/protocol";

export interface ShopBuyerOption {
  hunterId: number;
  displayName: string;
  gold: number;
  available: boolean;
}

export interface ShopPurchaseProjection {
  recipe: ShopRecipeSnapshot;
  displayItem: ShopDisplayItemSnapshot | null;
  buyers: ShopBuyerOption[];
  selectedBuyer: ShopBuyerOption | null;
  canPurchase: boolean;
  blocker: "buyer_required" | "buyer_unavailable" | "insufficient_gold" | "out_of_stock" | "price_unresolved" | null;
}

export function projectShopPurchase(
  system: BuildingSystemSnapshot,
  hunters: readonly HunterRosterMemberSnapshot[],
  shopId: string,
  productId: string,
  selectedHunterId: number | null,
): ShopPurchaseProjection | null {
  const recipe = system.recipes.find((candidate) => candidate.shop_id === shopId && candidate.id === productId);
  if (!recipe) return null;
  const displayItem = system.display_items.find((candidate) => candidate.shop_id === shopId && candidate.product_id === productId) ?? null;
  const buyers = hunters.map((hunter) => ({
    hunterId: hunter.hunter_id,
    displayName: hunter.display_name,
    gold: hunter.gold,
    available: hunter.hunt.status === "idle",
  }));
  const selectedBuyer = buyers.find((buyer) => buyer.hunterId === selectedHunterId) ?? null;
  const price = displayItem?.sale_price ?? recipe.sale_price;
  const blocker = recipe.stock < 1
    ? "out_of_stock"
    : price < 1
      ? "price_unresolved"
      : !selectedBuyer
        ? "buyer_required"
        : !selectedBuyer.available
          ? "buyer_unavailable"
          : selectedBuyer.gold < price
            ? "insufficient_gold"
            : null;
  return { recipe, displayItem, buyers, selectedBuyer, canPurchase: blocker === null, blocker };
}
