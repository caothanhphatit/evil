#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const info = JSON.parse(fs.readFileSync(path.join(root, "reverse-engineering/evidence/hunter-info-tables-v1.json"), "utf8"));
const generation = JSON.parse(fs.readFileSync(path.join(root, "reverse-engineering/evidence/hunter-generation-tables-v1.json"), "utf8"));
const release = "evil-hunter-1.411.hunter-info-v1";

const q = (value) => value === null || value === undefined ? "NULL" : `'${String(value).replaceAll("'", "''")}'`;
const json = (value) => `${q(JSON.stringify(value))}::jsonb`;
const number = (value) => Number.isFinite(value) ? String(value) : "NULL";
const localized = (row, field, locale = "en") => row.localized?.[locale]?.[field] ?? null;

const lines = [];
lines.push(`-- Generated from reverse-engineering/evidence/hunter-info-tables-v1.json.`);
lines.push(`-- Player-owned values remain nullable until recovered or explicitly written by gameplay.`);
lines.push(`INSERT INTO hunter_content_release(release_id, game_version, status, evidence_note)`);
lines.push(`VALUES (${q(release)}, '1.411', 'confirmed', 'Exact QuickSheet Hunter info definitions; player ownership and allocations are separate state.')`);
lines.push(`ON CONFLICT (release_id) DO NOTHING;`, ``);
lines.push(`INSERT INTO hunter_class_definition`);
lines.push(`    (release_id, class_id, display_name, source_job_index, visual_family, evidence_confidence, semantics_status)`);
lines.push(`SELECT ${q(release)}, class_id, CASE WHEN source_job_index = 4 THEN 'DarkKnight' ELSE display_name END,`);
lines.push(`       source_job_index, visual_family, 'confirmed', 'resolved'`);
lines.push(`FROM hunter_class_definition WHERE release_id = 'migration.hunter-demo-v1'`);
lines.push(`ON CONFLICT (release_id, class_id) DO NOTHING;`, ``);
lines.push(`UPDATE hunter_class_definition SET display_name = 'DarkKnight'`);
lines.push(`WHERE release_id = 'migration.hunter-demo-v1' AND source_job_index = 4;`, ``);
lines.push(`ALTER TABLE hunter_skill_definition`);
lines.push(`    ADD COLUMN source_kind TEXT CHECK (source_kind IN ('basic', 'class_change')),`);
lines.push(`    ADD COLUMN source_index INTEGER,`);
lines.push(`    ADD COLUMN description TEXT,`);
lines.push(`    ADD COLUMN detail_description TEXT,`);
lines.push(`    ADD COLUMN max_level SMALLINT CHECK (max_level IS NULL OR max_level > 0),`);
lines.push(`    ADD COLUMN sub_job SMALLINT,`);
lines.push(`    ADD COLUMN third_job SMALLINT,`);
lines.push(`    ADD COLUMN fourth_job SMALLINT,`);
lines.push(`    ADD COLUMN source_parameters JSONB;`, ``);
lines.push(`CREATE UNIQUE INDEX hunter_skill_source_unique`);
lines.push(`    ON hunter_skill_definition(release_id, source_kind, source_index)`);
lines.push(`    WHERE source_kind IS NOT NULL AND source_index IS NOT NULL;`, ``);

const skills = [
  ...info.skills.map((row) => ({ ...row, sourceKind: "basic", subJob: null, thirdJob: null, fourthJob: null })),
  ...info.subJobSkills.map((row) => ({ ...row, sourceKind: "class_change" })),
];
lines.push(`INSERT INTO hunter_skill_definition`);
lines.push(`    (release_id, skill_id, class_id, display_name, icon_path, animation_name, evidence_confidence, semantics_status,`);
lines.push(`     source_kind, source_index, description, detail_description, max_level, sub_job, third_job, fourth_job, source_parameters)`);
lines.push(`VALUES`);
lines.push(skills.map((row) => {
  const payload = { ...row };
  delete payload.localized;
  delete payload.sourceKind;
  return `    (${q(release)}, ${q(`${row.sourceKind}:${row.index}`)}, ${q(`h${row.job + 1}`)}, ${q(localized(row, "name"))}, NULL, NULL, 'confirmed', 'resolved', ${q(row.sourceKind)}, ${row.index}, ${q(localized(row, "description"))}, ${q(localized(row, "detailDescription"))}, ${row.maxLevel}, ${number(row.subJob)}, ${number(row.thirdJob)}, ${number(row.fourthJob)}, ${json(payload)})`;
}).join(",\n") + `\nON CONFLICT (release_id, skill_id) DO NOTHING;`);
lines.push(``);

