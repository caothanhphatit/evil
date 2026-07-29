/** Route contract recovered from GearCreatePop and the Blacksmith screenshots.
 *  This module deliberately keeps item sprites nullable: a building thumbnail
 *  or a guessed gear icon must never be presented as an item asset.
 */
export const BLACKSMITH_BUILDING_IDS = ["build_10"] as const;
/** Enhancement Forge route is separate from Blacksmith crafting. */
export const ENHANCEMENT_FORGE_BUILDING_IDS = ["build_15"] as const;
export const WEAPON_SHOP_BUILDING_IDS = ["build_7"] as const;
export const ARMOR_SHOP_BUILDING_IDS = ["build_8"] as const;
export const ACCESSORY_SHOP_BUILDING_IDS = ["build_20"] as const;
export const JEWELER_BUILDING_IDS = ["build_21"] as const;

export const BLACKSMITH_GEAR_TABS = ["weapon", "armor", "gloves", "boots"] as const;
export const JEWELER_GEAR_TABS = ["ring", "necklace", "belt"] as const;
/** Includes packaged helmet evidence; the tab remains hidden until runtime UI binding is proven. */
export const ALL_GEAR_KINDS = [...BLACKSMITH_GEAR_TABS, ...JEWELER_GEAR_TABS, "helmet"] as const;

export const GEAR_BUILDING_PROGRESSION = {
  build_10: { townHallLevels: [2, 5, 7, 9, 11], gold: [660, 5280, 17820, 53460, 160380], materialQuantity: 10 },
  build_7: { townHallLevels: [2, 5, 7, 9, 11], gold: [660, 5280, 17820, 53460, 160380], materialQuantity: 10 },
  build_8: { townHallLevels: [3, 6, 8, 10, 12], gold: [960, 8640, 25920, 77760, 233280], materialQuantity: 15 },
} as const;

export const GEAR_CREATE_LAYOUT = {
  controller: "GearCreatePop",
  width: 450,
  height: 950,
  panelSprite: "popup_bg_9",
  contentSprite: "alchemist_make_bg_9",
  gearFrameSprite: "box_gear_9",
  createButtonSprite: "btn_green_01_9",
  closeButtonSprite: "btn_red_01_9",
  titleLineSprite: "popup_top_line",
  ratingSprite: "item_grade_01",
  starSprite: "ic_star",
  quantityBarSprite: "product_countbar_ui_0",
  quantityFillSprite: "product_countbar_ui_1",
  quantityHandleSprite: "product_countbar_ui_2",
  grid: { columns: 3, rows: 3 },
  // Current original runtime captures expose four visible tabs. The decoded
  // helmet table is not rendered until a runtime state proves where it belongs.
  tabs: BLACKSMITH_GEAR_TABS,
} as const;

export type GearKind = (typeof ALL_GEAR_KINDS)[number];

export interface BlacksmithRecipeRow {
  id: string;
  kind: GearKind;
  gearId: string;
  rating: number;
  title: string;
  description: string;
  materialIds: number[];
  materialQuantities: number[];
  price: number;
  /** Exact source binding, null until the sprite atlas mapping is proven. */
  iconPath: string | null;
}

export interface GearCatalogRecipe {
  id: string;
  kind: GearKind;
  index: number;
  rating: number;
  job: number;
  difficultyGroup: number;
  itemLevel: number;
  productName: string;
  materialCosts: Array<{ materialId: string; displayName: string; quantity: number; iconPath: string | null }>;
  salePrice: number;
  iconPath: string | null;
}

export interface GearCatalogRow {
  productId: { value?: string | null };
  outputs?: { rows?: Array<{ itemId?: { value?: string | null } }> } | null;
  salePrice?: { value?: number | null } | null;
}

const RECIPE_RE = /^recipe:(weapon|armor|helmet|gloves|boots|ring|necklace|belt):([^:]+):rating:(\d+)$/;

/** Converts normalized product rows to the exact card model used by the route. */
export function adaptBlacksmithRecipes(
  rows: readonly GearCatalogRow[],
  gearData: ReadonlyMap<string, { title: string; description: string; materials: { ids: number[]; quantities: number[] }; price: number }>,
): BlacksmithRecipeRow[] {
  const out: BlacksmithRecipeRow[] = [];
  for (const row of rows) {
    const id = row.productId?.value;
    const match = id?.match(RECIPE_RE);
    if (!id || !match) continue;
    const [, kind, gearIndex, ratingText] = match;
    const gearId = `gear:${kind}:${gearIndex}`;
    const data = gearData.get(gearId);
    if (!data) continue;
    out.push({
      id,
      kind: kind as GearKind,
      gearId,
      rating: Number(ratingText),
      title: data.title,
      description: data.description,
      materialIds: [...data.materials.ids],
      materialQuantities: [...data.materials.quantities],
      price: data.price,
      iconPath: null,
    });
  }
  return out;
}

export function recipesForTab(rows: readonly BlacksmithRecipeRow[], kind: GearKind): BlacksmithRecipeRow[] {
  return rows.filter((row) => row.kind === kind);
}

export function pageGearRecipes(rows: readonly BlacksmithRecipeRow[], page: number, pageSize = 9): BlacksmithRecipeRow[] {
  const start = Math.max(0, page) * pageSize;
  return rows.slice(start, start + pageSize);
}

export function gearSpriteIsResolved(row: BlacksmithRecipeRow): row is BlacksmithRecipeRow & { iconPath: string } {
  return typeof row.iconPath === "string" && row.iconPath.length > 0;
}

