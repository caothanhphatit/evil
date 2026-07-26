#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const registryPath = path.join(
  root,
  "packages/content/releases/evil-hunter-1.411/building-registry.json",
);
const outputPath = path.join(
  root,
  "infra/db/migrations/0010_normalized_building_gameplay_content.sql",
);
const registry = JSON.parse(fs.readFileSync(registryPath, "utf8"));
const releaseId = registry.registryId;

function resolved(field) {
  return field?.state === "resolved" ? field.value : null;
}

function sql(value) {
  if (value === null || value === undefined) return "NULL";
  if (typeof value === "boolean") return value ? "TRUE" : "FALSE";
  if (typeof value === "number") {
    if (!Number.isSafeInteger(value)) throw new Error(`unsafe integer: ${value}`);
    return String(value);
  }
  return `'${String(value).replaceAll("'", "''")}'`;
}

function insert(table, columns, rows, batchSize = 500) {
  if (rows.length === 0) return "";
  const statements = [];
  for (let offset = 0; offset < rows.length; offset += batchSize) {
    const batch = rows.slice(offset, offset + batchSize);
    statements.push(
      `INSERT INTO ${table} (${columns.join(", ")}) VALUES\n` +
        batch.map((row) => `    (${row.map(sql).join(", ")})`).join(",\n") +
        ";\n",
    );
  }
  return statements.join("\n");
}

const capabilities = registry.catalogs.capabilities.rows.map((row) => [
  releaseId,
  resolved(row.capabilityId),
  resolved(row.buildingId),
  resolved(row.kind),
  row.readiness.staticDataReady,
  row.readiness.runnable && row.readiness.blockingPaths.length === 0,
]);

const resourceKinds = new Map();
const items = [];
const localizations = [];
const itemPrices = [];
const tierPrices = [];
for (const row of registry.catalogs.items.rows) {
  const itemId = resolved(row.itemId);
  resourceKinds.set(itemId, itemId.startsWith("currency:") ? "currency" : "item");
  items.push([
    releaseId,
    itemId,
    resolved(row.internalName),
    resolved(row.itemType),
    resolved(row.stackLimit),
    resolved(row.directionalEconomy?.townPaysHunterGoldPerUnit),
  ]);
  const names = resolved(row.displayName);
  for (const [locale, displayName] of Object.entries(names ?? {}).sort()) {
    localizations.push([releaseId, itemId, locale, displayName]);
  }
  for (const [direction, collection] of [
    ["buy", row.buyPrice],
    ["sell", row.sellPrice],
  ]) {
    for (const [ordinal, amount] of (collection?.rows ?? []).entries()) {
      const componentId = resolved(amount.itemId);
      resourceKinds.set(componentId, componentId.startsWith("currency:") ? "currency" : "reference");
      itemPrices.push([releaseId, itemId, direction, ordinal, componentId, resolved(amount.quantity)]);
    }
  }
  for (const [tier, price] of (
    resolved(row.directionalEconomy?.hunterPaysTownGoldByTier) ?? []
  ).entries()) {
    tierPrices.push([releaseId, itemId, tier + 1, price]);
  }
}

const products = [];
const productAmounts = [];
const services = [];
const completionCounts = [];
const conversions = [];
const randomOutputs = [];
for (const row of registry.catalogs.products.rows) {
  const productId = resolved(row.productId);
  products.push([
    releaseId,
    productId,
    resolved(row.buildingId),
    resolved(row.durationMs),
    false,
  ]);
  for (const [kind, collection] of [
    ["input", row.inputs],
    ["output", row.outputs],
    ["sale_price", row.salePrice],
  ]) {
    for (const [ordinal, amount] of (collection?.rows ?? []).entries()) {
      const resourceId = resolved(amount.itemId);
      if (!resourceKinds.has(resourceId)) {
        resourceKinds.set(resourceId, resourceId.startsWith("currency:") ? "currency" : "reference");
      }
      productAmounts.push([
        releaseId,
        productId,
        kind,
        ordinal,
        resourceId,
        resolved(amount.quantity),
      ]);
    }
  }
  if (row.serviceData) {
    const service = row.serviceData;
    services.push([
      releaseId,
      productId,
      resolved(service.sourceType),
      resolved(service.requiredLevel),
      resolved(service.serviceTimeMs),
      resolved(service.effectValue),
      resolved(service.useMoney),
      resolved(service.requiredCashCount),
      resolved(service.cashCompletionCount),
      resolved(service.requiredElementalCount),
      resolved(service.elementalCompletionCount),
    ]);
    for (const [ordinal, quantity] of (resolved(service.completionCounts) ?? []).entries()) {
      completionCounts.push([releaseId, productId, ordinal, quantity]);
    }
  }
  for (const [ordinal, option] of (row.conversionOptions?.rows ?? []).entries()) {
    const inputId = resolved(option.inputId);
    if (!resourceKinds.has(inputId)) {
      resourceKinds.set(inputId, inputId.startsWith("currency:") ? "currency" : "reference");
    }
    conversions.push([
      releaseId,
      productId,
      ordinal,
      resolved(option.inputKind),
      inputId,
      resolved(option.inputQuantity),
      resolved(option.outputStockQuantity),
    ]);
  }
  if (row.randomOutput) {
    randomOutputs.push([
      releaseId,
      productId,
      resolved(row.randomOutput.itemType),
      resolved(row.randomOutput.grade),
      resolved(row.randomOutput.quantity),
      row.randomOutput.rngBinding?.state === "resolved",
    ]);
  }
}