lines.push(`CREATE TABLE hunter_characteristic_definition (`);
lines.push(`    release_id TEXT NOT NULL REFERENCES hunter_content_release(release_id) ON DELETE CASCADE,`);
lines.push(`    characteristic_id TEXT NOT NULL,`);
lines.push(`    source_index SMALLINT NOT NULL CHECK (source_index >= 0),`);
lines.push(`    display_name TEXT NOT NULL,`);
lines.push(`    description TEXT,`);
lines.push(`    effect_value DOUBLE PRECISION NOT NULL,`);
lines.push(`    PRIMARY KEY (release_id, characteristic_id),`);
lines.push(`    UNIQUE (release_id, source_index)`);
lines.push(`);`, ``);
lines.push(`INSERT INTO hunter_characteristic_definition`);
lines.push(`    (release_id, characteristic_id, source_index, display_name, description, effect_value)`);
lines.push(`VALUES`);
lines.push(generation.characteristics.map((row) =>
  `    (${q(release)}, ${q(`characteristic:${row.index}`)}, ${row.index}, ${q(localized(row, "name"))}, ${q(localized(row, "description"))}, ${number(row.keepValue)})`
).join(",\n") + `;`);
lines.push(``);

lines.push(`CREATE TABLE hunter_growth_property_definition (`);
lines.push(`    release_id TEXT NOT NULL REFERENCES hunter_content_release(release_id) ON DELETE CASCADE,`);
lines.push(`    property_id TEXT NOT NULL,`);
lines.push(`    source_index SMALLINT NOT NULL CHECK (source_index BETWEEN 0 AND 14),`);
lines.push(`    display_name TEXT NOT NULL,`);
lines.push(`    description TEXT NOT NULL,`);
lines.push(`    value_per_rank DOUBLE PRECISION NOT NULL,`);
lines.push(`    icon_path TEXT NOT NULL,`);
lines.push(`    icon_binding_confidence TEXT NOT NULL CHECK (icon_binding_confidence IN ('confirmed', 'strongly_inferred')),`);
lines.push(`    PRIMARY KEY (release_id, property_id),`);
lines.push(`    UNIQUE (release_id, source_index)`);
lines.push(`);`, ``);
lines.push(`INSERT INTO hunter_growth_property_definition`);
lines.push(`    (release_id, property_id, source_index, display_name, description, value_per_rank, icon_path, icon_binding_confidence)`);
lines.push(`VALUES`);
lines.push(info.growthProperties.map((row) => {
  const icon = `/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-growth/growth_ic_${String(row.index).padStart(2, "0")}${growthSuffix(row.index)}`;
  return `    (${q(release)}, ${q(`growth:${row.index}`)}, ${row.index}, ${q(localized(row, "name"))}, ${q(localized(row, "description"))}, ${number(row.upValue)}, ${q(icon)}, 'strongly_inferred')`;
}).join(",\n") + `;`);
lines.push(``);

lines.push(`CREATE TABLE hunter_riding_pet_definition (`);
lines.push(`    release_id TEXT NOT NULL REFERENCES hunter_content_release(release_id) ON DELETE CASCADE,`);
lines.push(`    riding_pet_id TEXT NOT NULL, source_index SMALLINT NOT NULL, grade SMALLINT NOT NULL,`);
lines.push(`    background_type SMALLINT NOT NULL, display_name TEXT NOT NULL, content TEXT, icon_path TEXT,`);
lines.push(`    PRIMARY KEY (release_id, riding_pet_id), UNIQUE (release_id, source_index)`);
lines.push(`);`, ``);
lines.push(`INSERT INTO hunter_riding_pet_definition`);
lines.push(`    (release_id, riding_pet_id, source_index, grade, background_type, display_name, content, icon_path)`);
lines.push(`VALUES`);
lines.push(info.ridingPets.map((row) =>
  `    (${q(release)}, ${q(`riding-pet:${row.index}`)}, ${row.index}, ${row.grade}, ${row.backgroundType}, ${q(localized(row, "title"))}, ${q(localized(row, "content"))}, NULL)`
).join(",\n") + `;`);
lines.push(``);

lines.push(`ALTER TABLE player_hunter`);
lines.push(`    ADD COLUMN characteristic_release_id TEXT,`);
lines.push(`    ADD COLUMN characteristic_id TEXT,`);
lines.push(`    ADD COLUMN xp_to_next_level BIGINT CHECK (xp_to_next_level IS NULL OR xp_to_next_level >= 0),`);
lines.push(`    ADD COLUMN dps_milli BIGINT CHECK (dps_milli IS NULL OR dps_milli >= 0),`);
lines.push(`    ADD COLUMN critical_rate_bps INTEGER CHECK (critical_rate_bps IS NULL OR critical_rate_bps >= 0),`);
lines.push(`    ADD COLUMN attack_speed_milli INTEGER CHECK (attack_speed_milli IS NULL OR attack_speed_milli >= 0),`);
lines.push(`    ADD COLUMN evasion_rate_bps INTEGER CHECK (evasion_rate_bps IS NULL OR evasion_rate_bps >= 0),`);
lines.push(`    ADD COLUMN awakening_current INTEGER CHECK (awakening_current IS NULL OR awakening_current >= 0),`);
lines.push(`    ADD COLUMN awakening_maximum INTEGER CHECK (awakening_maximum IS NULL OR awakening_maximum >= 0),`);
lines.push(`    ADD COLUMN reincarnation_current INTEGER CHECK (reincarnation_current IS NULL OR reincarnation_current >= 0),`);
lines.push(`    ADD COLUMN reincarnation_maximum INTEGER CHECK (reincarnation_maximum IS NULL OR reincarnation_maximum >= 0),`);
lines.push(`    ADD COLUMN is_locked BOOLEAN,`);
lines.push(`    ADD COLUMN secret_points INTEGER CHECK (secret_points IS NULL OR secret_points >= 0),`);
lines.push(`    ADD COLUMN riding_pet_state_resolved BOOLEAN NOT NULL DEFAULT FALSE,`);
lines.push(`    ADD CONSTRAINT player_hunter_characteristic_fk FOREIGN KEY (characteristic_release_id, characteristic_id)`);
lines.push(`        REFERENCES hunter_characteristic_definition(release_id, characteristic_id),`);
lines.push(`    ADD CONSTRAINT player_hunter_awaken_pair CHECK ((awakening_current IS NULL) = (awakening_maximum IS NULL)),`);
lines.push(`    ADD CONSTRAINT player_hunter_reincarnation_pair CHECK ((reincarnation_current IS NULL) = (reincarnation_maximum IS NULL));`, ``);

