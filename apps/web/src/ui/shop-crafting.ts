export interface CraftMaterialCost {
  material_id: string;
  quantity: number;
  output_quantity: number;
}

export interface TownMaterialStock {
  id: string;
  town_quantity: number;
}

export interface StockedRecipe {
  stock: number;
  capacity: number;
}

/** Normalize keyboard input before it reaches a craft command or preview. */
export function clampQuantity(raw: string | number, minimum: number, maximum: number, fallback = minimum): number {
  const parsed = typeof raw === "number" ? raw : Number.parseInt(raw, 10);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.max(minimum, Math.min(maximum, Math.trunc(parsed)));
}

export function townMaterialQuantity(
  stocks: readonly TownMaterialStock[],
  materialId: string,
): number {
  return stocks.find((stock) => stock.id === materialId)?.town_quantity ?? 0;
}

export function serviceMaterialRequired(cost: CraftMaterialCost, outputQuantity: number): number {
  const outputPerBatch = Math.max(1, cost.output_quantity);
  const batches = Math.ceil(Math.max(1, outputQuantity) / outputPerBatch);
  return cost.quantity * batches;
}

export function canFundServiceMaterial(
  cost: CraftMaterialCost,
  stocks: readonly TownMaterialStock[],
  outputQuantity: number,
): boolean {
  return townMaterialQuantity(stocks, cost.material_id) >= serviceMaterialRequired(cost, outputQuantity);
}

export function resolveServiceMaterialId(
  costs: readonly CraftMaterialCost[],
  stocks: readonly TownMaterialStock[],
  outputQuantity: number,
  selectedMaterialId: string | null,
): string | null {
  if (selectedMaterialId && costs.some((cost) => cost.material_id === selectedMaterialId)) {
    return selectedMaterialId;
  }
  return costs.find((cost) => canFundServiceMaterial(cost, stocks, outputQuantity))?.material_id
    ?? costs[0]?.material_id
    ?? null;
}

export function remainingSharedCapacity<T extends StockedRecipe>(
  recipes: readonly T[],
  selectedRecipe: T,
  sharesStock: (candidate: T, selected: T) => boolean,
): number {
  // The server uses capacity=0 for a shop with no configured production cap.
  // Treat that contract as unlimited instead of turning an absent catalog row
  // into a permanently disabled Produce button.
  if (selectedRecipe.capacity <= 0) return Number.MAX_SAFE_INTEGER;
  const stocked = recipes
    .filter((candidate) => sharesStock(candidate, selectedRecipe))
    .reduce((total, candidate) => total + candidate.stock, 0);
  return Math.max(0, selectedRecipe.capacity - stocked);
}

export function requiredCraftMaterials(
  costs: readonly CraftMaterialCost[],
  outputQuantity: number,
): Array<{ materialId: string; quantity: number }> {
  return costs.map((cost) => ({
    materialId: cost.material_id,
    quantity: cost.quantity * Math.max(1, outputQuantity),
  }));
}

export function missingCraftMaterial(
  costs: readonly CraftMaterialCost[],
  stocks: readonly TownMaterialStock[],
  outputQuantity: number,
): string | null {
  return requiredCraftMaterials(costs, outputQuantity)
    .find((cost) => townMaterialQuantity(stocks, cost.materialId) < cost.quantity)
    ?.materialId ?? null;
}
