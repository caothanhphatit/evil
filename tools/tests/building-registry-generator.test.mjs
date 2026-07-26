import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";
import { validateBuildingRegistry } from "../validate-building-registry.mjs";

const run = promisify(execFile);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const generator = path.join(root, "tools/generate-building-registry.py");
const committed = path.join(root, "packages/content/releases/evil-hunter-1.411/building-registry.json");

async function readJson(file) {
  return JSON.parse(await fs.readFile(file, "utf8"));
}

test("building registry generator is deterministic and matches the committed release", async () => {
  const temporaryDirectory = await fs.mkdtemp(path.join(os.tmpdir(), "evil-building-registry-"));
  const first = path.join(temporaryDirectory, "first.json");
  const second = path.join(temporaryDirectory, "second.json");
  try {
    await run("python3", [generator, "--output", first], { cwd: root });
    await run("python3", [generator, "--output", second], { cwd: root });
    const [firstBytes, secondBytes, committedBytes] = await Promise.all([
      fs.readFile(first),
      fs.readFile(second),
      fs.readFile(committed),
    ]);
    assert.deepEqual(firstBytes, secondBytes);
    assert.deepEqual(firstBytes, committedBytes);
    validateBuildingRegistry(JSON.parse(firstBytes.toString("utf8")));
  } finally {
    await fs.rm(temporaryDirectory, { recursive: true, force: true });
  }
});

test("building registry preserves exact sentinel identities and level costs", async () => {
  const registry = await readJson(committed);
  const buildings = new Map(registry.buildings.rows.map((building) => [building.buildId.value, building]));
  const expected = new Map([
    ["build_1", ["Town Hall", 500]],
    ["build_3", ["Trading Post", 300]],
    ["build_7", ["Weapon Shop", 660]],
    ["build_10", ["Blacksmith", 660]],
    ["build_11", ["Potion Shop", 2400]],
    ["build_14", ["Alchemist's Home", 2400]],
    ["build_20", ["Accessory Shop", 2400]],
    ["build_21", ["Jeweler", 2400]],
  ]);

  for (const [buildId, [name, gold]] of expected) {
    const building = buildings.get(buildId);
    assert.ok(building, `missing ${buildId}`);
    assert.equal(building.displayName.value.en, name);
    assert.equal(building.levels.rows[0].upgradeCosts.rows[0].itemId.value, "currency:gold");
    assert.equal(building.levels.rows[0].upgradeCosts.rows[0].quantity.value, gold);
    assert.equal(building.levels.rows[0].conditions.rows[0].kind.value, "possibleBuild");
    assert.equal(building.levels.rows[0].conditions.rows[0].subjectId.value, "build_1.level");
    assert.equal(building.levels.rows[0].conditions.rows[0].operator.value, "greater-than-or-equal");
    assert.equal(building.levels.rows[0].conditions.rows[0].subjectId.confidence, "strongly-inferred");
  }

  const weaponShop = buildings.get("build_7");
  assert.equal(weaponShop.sourceData.maxBuild.value, 1);
  assert.deepEqual(weaponShop.sourceData.gridSize.value, [2, 2]);
  assert.equal(weaponShop.sourceData.movable.value, 0);
  assert.equal(weaponShop.sourceData.visibility.value, 0);
  assert.deepEqual(weaponShop.sourceData.firstValues.value, [0, 1, 2, 3, 4]);
});

test("building registry contains every recovered building and product without becoming runnable", async () => {
  const registry = validateBuildingRegistry(await readJson(committed));
  assert.equal(registry.buildings.rows.length, 79);
  assert.equal(registry.catalogs.items.rows.length, 1107);
  assert.equal(registry.catalogs.products.rows.length, 3457);
  assert.equal(registry.catalogs.capabilities.rows.length, 10);
  assert.equal(registry.catalogs.skins.rows.length, 61);
  assert.equal(registry.runtimeState, "blocked");
  assert.equal(registry.releaseGate.runnable, false);
  assert.ok(registry.releaseGate.blockingPaths.includes("buildings.rows[1].visualBinding.popupClass"));
});