/** Projects the full checksum-verified registry into lightweight client rows. */
export function decodeGearCatalog(itemRows: readonly unknown[], productRows: readonly unknown[]): GearCatalogRecipe[] {
  const items = new Map<string, Record<string, unknown>>();
  for (const candidate of itemRows) {
    if (!isRecord(candidate)) continue;
    const id = fieldValue(candidate.itemId);
    if (typeof id === "string") items.set(id, candidate);
  }

  const recipes: GearCatalogRecipe[] = [];
  for (const candidate of productRows) {
    if (!isRecord(candidate)) continue;
    const id = fieldValue(candidate.productId);
    if (typeof id !== "string") continue;
    const match = id.match(/^recipe:(weapon|armor|helmet|gloves|boots|ring|necklace|belt):(\d+):rating:(\d+)$/);
    if (!match) continue;
    const outputRows = collectionRows(candidate.outputs);
    const gearId = outputRows.map((row) => fieldValue(row.itemId)).find((value): value is string => typeof value === "string");
    const gear = gearId ? items.get(gearId) : undefined;
    const localizedName = gear ? fieldValue(gear.displayName) : null;
    const prices = gear && isRecord(gear.directionalEconomy)
      ? fieldValue(gear.directionalEconomy.hunterPaysTownGoldByTier)
      : null;
    const rating = Number(match[3]);
    const materialCosts = collectionRows(candidate.inputs).flatMap((row) => {
      const materialId = fieldValue(row.itemId);
      const quantity = fieldValue(row.quantity);
      if (typeof materialId !== "string" || typeof quantity !== "number") return [];
      const material = items.get(materialId);
      const names = material ? fieldValue(material.displayName) : null;
      return [{
        materialId,
        displayName: localizedText(names, materialId),
        quantity,
        iconPath: null,
      }];
    });
    recipes.push({
      id,
      kind: match[1] as GearKind,
      index: Number(match[2]),
      rating,
      job: -1,
      difficultyGroup: -1,
      itemLevel: 0,
      productName: localizedText(localizedName, gearId ?? id),
      materialCosts,
      salePrice: Array.isArray(prices) && typeof prices[rating] === "number" ? prices[rating] : 0,
      iconPath: null,
    });
  }
  return recipes.sort((left, right) => left.kind.localeCompare(right.kind) || left.rating - right.rating || left.index - right.index);
}

export async function loadGearCatalog(fetchFn: typeof fetch = fetch): Promise<GearCatalogRecipe[]> {
  const response = await fetchFn("/content/releases/evil-hunter-1.411/gear-catalog.json", { cache: "no-cache", credentials: "same-origin" });
  if (!response.ok) throw new Error(`Gear catalog returned ${response.status}`);
  const payload = await response.json() as unknown;
  if (!isRecord(payload) || payload.schemaVersion !== 1 || !Array.isArray(payload.rows) || !isRecord(payload.materials) || !isRecord(payload.materialIcons)) {
    throw new Error("Gear catalog structure is invalid");
  }
  const materialNames = payload.materials;
  const materialIcons = payload.materialIcons;
  const recipes: GearCatalogRecipe[] = [];
  for (const row of payload.rows) {
    if (!isRecord(row) || !ALL_GEAR_KINDS.includes(row.kind as GearKind)
      || !Number.isInteger(row.index) || !Number.isInteger(row.job) || !Number.isInteger(row.group)
      || typeof row.name !== "string" || !Array.isArray(row.prices) || !Array.isArray(row.materialsByRating)) continue;
    for (let rating = 0; rating < 5; rating += 1) {
      const costs = row.materialsByRating[rating];
      if (!Array.isArray(costs)) continue;
      recipes.push({
        id: `recipe:${row.kind}:${row.index}:rating:${rating}`,
        kind: row.kind as GearKind,
        index: row.index as number,
        rating,
        job: row.job as number,
        difficultyGroup: row.group as number,
        itemLevel: typeof row.itemLevel === "number" ? row.itemLevel : 0,
        productName: row.name,
        materialCosts: costs.flatMap((cost) => {
          if (!isRecord(cost) || typeof cost.id !== "string" || typeof cost.quantity !== "number") return [];
          const displayName = materialNames[cost.id];
          const materialIcon = materialIcons[cost.id];
          return [{
            materialId: cost.id,
            displayName: typeof displayName === "string" ? displayName : cost.id,
            quantity: cost.quantity,
            iconPath: typeof materialIcon === "string" ? materialIcon : null,
          }];
        }),
        salePrice: typeof row.prices[rating] === "number" ? row.prices[rating] : 0,
        iconPath: typeof row.iconPath === "string" && row.iconPath.startsWith("/content/releases/evil-hunter-1.411/gear-icons/")
          ? row.iconPath
          : null,
      });
    }
  }
  return recipes.sort((left, right) => left.kind.localeCompare(right.kind)
    || left.difficultyGroup - right.difficultyGroup
    || left.job - right.job
    || left.index - right.index
    || left.rating - right.rating);
}

function collectionRows(value: unknown): Array<Record<string, unknown>> {
  if (!isRecord(value) || !Array.isArray(value.rows)) return [];
  return value.rows.filter(isRecord);
}

function fieldValue(value: unknown): unknown {
  return isRecord(value) ? value.value : undefined;
}

function localizedText(value: unknown, fallback: string): string {
  if (!isRecord(value)) return fallback;
  return typeof value.en === "string" && value.en.length > 0 ? value.en : fallback;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