lines.push(`CREATE TABLE player_hunter_growth (`);
lines.push(`    player_token UUID NOT NULL, hunter_id BIGINT NOT NULL, release_id TEXT NOT NULL, property_id TEXT NOT NULL,`);
lines.push(`    allocated_points INTEGER NOT NULL CHECK (allocated_points >= 0),`);
lines.push(`    PRIMARY KEY (player_token, hunter_id, property_id),`);
lines.push(`    FOREIGN KEY (player_token, hunter_id) REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE,`);
lines.push(`    FOREIGN KEY (release_id, property_id) REFERENCES hunter_growth_property_definition(release_id, property_id)`);
lines.push(`);`, ``);
lines.push(`CREATE TABLE player_hunter_material_stack (`);
lines.push(`    player_token UUID NOT NULL, hunter_id BIGINT NOT NULL, economy_release_id TEXT NOT NULL, item_id TEXT NOT NULL,`);
lines.push(`    quantity BIGINT NOT NULL CHECK (quantity > 0), reserved_quantity BIGINT NOT NULL DEFAULT 0 CHECK (reserved_quantity >= 0 AND reserved_quantity <= quantity),`);
lines.push(`    PRIMARY KEY (player_token, hunter_id, economy_release_id, item_id),`);
lines.push(`    FOREIGN KEY (player_token, hunter_id) REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE,`);
lines.push(`    FOREIGN KEY (economy_release_id, item_id) REFERENCES economy_item_definition(release_id, item_id)`);
lines.push(`);`, ``);
lines.push(`CREATE TABLE player_riding_pet (`);
lines.push(`    player_token UUID NOT NULL, riding_pet_instance_id UUID NOT NULL DEFAULT gen_random_uuid(), release_id TEXT NOT NULL, riding_pet_id TEXT NOT NULL,`);
lines.push(`    level INTEGER, grade INTEGER, PRIMARY KEY (player_token, riding_pet_instance_id),`);
lines.push(`    FOREIGN KEY (release_id, riding_pet_id) REFERENCES hunter_riding_pet_definition(release_id, riding_pet_id)`);
lines.push(`);`, ``);
lines.push(`CREATE TABLE player_hunter_riding_pet (`);
lines.push(`    player_token UUID NOT NULL, hunter_id BIGINT NOT NULL, riding_pet_instance_id UUID NOT NULL,`);
lines.push(`    PRIMARY KEY (player_token, hunter_id), UNIQUE (player_token, riding_pet_instance_id),`);
lines.push(`    FOREIGN KEY (player_token, hunter_id) REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE,`);
lines.push(`    FOREIGN KEY (player_token, riding_pet_instance_id) REFERENCES player_riding_pet(player_token, riding_pet_instance_id) ON DELETE CASCADE`);
lines.push(`);`, ``);
lines.push(`COMMENT ON COLUMN player_hunter.dps_milli IS 'Nullable authoritative snapshot; no original DPS formula has been recovered.';`);
lines.push(`COMMENT ON TABLE player_hunter_material_stack IS 'Per-Hunter carried material state; never projected from town stock.';`);

fs.writeFileSync(path.join(root, "infra/db/migrations/0016_hunter_info_domain.sql"), lines.join("\n") + "\n");

function growthSuffix(index) {
  const dir = path.join(root, "apps/web/public/content/releases/evil-hunter-1.411/hunter-assets/ui/hunter-info-growth");
  const prefix = `growth_ic_${String(index).padStart(2, "0")}__`;
  const file = fs.readdirSync(dir).find((name) => name.startsWith(prefix));
  if (!file) throw new Error(`missing packaged growth icon ${prefix}`);
  return file.slice(`growth_ic_${String(index).padStart(2, "0")}`.length);
}
