DROP TABLE IF EXISTS player_hunter_riding_pet;
DROP TABLE IF EXISTS player_riding_pet;
DROP TABLE IF EXISTS player_hunter_material_stack;
DROP TABLE IF EXISTS player_hunter_growth;

ALTER TABLE player_hunter
    DROP CONSTRAINT IF EXISTS player_hunter_reincarnation_pair,
    DROP CONSTRAINT IF EXISTS player_hunter_awaken_pair,
    DROP CONSTRAINT IF EXISTS player_hunter_characteristic_fk,
    DROP COLUMN IF EXISTS riding_pet_state_resolved,
    DROP COLUMN IF EXISTS secret_points,
    DROP COLUMN IF EXISTS is_locked,
    DROP COLUMN IF EXISTS reincarnation_maximum,
    DROP COLUMN IF EXISTS reincarnation_current,
    DROP COLUMN IF EXISTS awakening_maximum,
    DROP COLUMN IF EXISTS awakening_current,
    DROP COLUMN IF EXISTS evasion_rate_bps,
    DROP COLUMN IF EXISTS attack_speed_milli,
    DROP COLUMN IF EXISTS critical_rate_bps,
    DROP COLUMN IF EXISTS dps_milli,
    DROP COLUMN IF EXISTS xp_to_next_level,
    DROP COLUMN IF EXISTS characteristic_id,
    DROP COLUMN IF EXISTS characteristic_release_id;

DROP TABLE IF EXISTS hunter_riding_pet_definition;
DROP TABLE IF EXISTS hunter_growth_property_definition;
DROP TABLE IF EXISTS hunter_characteristic_definition;

DROP INDEX IF EXISTS hunter_skill_source_unique;
ALTER TABLE hunter_skill_definition
    DROP COLUMN IF EXISTS source_parameters,
    DROP COLUMN IF EXISTS fourth_job,
    DROP COLUMN IF EXISTS third_job,
    DROP COLUMN IF EXISTS sub_job,
    DROP COLUMN IF EXISTS max_level,
    DROP COLUMN IF EXISTS detail_description,
    DROP COLUMN IF EXISTS description,
    DROP COLUMN IF EXISTS source_index,
    DROP COLUMN IF EXISTS source_kind;

DELETE FROM hunter_content_release WHERE release_id = 'evil-hunter-1.411.hunter-info-v1';
