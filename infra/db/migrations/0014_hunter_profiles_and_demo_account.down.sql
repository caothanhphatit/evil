DELETE FROM local_identities WHERE player_token = '00000000-0000-4000-8000-00000000a001';
DELETE FROM player_world_state WHERE player_token = '00000000-0000-4000-8000-00000000a001';

DROP TABLE IF EXISTS player_hunter_skill;
DROP TABLE IF EXISTS player_hunter_trait;

ALTER TABLE player_hunter
    DROP CONSTRAINT IF EXISTS player_hunter_rarity_fk,
    DROP CONSTRAINT IF EXISTS player_hunter_class_fk,
    DROP CONSTRAINT IF EXISTS player_hunter_instance_unique,
    DROP COLUMN IF EXISTS seed_ordinal,
    DROP COLUMN IF EXISTS state_revision,
    DROP COLUMN IF EXISTS animation_name,
    DROP COLUMN IF EXISTS action_state,
    DROP COLUMN IF EXISTS defense,
    DROP COLUMN IF EXISTS attack,
    DROP COLUMN IF EXISTS xp,
    DROP COLUMN IF EXISTS level,
    DROP COLUMN IF EXISTS rarity_id,
    DROP COLUMN IF EXISTS class_id,
    DROP COLUMN IF EXISTS portrait_asset_id,
    DROP COLUMN IF EXISTS display_name,
    DROP COLUMN IF EXISTS content_release_id,
    DROP COLUMN IF EXISTS hunter_instance_id;

DROP TABLE IF EXISTS player_profile;
DROP TABLE IF EXISTS hunter_skill_definition;
DROP TABLE IF EXISTS hunter_trait_definition;
DROP TABLE IF EXISTS hunter_rarity_definition;
DROP TABLE IF EXISTS hunter_class_definition;
DROP TABLE IF EXISTS hunter_content_release;