test("building registry migrates serialized skin rows without cataloging orphan assets", async () => {
  const registry = validateBuildingRegistry(await readJson(committed));
  const skins = new Map(registry.catalogs.skins.rows.map((skin) => [skin.key, skin]));
  assert.equal(skins.size, 61);
  assert.equal([...skins.values()].filter((skin) => skin.visualBinding.binding.state === "resolved").length, 47);
  assert.equal([...skins.values()].filter((skin) => skin.visualBinding.binding.state === "unresolved").length, 14);
  assert.equal(skins.has("build_3:skin_29"), false);

  const townHall = skins.get("build_1:skin_1");
  assert.equal(townHall.displayName.value.en, "Middle Ages Town Hall");
  assert.equal(townHall.requiredLevel.value, 4);
  assert.deepEqual(townHall.costs.rows.map((cost) => [cost.itemId.value, cost.quantity.value]), [
    ["currency:gold", 1_000_000],
    ["material:267", 50],
    ["material:269", 50],
    ["material:271", 50],
    ["material:277", 3],
  ]);
  assert.equal(townHall.visualBinding.assetKey.value, "buildSkin_1_0");
  assert.equal(townHall.visualBinding.spritePrefix.value, "bd_a_cos_001_");
  assert.equal(townHall.visualBinding.spriteFrames.value.length, 5);

  const dungeon = skins.get("build_16:skin_1");
  assert.equal(dungeon.visualBinding.binding.state, "unresolved");
  assert.equal(dungeon.visualBinding.assetKey.value, null);
});

test("building registry exposes evidence-backed core capability data without claiming runtime readiness", async () => {
  const registry = validateBuildingRegistry(await readJson(committed));
  const capabilities = new Map(
    registry.catalogs.capabilities.rows.map((capability) => [capability.buildingId.value, capability])
  );
  const expected = new Map([
    ["build_2", ["automatic-revival", "popup-template:BuildingReviveCheckPop"]],
    ["build_3", ["loot-purchase-reservations", null]],
    ["build_4", ["bounty-quest-list", "popup-template:QuestPop"]],
    ["build_7", ["weapon-display-and-sale", "popup-template:BuildingPop"]],
    ["build_8", ["armor-display-and-sale", "popup-template:BuildingPop"]],
    ["build_10", ["weapon-and-armor-crafting", "popup-template:GearCreatePop"]],
    ["build_11", ["potion-display-and-sale", "popup-template:BuildingPop"]],
    ["build_14", ["potion-crafting", "popup-template:ConsumCreatePop"]],
    ["build_20", ["accessory-display-and-sale", "popup-template:BuildingPop"]],
    ["build_21", ["accessory-crafting", "popup-template:GearCreatePop"]],
  ]);

  for (const [buildingId, [kind, popupTemplateId]] of expected) {
    const capability = capabilities.get(buildingId);
    assert.ok(capability, `missing capability for ${buildingId}`);
    assert.equal(capability.kind.value, kind);
    assert.equal(capability.popupTemplateId.value, popupTemplateId);
    assert.equal(capability.parameters.value.description.en.length > 0, true);
    assert.equal(capability.readiness.staticDataReady, buildingId !== "build_3");
    assert.equal(capability.readiness.runnable, false);
    assert.deepEqual(
      capability.readiness.blockingPaths,
      buildingId === "build_3"
        ? ["conditions.binding", "popupBinding", "popupTemplateId", "runtimeBinding"]
        : ["conditions.binding", "popupBinding", "runtimeBinding"]
    );
    assert.equal(capability.popupBinding.state, "unresolved");
    assert.equal(capability.runtimeBinding.state, "unresolved");
  }

  const buildings = new Map(registry.buildings.rows.map((building) => [building.buildId.value, building]));
  assert.equal(
    buildings.get("build_10").capabilityIds.rows[0].id.value,
    "capability:weapon-and-armor-crafting"
  );
  assert.equal(
    buildings.get("build_10").levels.rows[0].capabilityIds.rows[0].id.value,
    "capability:weapon-and-armor-crafting"
  );
});

test("building registry validator rejects a capability readiness projection that overclaims runnable state", async () => {
  const registry = await readJson(committed);
  registry.catalogs.capabilities.rows[0].readiness.runnable = true;
  assert.throws(
    () => validateBuildingRegistry(registry),
    /readiness\.runnable disagrees with local unresolved bindings/
  );
});