const resources = [...resourceKinds.entries()]
  .sort(([left], [right]) => left.localeCompare(right))
  .map(([resourceId, resourceKind]) => [releaseId, resourceId, resourceKind]);

const expected = {
  capabilities: 10,
  resources: 1109,
  items: 1107,
  localizations: 15484,
  itemPrices: 0,
  tierPrices: 3395,
  products: 3457,
  productAmounts: 13192,
  services: 52,
  completionCounts: 131,
  conversions: 214,
  randomOutputs: 10,
};
const actual = {
  capabilities: capabilities.length,
  resources: resources.length,
  items: items.length,
  localizations: localizations.length,
  itemPrices: itemPrices.length,
  tierPrices: tierPrices.length,
  products: products.length,
  productAmounts: productAmounts.length,
  services: services.length,
  completionCounts: completionCounts.length,
  conversions: conversions.length,
  randomOutputs: randomOutputs.length,
};
for (const [name, count] of Object.entries(expected)) {
  if (actual[name] !== count) throw new Error(`${name}: expected ${count}, got ${actual[name]}`);
}

const schema = `-- Generated from the pinned building registry. Do not hand-edit content rows.
-- Gameplay reads these relations; the registry JSON is migration input only.
CREATE TABLE building_capability_definition (
    release_id TEXT NOT NULL,
    capability_id TEXT NOT NULL CHECK (btrim(capability_id) <> ''),
    building_id TEXT NOT NULL,
    capability_kind TEXT NOT NULL CHECK (btrim(capability_kind) <> ''),
    static_data_ready BOOLEAN NOT NULL,
    runnable BOOLEAN NOT NULL,
    PRIMARY KEY (release_id, capability_id),
    UNIQUE (release_id, building_id, capability_kind),
    FOREIGN KEY (release_id, building_id)
        REFERENCES building_definition(release_id, building_id) ON DELETE CASCADE
);

CREATE INDEX building_capability_by_building_idx
    ON building_capability_definition (release_id, building_id);

CREATE TABLE economy_resource_definition (
    release_id TEXT NOT NULL REFERENCES content_release(release_id) ON DELETE CASCADE,
    resource_id TEXT NOT NULL CHECK (btrim(resource_id) <> ''),
    resource_kind TEXT NOT NULL CHECK (resource_kind IN ('item', 'currency', 'reference')),
    PRIMARY KEY (release_id, resource_id)
);

CREATE TABLE economy_item_definition (
    release_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    internal_name TEXT,
    item_type TEXT,
    stack_limit BIGINT CHECK (stack_limit IS NULL OR stack_limit >= 0),
    town_pays_hunter_gold_per_unit BIGINT
        CHECK (town_pays_hunter_gold_per_unit IS NULL OR town_pays_hunter_gold_per_unit >= 0),
    PRIMARY KEY (release_id, item_id),
    FOREIGN KEY (release_id, item_id)
        REFERENCES economy_resource_definition(release_id, resource_id) ON DELETE CASCADE
);

CREATE TABLE economy_item_localization (
    release_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    locale TEXT NOT NULL CHECK (btrim(locale) <> ''),
    display_name TEXT NOT NULL,
    PRIMARY KEY (release_id, item_id, locale),
    FOREIGN KEY (release_id, item_id)
        REFERENCES economy_item_definition(release_id, item_id) ON DELETE CASCADE
);

CREATE TABLE economy_item_price_component (
    release_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    price_direction TEXT NOT NULL CHECK (price_direction IN ('buy', 'sell')),
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    resource_id TEXT NOT NULL,
    quantity BIGINT NOT NULL CHECK (quantity > 0),
    PRIMARY KEY (release_id, item_id, price_direction, ordinal),
    FOREIGN KEY (release_id, item_id)
        REFERENCES economy_item_definition(release_id, item_id) ON DELETE CASCADE,
    FOREIGN KEY (release_id, resource_id)
        REFERENCES economy_resource_definition(release_id, resource_id) ON DELETE RESTRICT
);

CREATE TABLE economy_item_hunter_tier_price (
    release_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    tier INTEGER NOT NULL CHECK (tier > 0),
    gold BIGINT NOT NULL CHECK (gold >= 0),
    PRIMARY KEY (release_id, item_id, tier),
    FOREIGN KEY (release_id, item_id)
        REFERENCES economy_item_definition(release_id, item_id) ON DELETE CASCADE
);

CREATE TABLE economy_product_definition (
    release_id TEXT NOT NULL REFERENCES content_release(release_id) ON DELETE CASCADE,
    product_id TEXT NOT NULL CHECK (btrim(product_id) <> ''),
    building_id TEXT,
    duration_ms BIGINT CHECK (duration_ms IS NULL OR duration_ms >= 0),
    exact_mutation_ready BOOLEAN NOT NULL,
    PRIMARY KEY (release_id, product_id),
    FOREIGN KEY (release_id, building_id)
        REFERENCES building_definition(release_id, building_id) ON DELETE RESTRICT
);

CREATE INDEX economy_product_by_building_idx
    ON economy_product_definition (release_id, building_id, product_id);

CREATE TABLE economy_product_amount (
    release_id TEXT NOT NULL,
    product_id TEXT NOT NULL,
    amount_kind TEXT NOT NULL CHECK (amount_kind IN ('input', 'output', 'sale_price')),
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    resource_id TEXT NOT NULL,
    quantity BIGINT NOT NULL CHECK (quantity > 0),
    PRIMARY KEY (release_id, product_id, amount_kind, ordinal),
    FOREIGN KEY (release_id, product_id)
        REFERENCES economy_product_definition(release_id, product_id) ON DELETE CASCADE,
    FOREIGN KEY (release_id, resource_id)
        REFERENCES economy_resource_definition(release_id, resource_id) ON DELETE RESTRICT
);

CREATE TABLE economy_product_service (
    release_id TEXT NOT NULL,
    product_id TEXT NOT NULL,
    source_type BIGINT NOT NULL,
    required_level INTEGER NOT NULL CHECK (required_level >= 0),
    service_time_ms BIGINT NOT NULL CHECK (service_time_ms >= 0),
    effect_value BIGINT NOT NULL,
    use_money BIGINT NOT NULL CHECK (use_money >= 0),
    required_cash_count BIGINT NOT NULL CHECK (required_cash_count >= 0),
    cash_completion_count BIGINT NOT NULL CHECK (cash_completion_count >= 0),
    required_elemental_count BIGINT NOT NULL CHECK (required_elemental_count >= 0),
    elemental_completion_count BIGINT NOT NULL CHECK (elemental_completion_count >= 0),
    PRIMARY KEY (release_id, product_id),
    FOREIGN KEY (release_id, product_id)
        REFERENCES economy_product_definition(release_id, product_id) ON DELETE CASCADE
);

CREATE TABLE economy_product_service_completion (
    release_id TEXT NOT NULL,
    product_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    quantity BIGINT NOT NULL CHECK (quantity >= 0),
    PRIMARY KEY (release_id, product_id, ordinal),
    FOREIGN KEY (release_id, product_id)
        REFERENCES economy_product_service(release_id, product_id) ON DELETE CASCADE
);

CREATE TABLE economy_product_conversion_option (
    release_id TEXT NOT NULL,
    product_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    input_kind TEXT NOT NULL CHECK (btrim(input_kind) <> ''),
    input_resource_id TEXT NOT NULL,
    input_quantity BIGINT NOT NULL CHECK (input_quantity >= 0),
    output_stock_quantity BIGINT NOT NULL CHECK (output_stock_quantity >= 0),
    PRIMARY KEY (release_id, product_id, ordinal),
    FOREIGN KEY (release_id, product_id)
        REFERENCES economy_product_definition(release_id, product_id) ON DELETE CASCADE,
    FOREIGN KEY (release_id, input_resource_id)
        REFERENCES economy_resource_definition(release_id, resource_id) ON DELETE RESTRICT
);

CREATE TABLE economy_product_random_output (
    release_id TEXT NOT NULL,
    product_id TEXT NOT NULL,
    item_type TEXT NOT NULL CHECK (btrim(item_type) <> ''),
    grade BIGINT NOT NULL CHECK (grade >= 0),
    quantity BIGINT NOT NULL CHECK (quantity > 0),
    rng_ready BOOLEAN NOT NULL,
    PRIMARY KEY (release_id, product_id),
    FOREIGN KEY (release_id, product_id)
        REFERENCES economy_product_definition(release_id, product_id) ON DELETE CASCADE
);

`;

