import { t } from "../i18n";

/** Exact ProductCreatePop contracts recovered for the four service buildings. */
export type ProductServiceBuildingId = "build_9" | "build_12" | "build_13" | "build_19";

export interface ProductServiceRouteSpec {
  buildingId: ProductServiceBuildingId;
  routeId: `${string}-product-service`;
  title: string;
  effectLabel: string;
  effectKind: "Stamina" | "HP" | "satiety" | "mood";
  hunterEmptyLabel: string;
  productIds: readonly string[];
  popup: "ProductCreatePop";
  capacitySource: "building-product-stock";
}

export const PRODUCT_SERVICE_ROUTES: Record<ProductServiceBuildingId, ProductServiceRouteSpec> = {
  build_9: { buildingId: "build_9", routeId: "inn-product-service", title: t("service.inn.title"), effectLabel: t("service.inn.effect"), effectKind: "Stamina", hunterEmptyLabel: t("service.inn.empty"), productIds: ["product:0", "product:1", "product:2", "product:3", "product:4", "product:29", "product:48"], popup: "ProductCreatePop", capacitySource: "building-product-stock" },
  build_12: { buildingId: "build_12", routeId: "infirmary-product-service", title: t("service.infirmary.title"), effectLabel: t("service.infirmary.effect"), effectKind: "HP", hunterEmptyLabel: t("service.infirmary.empty"), productIds: ["product:5", "product:6", "product:7", "product:8", "product:9", "product:30", "product:49"], popup: "ProductCreatePop", capacitySource: "building-product-stock" },
  build_13: { buildingId: "build_13", routeId: "restaurant-product-service", title: t("service.restaurant.title"), effectLabel: t("service.restaurant.effect"), effectKind: "satiety", hunterEmptyLabel: t("service.restaurant.empty"), productIds: ["product:10", "product:11", "product:12", "product:13", "product:14", "product:31", "product:50"], popup: "ProductCreatePop", capacitySource: "building-product-stock" },
  build_19: { buildingId: "build_19", routeId: "tavern-product-service", title: t("service.tavern.title"), effectLabel: t("service.tavern.effect"), effectKind: "mood", hunterEmptyLabel: t("service.tavern.empty"), productIds: ["product:15", "product:16", "product:17", "product:18", "product:19", "product:32", "product:51"], popup: "ProductCreatePop", capacitySource: "building-product-stock" },
};

/** Product sprites are the extracted product atlas, never building/shop art. */
export const PRODUCT_SERVICE_SPRITES: Record<string, string> = Object.fromEntries(
  [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 29, 30, 31, 32, 48, 49, 50, 51].map((id) => [
    `product:${id}`, `/content/releases/original-flow-v1/sprites/product_${String(id).padStart(2, "0")}__${({ 0: "3523", 1: "4988", 2: "4912", 3: "2634", 4: "7168", 5: "2957", 6: "3994", 7: "2037", 8: "6490", 9: "1935", 10: "6271", 11: "2026", 12: "1368", 13: "3637", 14: "1604", 15: "6488", 16: "3707", 17: "6592", 18: "6216", 19: "5193", 29: "6396", 30: "3026", 31: "1771", 32: "7065", 48: "4411", 49: "4142", 50: "4905", 51: "6664" } as Record<number, string>)[id]}.png`,
  ]),
);

export interface ProductServiceInput {
  productId: string;
  productName: string;
  requiredLevel: number;
  effectValue: number;
  serviceTimeMs: number;
  useMoney: number;
  stock: number;
  capacity: number;
  materialCosts: readonly ProductMaterialCost[];
}

export interface ProductMaterialCost { materialId: string; displayName: string; quantity: number; outputQuantity: number; iconPath?: string; }

export interface ProductServiceViewModel {
  spec: ProductServiceRouteSpec;
  products: readonly ProductServiceInput[];
  capacity: { stock: number; maximum: number };
}

export function productServiceRoute(buildingId: string): ProductServiceRouteSpec | null {
  return PRODUCT_SERVICE_ROUTES[buildingId as ProductServiceBuildingId] ?? null;
}

export function projectProductService(buildingId: string, products: readonly ProductServiceInput[], capacity: number): ProductServiceViewModel | null {
  const spec = productServiceRoute(buildingId);
  if (!spec) return null;
  const expected = new Set(spec.productIds);
  const scoped = products.filter((product) => expected.has(product.productId));
  // Fail closed: a route must not display another building's products.
  if (scoped.some((product) => !expected.has(product.productId))) return null;
  return { spec, products: scoped, capacity: { stock: scoped.reduce((sum, product) => sum + product.stock, 0), maximum: Math.max(0, capacity) } };
}

export function productServiceSprite(productId: string): string | null {
  return PRODUCT_SERVICE_SPRITES[productId] ?? null;
}
