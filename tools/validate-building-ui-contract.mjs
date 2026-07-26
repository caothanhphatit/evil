#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const contractPath = path.join(root, "reverse-engineering/evidence/building-ui-contract-v1.json");
const contract = JSON.parse(fs.readFileSync(contractPath, "utf8"));

if (contract.schemaVersion !== 1 || contract.contractType !== "building-ui-evidence") {
  throw new Error("Unexpected building UI contract schema");
}
if (contract.runtimeCompatibility !== "not-claimed") {
  throw new Error("Evidence contract must not claim runtime compatibility");
}
if (contract.displayNames?.status !== "unresolved" || contract.displayNames.bindings.length !== 0) {
  throw new Error("Display names must remain unresolved until localization bindings are decoded");
}
if (contract.placement?.confirmed?.length !== 1 || contract.placement.confirmed[0].sceneGameObjectPathId !== 260) {
  throw new Error("Only the serialized ReviveBuilding anchor may be confirmed");
}

const expectedPopups = new Map([
  ["BuildingPop", { root: 932, panel: 1531, width: 560, height: 900 }],
  ["RequestPop", { root: 2504, panel: 815, width: 480, height: 820 }],
  ["GearCreatePop", { root: 1272, panel: 429, width: 450, height: 950 }],
  ["ConsumCreatePop", { root: 1264, panel: 2132, width: 450, height: 830 }],
  ["ProductCreatePop", { root: 1359, panel: 631, width: 450, height: 860 }],
  ["TradeWagonExchangePop", { root: 1896, panel: 5179, width: 450, height: 710 }],
]);
if (!Array.isArray(contract.popupTemplates) || contract.popupTemplates.length !== expectedPopups.size) {
  throw new Error("Popup template contract is incomplete");
}
for (const popup of contract.popupTemplates) {
  const expected = expectedPopups.get(popup.name);
  if (!expected || popup.rootGameObjectPathId !== expected.root || popup.panelGameObjectPathId !== expected.panel
    || popup.rootControllerClass !== popup.name || popup.panelDimensions.x !== expected.width || popup.panelDimensions.y !== expected.height) {
    throw new Error(`Popup template identity or dimensions are invalid for ${popup.name}`);
  }
  if (popup.panelSprite?.name !== "popup_bg_9" || popup.panelSprite.status !== "resolved-inventory"
    || popup.hierarchy[0]?.pathId !== expected.root || popup.labels.length === 0 || popup.spriteBindings.length === 0) {
    throw new Error(`Popup template evidence is incomplete for ${popup.name}`);
  }
  if (popup.buildingIdBinding?.status !== "unresolved" || popup.buildingIdBinding.value !== null) {
    throw new Error(`Popup template overclaims a building dispatch binding for ${popup.name}`);
  }
  if (typeof popup.semanticRole?.value !== "string" || popup.semanticRole.confidence !== "strongly-inferred") {
    throw new Error(`Popup semantic role confidence is invalid for ${popup.name}`);
  }
}
const popupRows = new Map(contract.popupLocalization?.rows?.map((row) => [row.key, row]));
for (const key of ["buildpop_0", "buildpop_25", "buildpop_26", "buildpop_30", "requestpop_0", "gearcreatepop_3", "consumcreatepop_0", "productcreatepop_0"]) {
  const row = popupRows.get(key);
  if (!row || typeof row.localized.en !== "string" || typeof row.localized.vi !== "string") {
    throw new Error(`Missing decoded popup localization ${key}`);
  }
}

const clips = new Map(contract.animationAssets.map((item) => [item.clipPathId, item]));
for (const controller of contract.controllers) {
  for (const clipPathId of controller.clipPathIds) {
    if (!clips.has(clipPathId)) throw new Error(`Controller ${controller.name} references missing clip ${clipPathId}`);
  }
}
for (const animation of contract.animationAssets) {
  if (!animation.spriteSequence.length) throw new Error(`Animation ${animation.name} has no sprite sequence`);
  if (animation.spriteSequence.some((item) => item.status !== "resolved")) {
    throw new Error(`Animation ${animation.name} contains an unresolved sprite reference`);
  }
  if (animation.displayName.status !== "unresolved" || animation.townPosition.status !== "unresolved") {
    throw new Error(`Animation ${animation.name} overclaims semantic or placement evidence`);
  }
}

console.log(`Validated building UI contract: ${contract.animationAssets.length} animations, ${contract.controllers.length} controllers, ${contract.sceneObjects.length} scene objects, ${contract.prefabObjects.length} prefab objects.`);
