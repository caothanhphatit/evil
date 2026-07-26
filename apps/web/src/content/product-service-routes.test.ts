import { describe, expect, it } from "vitest";
import { PRODUCT_SERVICE_ROUTES, productServiceRoute, productServiceSprite, projectProductService } from "./product-service-routes";

describe("ProductCreatePop service routes", () => {
  it("keeps the recovered seven-product order per building", () => {
    expect(PRODUCT_SERVICE_ROUTES.build_9.productIds).toEqual(["product:0", "product:1", "product:2", "product:3", "product:4", "product:29", "product:48"]);
    expect(PRODUCT_SERVICE_ROUTES.build_12.productIds).toEqual(["product:5", "product:6", "product:7", "product:8", "product:9", "product:30", "product:49"]);
    expect(PRODUCT_SERVICE_ROUTES.build_13.productIds).toEqual(["product:10", "product:11", "product:12", "product:13", "product:14", "product:31", "product:50"]);
    expect(PRODUCT_SERVICE_ROUTES.build_19.productIds).toEqual(["product:15", "product:16", "product:17", "product:18", "product:19", "product:32", "product:51"]);
  });

  it("does not leak products from another building into a route", () => {
    const result = projectProductService("build_13", [
      { productId: "product:10", productName: "Meal", requiredLevel: 0, effectValue: 140, serviceTimeMs: 10000, useMoney: 90, stock: 2, capacity: 3, materialCosts: [] },
      { productId: "product:0", productName: "Inn rest", requiredLevel: 0, effectValue: 140, serviceTimeMs: 10000, useMoney: 90, stock: 1, capacity: 3, materialCosts: [] },
    ], 3);
    expect(result?.products.map((product) => product.productId)).toEqual(["product:10"]);
  });

  it("returns null for non-service buildings and resolves extracted product sprites", () => {
    expect(productServiceRoute("build_10")).toBeNull();
    expect(productServiceSprite("product:0")).toContain("product_00__3523.png");
    expect(productServiceSprite("product:5")).toContain("product_05__2957.png");
  });
});
