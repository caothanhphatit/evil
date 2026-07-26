#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const manifest = JSON.parse(fs.readFileSync(path.join(root, "reverse-engineering/evidence/building-route-manifest-v1.json"), "utf8"));

if (manifest.schemaVersion !== 1 || manifest.manifestType !== "building-route-migration-scope") {
  throw new Error("Unexpected building route manifest schema");
}
if (manifest.buildings.length !== 79 || new Set(manifest.buildings.map((row) => row.buildingId)).size !== 79) {
  throw new Error("Building route manifest must contain each serialized building exactly once");
}

const expectedPopups = new Map([
  ["BuildingPop", [560, 900]],
  ["GearCreatePop", [450, 950]],
  ["ConsumCreatePop", [450, 830]],
  ["ProductCreatePop", [450, 860]],
  ["RequestPop", [480, 820]],
  ["TradeWagonExchangePop", [450, 710]],
]);
for (const [name, [width, height]] of expectedPopups) {
  const popup = manifest.popupTemplates[name];
  if (!popup || popup.dimensions?.x !== width || popup.dimensions?.y !== height || popup.panelSprite !== "popup_bg_9") {
    throw new Error(`Missing or invalid popup template ${name}`);
  }
}

const routes = new Map(manifest.buildings.map((row) => [row.buildingId, row]));
const expectedRoutes = new Map([
  ["build_3", ["trading-post-purchase", "BuildingPop"]],
  ["build_10", ["gear-crafting", "GearCreatePop"]],
  ["build_11", ["potion-display-sale", "BuildingPop"]],
  ["build_14", ["potion-crafting", "ConsumCreatePop"]],
  ["build_9", ["inn-product-service", "BuildingPop", "ProductCreatePop"]],
  ["build_12", ["infirmary-product-service", "BuildingPop", "ProductCreatePop"]],
  ["build_13", ["restaurant-product-service", "BuildingPop", "ProductCreatePop"]],
  ["build_19", ["tavern-product-service", "BuildingPop", "ProductCreatePop"]],
]);
for (const [buildingId, [routeId, ...popupChain]] of expectedRoutes) {
  const route = routes.get(buildingId);
  if (!route || route.routeId !== routeId || JSON.stringify(route.popupChain) !== JSON.stringify(popupChain)) {
    throw new Error(`Invalid priority route ${buildingId}`);
  }
}

if (routes.get("build_10").productCount !== 2755 || routes.get("build_14").productCount !== 40) {
  throw new Error("Recovered crafting product counts changed unexpectedly");
}
for (const buildingId of ["build_9", "build_12", "build_13", "build_19"]) {
  if (routes.get(buildingId).productCount !== 7) throw new Error(`Invalid service product count for ${buildingId}`);
}
for (const screenshots of Object.values(manifest.screenshotCoverage ?? {})) {
  for (const screenshot of screenshots) {
    if (!fs.existsSync(path.join(root, "screenshot", screenshot))) throw new Error(`Missing screenshot ${screenshot}`);
  }
}

console.log(`Validated building route manifest: ${manifest.buildings.length} buildings, ${manifest.priorityRoutes.length} priority route groups.`);
