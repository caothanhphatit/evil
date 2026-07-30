import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const releaseId = "evil-hunter-1.411.buildings-v1";
const monsterPath = resolve(root, "packages/content/releases/evil-hunter-1.411/ordinary-hunting-monster-map.json");
const gearPath = resolve(root, "packages/content/releases/evil-hunter-1.411/gear-catalog.json");
const economyPath = resolve(root, "reverse-engineering/evidence/core-economy-tables-v1.json");
const experiencePath = resolve(root, "packages/content/releases/evil-hunter-1.411/experience-runtime-catalog.json");
const outputPath = resolve(root, "infra/db/migrations/0033_runtime_object_catalog.sql");
const downPath = resolve(root, "infra/db/migrations/0033_runtime_object_catalog.down.sql");

const [monsterBytes, gearBytes, economyBytes, experienceBytes] = await Promise.all([
  readFile(monsterPath), readFile(gearPath), readFile(economyPath), readFile(experiencePath),
]);
const monsterCatalog = JSON.parse(monsterBytes);
const gearCatalog = JSON.parse(gearBytes);
const economyCatalog = JSON.parse(economyBytes);
const mapByArea = new Map([[0, "map_new01"], [1, "background_08"], [2, "background_11"]]);
const quote = (value) => `'${String(value).replaceAll("'", "''")}'`;
const nullable = (value) => value == null ? "NULL" : quote(value);
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");

if (!Array.isArray(monsterCatalog.regions) || !Array.isArray(gearCatalog.rows)) {
  throw new Error("Unexpected runtime content catalog shape");
}

const monsters = [];
const pools = [];
const drops = [];
for (const region of monsterCatalog.regions) {
  const mapId = mapByArea.get(region.area);
  if (!mapId) throw new Error(`Missing map binding for monster area ${region.area}`);
  for (const difficulty of region.difficulties) {
    difficulty.monsterPool.forEach((monster, poolOrdinal) => {
      if (monster.materials.indices.length !== monster.materials.counts.length
          || monster.materials.percentValues.length < monster.materials.indices.length) {
        throw new Error(`Invalid material arrays for monster ${monster.sourceIndex}`);
      }
      const unresolved = monster.materials.percentValues.length === monster.materials.indices.length
        ? "{}"
        : JSON.stringify({ materialArrayLengths: monster.materials.arrayLengths, extraPercentValues: monster.materials.percentValues.slice(monster.materials.indices.length) });
      monsters.push(`    (${quote(releaseId)}, ${monster.sourceIndex}, ${monster.type}, ${monster.uniqueLevel}, ${monster.race}, ${monster.hp}, ${monster.damage}, ${monster.armor}, ${monster.experience}, ${monster.gold}, 'mon_a_01_1', ${quote(unresolved)}::jsonb)`);
      pools.push(`    (${quote(releaseId)}, ${quote(mapId)}, ${difficulty.globalDifficulty}, ${poolOrdinal}, ${monster.sourceIndex})`);
      monster.materials.indices.forEach((sourceIndex, slot) => {
        drops.push(`    (${quote(releaseId)}, ${monster.sourceIndex}, ${slot}, ${sourceIndex}, ${monster.materials.counts[slot]}, ${monster.materials.percentValues[slot]})`);
      });
    });
  }
}

const gearDefinitions = [];
const gearRatings = [];
const gearMaterials = [];
const gearBindings = [];
for (const gear of gearCatalog.rows) {
  if (gear.prices.length !== 5 || gear.materialsByRating.length !== 5) {
    throw new Error(`Invalid rating arrays for ${gear.kind}:${gear.index}`);
  }
  gearDefinitions.push(`    (${quote(releaseId)}, ${quote(gear.kind)}, ${gear.index}, ${gear.job}, ${gear.group}, ${gear.itemLevel}, ${gear.visible}, ${gear.sortGroup}, ${quote(gear.name)}, ${quote(gear.description)}, ${nullable(gear.iconPath)})`);
  gear.prices.forEach((price, rating) => {
    gearRatings.push(`    (${quote(releaseId)}, ${quote(gear.kind)}, ${gear.index}, ${rating}, ${price})`);
    gearBindings.push(`    (${quote(releaseId)}, ${quote(`recipe:${gear.kind}:${gear.index}:rating:${rating}`)}, ${quote(gear.kind)}, ${gear.index}, ${rating})`);
    gear.materialsByRating[rating].forEach((material, ordinal) => {
      gearMaterials.push(`    (${quote(releaseId)}, ${quote(gear.kind)}, ${gear.index}, ${rating}, ${ordinal}, ${quote(material.id)}, ${material.quantity})`);
    });
  });
}

