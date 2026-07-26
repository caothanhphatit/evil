/** Source-confirmed split between potion production and potion retail. */
export const POTION_SHOP_BUILDING_ID = "build_11" as const;
export const ALCHEMIST_BUILDING_ID = "build_14" as const;

export function isPotionBuilding(buildingId: string | null): boolean {
  return buildingId === POTION_SHOP_BUILDING_ID || buildingId === ALCHEMIST_BUILDING_ID;
}