const content = [
  schema,
  insert("building_capability_definition", ["release_id", "capability_id", "building_id", "capability_kind", "static_data_ready", "runnable"], capabilities),
  insert("economy_resource_definition", ["release_id", "resource_id", "resource_kind"], resources),
  insert("economy_item_definition", ["release_id", "item_id", "internal_name", "item_type", "stack_limit", "town_pays_hunter_gold_per_unit"], items),
  insert("economy_item_localization", ["release_id", "item_id", "locale", "display_name"], localizations),
  insert("economy_item_price_component", ["release_id", "item_id", "price_direction", "ordinal", "resource_id", "quantity"], itemPrices),
  insert("economy_item_hunter_tier_price", ["release_id", "item_id", "tier", "gold"], tierPrices),
  insert("economy_product_definition", ["release_id", "product_id", "building_id", "duration_ms", "exact_mutation_ready"], products),
  insert("economy_product_amount", ["release_id", "product_id", "amount_kind", "ordinal", "resource_id", "quantity"], productAmounts),
  insert("economy_product_service", ["release_id", "product_id", "source_type", "required_level", "service_time_ms", "effect_value", "use_money", "required_cash_count", "cash_completion_count", "required_elemental_count", "elemental_completion_count"], services),
  insert("economy_product_service_completion", ["release_id", "product_id", "ordinal", "quantity"], completionCounts),
  insert("economy_product_conversion_option", ["release_id", "product_id", "ordinal", "input_kind", "input_resource_id", "input_quantity", "output_stock_quantity"], conversions),
  insert("economy_product_random_output", ["release_id", "product_id", "item_type", "grade", "quantity", "rng_ready"], randomOutputs),
  `DO $$
DECLARE
    target_release CONSTANT TEXT := ${sql(releaseId)};
BEGIN
    IF (SELECT count(*) FROM building_capability_definition WHERE release_id = target_release) <> ${expected.capabilities}
       OR (SELECT count(*) FROM economy_resource_definition WHERE release_id = target_release) <> ${expected.resources}
       OR (SELECT count(*) FROM economy_item_definition WHERE release_id = target_release) <> ${expected.items}
       OR (SELECT count(*) FROM economy_item_localization WHERE release_id = target_release) <> ${expected.localizations}
       OR (SELECT count(*) FROM economy_item_price_component WHERE release_id = target_release) <> ${expected.itemPrices}
       OR (SELECT count(*) FROM economy_item_hunter_tier_price WHERE release_id = target_release) <> ${expected.tierPrices}
       OR (SELECT count(*) FROM economy_product_definition WHERE release_id = target_release) <> ${expected.products}
       OR (SELECT count(*) FROM economy_product_amount WHERE release_id = target_release) <> ${expected.productAmounts}
       OR (SELECT count(*) FROM economy_product_service WHERE release_id = target_release) <> ${expected.services}
       OR (SELECT count(*) FROM economy_product_service_completion WHERE release_id = target_release) <> ${expected.completionCounts}
       OR (SELECT count(*) FROM economy_product_conversion_option WHERE release_id = target_release) <> ${expected.conversions}
       OR (SELECT count(*) FROM economy_product_random_output WHERE release_id = target_release) <> ${expected.randomOutputs}
       OR EXISTS (
           SELECT 1 FROM economy_item_hunter_tier_price
           WHERE release_id = target_release
           GROUP BY item_id
           HAVING min(tier) <> 1 OR max(tier) <> count(*)
       )
       OR EXISTS (
           SELECT 1 FROM economy_product_amount
           WHERE release_id = target_release
           GROUP BY product_id, amount_kind
           HAVING min(ordinal) <> 0 OR max(ordinal) <> count(*) - 1
       )
       OR EXISTS (
           SELECT 1 FROM economy_product_service_completion
           WHERE release_id = target_release
           GROUP BY product_id
           HAVING min(ordinal) <> 0 OR max(ordinal) <> count(*) - 1
       )
       OR EXISTS (
           SELECT 1 FROM economy_product_conversion_option
           WHERE release_id = target_release
           GROUP BY product_id
           HAVING min(ordinal) <> 0 OR max(ordinal) <> count(*) - 1
       )
    THEN
        RAISE EXCEPTION 'normalized building gameplay content count mismatch';
    END IF;
END $$;
`,
].join("\n");

fs.writeFileSync(outputPath, content);
console.log(JSON.stringify({ outputPath, ...actual }, null, 2));