const materials = economyCatalog.materials.map((material) =>
  `    (${quote(releaseId)}, ${quote(`material:${material.index}`)}, ${material.index}, ${material.rating}, ${material.level}, ${material.convert}, ${material.compose}, ${material.parentIndex}, ${material.magic})`,
);
const consumables = [];
const consumableLevels = [];
const consumableBindings = [];
for (const consumable of economyCatalog.consumables) {
  consumables.push(`    (${quote(releaseId)}, ${consumable.index}, ${consumable.type}, ${consumable.maxLevel}, ${consumable.coolTime * 1000})`);
  consumable.keepValueByLevel.forEach((keepValue, level) => {
    consumableLevels.push(`    (${quote(releaseId)}, ${consumable.index}, ${level}, ${consumable.keepTimeByLevel[level] * 1000}, ${keepValue}, ${consumable.priceByLevel[level]})`);
    consumableBindings.push(`    (${quote(releaseId)}, ${quote(`recipe:consumable:${consumable.index}:level:${level}`)}, ${consumable.index}, ${level})`);
  });
}

const values = (rows) => rows.join(",\n");
const sql = `-- Generated by tools/generate-runtime-content-migration.mjs. Do not hand-edit rows.
CREATE TABLE content_source_manifest (
    release_id TEXT NOT NULL REFERENCES content_release(release_id) ON DELETE CASCADE,
    source_id TEXT NOT NULL CHECK (btrim(source_id) <> ''),
    source_path TEXT NOT NULL CHECK (btrim(source_path) <> ''),
    source_sha256 BYTEA NOT NULL CHECK (octet_length(source_sha256) = 32),
    PRIMARY KEY (release_id, source_id)
);

CREATE TABLE monster_definition (
    release_id TEXT NOT NULL REFERENCES content_release(release_id) ON DELETE CASCADE,
    source_index INTEGER NOT NULL CHECK (source_index >= 0),
    monster_type INTEGER NOT NULL CHECK (monster_type >= 0),
    unique_level INTEGER NOT NULL CHECK (unique_level >= 0),
    race INTEGER NOT NULL CHECK (race >= 0),
    hp BIGINT NOT NULL CHECK (hp > 0),
    damage BIGINT NOT NULL CHECK (damage >= 0),
    armor BIGINT NOT NULL CHECK (armor >= 0),
    experience BIGINT NOT NULL CHECK (experience >= 0),
    gold BIGINT NOT NULL CHECK (gold >= 0),
    asset_bundle_id TEXT NOT NULL CHECK (btrim(asset_bundle_id) <> ''),
    unresolved_evidence JSONB NOT NULL DEFAULT '{}'::jsonb,
    PRIMARY KEY (release_id, source_index)
);

CREATE TABLE material_definition (
    release_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    source_index INTEGER NOT NULL CHECK (source_index >= 0),
    difficulty_rating INTEGER NOT NULL CHECK (difficulty_rating BETWEEN 0 AND 5),
    material_level INTEGER NOT NULL CHECK (material_level >= 0),
    convert_value INTEGER NOT NULL,
    compose_value INTEGER NOT NULL,
    parent_source_index INTEGER NOT NULL CHECK (parent_source_index >= 0),
    magic INTEGER NOT NULL,
    PRIMARY KEY (release_id, item_id),
    UNIQUE (release_id, source_index),
    FOREIGN KEY (release_id, item_id)
        REFERENCES economy_item_definition(release_id, item_id) ON DELETE CASCADE
);

CREATE TABLE monster_material_drop_definition (
    release_id TEXT NOT NULL,
    monster_source_index INTEGER NOT NULL,
    slot INTEGER NOT NULL CHECK (slot >= 0),
    material_source_index INTEGER NOT NULL CHECK (material_source_index >= 0),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    raw_percent INTEGER NOT NULL CHECK (raw_percent >= 0),
    PRIMARY KEY (release_id, monster_source_index, slot),
    FOREIGN KEY (release_id, monster_source_index)
        REFERENCES monster_definition(release_id, source_index) ON DELETE CASCADE
);

CREATE TABLE ordinary_monster_pool_definition (
    release_id TEXT NOT NULL,
    map_id TEXT NOT NULL,
    global_difficulty INTEGER NOT NULL CHECK (global_difficulty BETWEEN 0 AND 4),
    pool_ordinal INTEGER NOT NULL CHECK (pool_ordinal >= 0),
    monster_source_index INTEGER NOT NULL,
    PRIMARY KEY (release_id, map_id, global_difficulty, pool_ordinal),
    UNIQUE (release_id, map_id, global_difficulty, monster_source_index),
    FOREIGN KEY (release_id, map_id)
        REFERENCES world_map_definition(release_id, map_id) ON DELETE CASCADE,
    FOREIGN KEY (release_id, monster_source_index)
        REFERENCES monster_definition(release_id, source_index) ON DELETE RESTRICT
);

CREATE TABLE gear_definition (
    release_id TEXT NOT NULL REFERENCES content_release(release_id) ON DELETE CASCADE,
    gear_kind TEXT NOT NULL CHECK (gear_kind IN ('weapon', 'armor', 'helmet', 'gloves', 'boots', 'ring', 'necklace', 'belt')),
    gear_index INTEGER NOT NULL CHECK (gear_index >= 0),
    job INTEGER NOT NULL,
    difficulty_group INTEGER NOT NULL CHECK (difficulty_group >= 0),
    item_level INTEGER NOT NULL CHECK (item_level >= 0),
    visibility INTEGER NOT NULL,
    sort_group INTEGER NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL,
    icon_path TEXT,
    PRIMARY KEY (release_id, gear_kind, gear_index)
);

CREATE TABLE gear_rating_definition (
    release_id TEXT NOT NULL,
    gear_kind TEXT NOT NULL,
    gear_index INTEGER NOT NULL,
    rating INTEGER NOT NULL CHECK (rating BETWEEN 0 AND 4),
    price BIGINT NOT NULL CHECK (price >= 0),
    PRIMARY KEY (release_id, gear_kind, gear_index, rating),
    FOREIGN KEY (release_id, gear_kind, gear_index)
        REFERENCES gear_definition(release_id, gear_kind, gear_index) ON DELETE CASCADE
);

CREATE TABLE gear_material_requirement (
    release_id TEXT NOT NULL,
    gear_kind TEXT NOT NULL,
    gear_index INTEGER NOT NULL,
    rating INTEGER NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    material_id TEXT NOT NULL,
    quantity BIGINT NOT NULL CHECK (quantity > 0),
    PRIMARY KEY (release_id, gear_kind, gear_index, rating, ordinal),
    FOREIGN KEY (release_id, gear_kind, gear_index, rating)
        REFERENCES gear_rating_definition(release_id, gear_kind, gear_index, rating) ON DELETE CASCADE,
    FOREIGN KEY (release_id, material_id)
        REFERENCES economy_item_definition(release_id, item_id) ON DELETE RESTRICT
);

CREATE TABLE economy_product_gear_binding (
    release_id TEXT NOT NULL,
    product_id TEXT NOT NULL,
    gear_kind TEXT NOT NULL,
    gear_index INTEGER NOT NULL,
    rating INTEGER NOT NULL,
    PRIMARY KEY (release_id, product_id),
    FOREIGN KEY (release_id, product_id)
        REFERENCES economy_product_definition(release_id, product_id) ON DELETE CASCADE,
    FOREIGN KEY (release_id, gear_kind, gear_index, rating)
        REFERENCES gear_rating_definition(release_id, gear_kind, gear_index, rating) ON DELETE RESTRICT
);

CREATE TABLE consumable_definition (
    release_id TEXT NOT NULL REFERENCES content_release(release_id) ON DELETE CASCADE,
    consumable_index INTEGER NOT NULL CHECK (consumable_index >= 0),
    consumable_type INTEGER NOT NULL,
    max_level INTEGER NOT NULL CHECK (max_level > 0),
    cooldown_ms BIGINT NOT NULL CHECK (cooldown_ms >= 0),
    PRIMARY KEY (release_id, consumable_index)
);

CREATE TABLE consumable_level_definition (
    release_id TEXT NOT NULL,
    consumable_index INTEGER NOT NULL,
    level INTEGER NOT NULL CHECK (level >= 0),
    keep_time_ms BIGINT NOT NULL CHECK (keep_time_ms >= 0),
    keep_value BIGINT NOT NULL,
    price BIGINT NOT NULL CHECK (price >= 0),
    PRIMARY KEY (release_id, consumable_index, level),
    FOREIGN KEY (release_id, consumable_index)
        REFERENCES consumable_definition(release_id, consumable_index) ON DELETE CASCADE
);

CREATE TABLE economy_product_consumable_binding (
    release_id TEXT NOT NULL,
    product_id TEXT NOT NULL,
    consumable_index INTEGER NOT NULL,
    level INTEGER NOT NULL,
    PRIMARY KEY (release_id, product_id),
    FOREIGN KEY (release_id, product_id)
        REFERENCES economy_product_definition(release_id, product_id) ON DELETE CASCADE,
    FOREIGN KEY (release_id, consumable_index, level)
        REFERENCES consumable_level_definition(release_id, consumable_index, level) ON DELETE RESTRICT
);

INSERT INTO content_source_manifest (release_id, source_id, source_path, source_sha256) VALUES
    (${quote(releaseId)}, 'ordinary-monster-map', 'packages/content/releases/evil-hunter-1.411/ordinary-hunting-monster-map.json', decode('${hash(monsterBytes)}', 'hex')),
    (${quote(releaseId)}, 'gear-catalog', 'packages/content/releases/evil-hunter-1.411/gear-catalog.json', decode('${hash(gearBytes)}', 'hex')),
    (${quote(releaseId)}, 'core-economy', 'reverse-engineering/evidence/core-economy-tables-v1.json', decode('${hash(economyBytes)}', 'hex')),
    (${quote(releaseId)}, 'experience-catalog', 'packages/content/releases/evil-hunter-1.411/experience-runtime-catalog.json', decode('${hash(experienceBytes)}', 'hex'));

INSERT INTO material_definition (release_id, item_id, source_index, difficulty_rating, material_level, convert_value, compose_value, parent_source_index, magic) VALUES
${values(materials)};

UPDATE hunter_skill_definition
SET icon_path = CASE skill_id
    WHEN 'skill_h1_01' THEN 'sprites/skill_h1_01__1395.png'
    WHEN 'skill_h1_02' THEN 'sprites/skill_h1_02__5620.png'
END
WHERE release_id = 'migration.hunter-demo-v1'
  AND skill_id IN ('skill_h1_01', 'skill_h1_02');

INSERT INTO monster_definition (release_id, source_index, monster_type, unique_level, race, hp, damage, armor, experience, gold, asset_bundle_id, unresolved_evidence) VALUES
${values(monsters)};
INSERT INTO monster_material_drop_definition (release_id, monster_source_index, slot, material_source_index, quantity, raw_percent) VALUES
${values(drops)};
INSERT INTO ordinary_monster_pool_definition (release_id, map_id, global_difficulty, pool_ordinal, monster_source_index) VALUES
${values(pools)};
INSERT INTO gear_definition (release_id, gear_kind, gear_index, job, difficulty_group, item_level, visibility, sort_group, display_name, description, icon_path) VALUES
${values(gearDefinitions)};
INSERT INTO gear_rating_definition (release_id, gear_kind, gear_index, rating, price) VALUES
${values(gearRatings)};
INSERT INTO gear_material_requirement (release_id, gear_kind, gear_index, rating, ordinal, material_id, quantity) VALUES
${values(gearMaterials)};
INSERT INTO economy_product_gear_binding (release_id, product_id, gear_kind, gear_index, rating) VALUES
${values(gearBindings)};
INSERT INTO consumable_definition (release_id, consumable_index, consumable_type, max_level, cooldown_ms) VALUES
${values(consumables)};
INSERT INTO consumable_level_definition (release_id, consumable_index, level, keep_time_ms, keep_value, price) VALUES
${values(consumableLevels)};
INSERT INTO economy_product_consumable_binding (release_id, product_id, consumable_index, level) VALUES
${values(consumableBindings)};
`;

await writeFile(outputPath, sql);
await writeFile(downPath, `DROP TABLE IF EXISTS economy_product_consumable_binding;
DROP TABLE IF EXISTS consumable_level_definition;
DROP TABLE IF EXISTS consumable_definition;
DROP TABLE IF EXISTS economy_product_gear_binding;
DROP TABLE IF EXISTS gear_material_requirement;
DROP TABLE IF EXISTS gear_rating_definition;
DROP TABLE IF EXISTS gear_definition;
DROP TABLE IF EXISTS ordinary_monster_pool_definition;
DROP TABLE IF EXISTS monster_material_drop_definition;
DROP TABLE IF EXISTS monster_definition;
DROP TABLE IF EXISTS material_definition;
DROP TABLE IF EXISTS content_source_manifest;
`);