test("building registry migrates exact economy item and recipe sentinels", async () => {
  const registry = await readJson(committed);
  const items = new Map(registry.catalogs.items.rows.map((item) => [item.itemId.value, item]));
  const products = new Map(registry.catalogs.products.rows.map((product) => [product.productId.value, product]));

  const room = products.get("product:0");
  assert.equal(room.durationMs.value, 10_000);
  assert.equal(room.serviceData.sourceType.value, 0);
  assert.equal(room.serviceData.requiredLevel.value, 0);
  assert.equal(room.serviceData.serviceTimeMs.value, 10_000);
  assert.equal(room.serviceData.effectValue.value, 140);
  assert.equal(room.serviceData.useMoney.value, 90);
  assert.deepEqual(room.serviceData.completionCounts.value, [1, 2, 10]);
  assert.equal(room.serviceData.requiredCashCount.value, 3);
  assert.equal(room.serviceData.cashCompletionCount.value, 1);
  assert.equal(room.serviceData.requiredElementalCount.value, 150);
  assert.equal(room.serviceData.elementalCompletionCount.value, 1);
  assert.equal(room.inputs, null);
  assert.equal(room.outputs, null);
  assert.equal(room.salePrice, null);
  assert.equal(room.conditions.binding.state, "unresolved");
  assert.deepEqual(
    room.conversionOptions.rows.map((option) => [
      option.inputKind.value,
      option.inputId.value,
      option.inputQuantity.value,
      option.outputStockQuantity.value,
    ]),
    [
      ["material", "material:32", 1, 1],
      ["material", "material:92", 1, 2],
      ["material", "material:16", 1, 10],
      ["gem", "currency:gem", 3, 1],
      ["elemental", "currency:elemental", 150, 1],
    ]
  );

  assert.equal(items.get("material:11").displayName.value.en, "Heartwood Fragment");
  assert.equal(items.get("gear:weapon:0").displayName.value.en, "Junk Sword");
  assert.equal(items.get("consumable:0").displayName.value.en, "Healing Potion");
  assert.equal(items.get("rune:1").displayName.value.en, "Mood Consumption Rune");
  assert.equal(items.get("material:32").directionalEconomy.townPaysHunterGoldPerUnit.value, 10);
  assert.equal(items.get("material:32").directionalEconomy.hunterPaysTownGoldByTier, null);
  assert.deepEqual(items.get("gear:weapon:0").directionalEconomy.hunterPaysTownGoldByTier.value, [200, 300, 400, 500, 600]);
  assert.deepEqual(items.get("consumable:0").directionalEconomy.hunterPaysTownGoldByTier.value, [68, 203, 608, 1823, 5468, 24605, 118098, 247500]);

  const weapon = products.get("recipe:weapon:0:rating:0");
  assert.equal(weapon.buildingId.value, "build_10");
  assert.deepEqual(weapon.inputs.rows.map((row) => [row.itemId.value, row.quantity.value]), [
    ["material:1", 10],
    ["material:1", 10],
    ["material:1", 10],
  ]);
  assert.equal(weapon.outputs.rows[0].itemId.value, "gear:weapon:0");

  const potion = products.get("recipe:consumable:0:level:0");
  assert.equal(potion.buildingId.value, "build_14");
  assert.deepEqual(potion.inputs.rows.map((row) => [row.itemId.value, row.quantity.value]), [["material:139", 3]]);
  assert.equal(potion.outputs.rows[0].itemId.value, "consumable:0");

  const randomRune = products.get("recipe:rune-random:0");
  assert.equal(randomRune.buildingId.value, null);
  assert.deepEqual(randomRune.inputs.rows.map((row) => [row.itemId.value, row.quantity.value]), [["material:189", 5]]);
  assert.equal(randomRune.outputs, null);
  assert.equal(randomRune.salePrice, null);
  assert.equal(randomRune.randomOutput.itemType.value, "rune");
  assert.equal(randomRune.randomOutput.grade.value, 0);
  assert.equal(randomRune.randomOutput.quantity.value, 1);
  assert.equal(randomRune.randomOutput.rngBinding.state, "unresolved");
});
