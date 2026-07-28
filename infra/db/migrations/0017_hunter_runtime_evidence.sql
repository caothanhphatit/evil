-- Additive runtime-schema storage. Values remain nullable until a controlled
-- capture or authoritative rebuild command supplies them.
ALTER TABLE player_hunter
    ADD COLUMN source_dictionary_key TEXT,
    ADD COLUMN source_index INTEGER,
    ADD COLUMN source_job INTEGER,
    ADD COLUMN source_sub_job INTEGER,
    ADD COLUMN source_third_job INTEGER,
    ADD COLUMN source_fourth_job INTEGER,
    ADD COLUMN source_personality INTEGER,
    ADD COLUMN source_grade_rank_up INTEGER,
    ADD COLUMN source_dark_soul BIGINT,
    ADD COLUMN source_used_dark_soul BIGINT,
    ADD COLUMN source_used_job_trait BIGINT,
    ADD COLUMN source_hp BIGINT,
    ADD COLUMN source_now_hp BIGINT,
    ADD COLUMN source_feel REAL,
    ADD COLUMN source_now_feel REAL,
    ADD COLUMN source_hungry REAL,
    ADD COLUMN source_now_hungry REAL,
    ADD COLUMN source_tire REAL,
    ADD COLUMN source_now_tire REAL,
    ADD COLUMN source_damage BIGINT,
    ADD COLUMN source_armor BIGINT,
    ADD COLUMN source_critical INTEGER,
    ADD COLUMN source_attack_speed REAL,
    ADD COLUMN source_dodge INTEGER,
    ADD CONSTRAINT player_hunter_source_dictionary_unique
        UNIQUE (player_token, roster_state, source_dictionary_key);

CREATE TABLE player_hunter_runtime_section (
    player_token UUID NOT NULL,
    hunter_id BIGINT NOT NULL,
    section TEXT NOT NULL CHECK (section IN ('status', 'skills', 'inventory', 'growth', 'riding_pet')),
    value_captured BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (player_token, hunter_id, section),
    FOREIGN KEY (player_token, hunter_id)
        REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE
);

CREATE TABLE player_hunter_runtime_appearance (
    player_token UUID NOT NULL,
    hunter_id BIGINT NOT NULL,
    body_index INTEGER NOT NULL,
    costume_index INTEGER NOT NULL,
    costume_hidden BOOLEAN NOT NULL,
    fairy_index INTEGER NOT NULL,
    fairy_hidden BOOLEAN NOT NULL,
    weapon_costume_index INTEGER NOT NULL,
    weapon_costume_hidden BOOLEAN NOT NULL,
    wing_costume_index INTEGER NOT NULL,
    wing_costume_hidden BOOLEAN NOT NULL,
    seal_costume_index INTEGER NOT NULL,
    seal_costume_hidden BOOLEAN NOT NULL,
    ramble_pet_index INTEGER NOT NULL,
    ramble_pet_hidden BOOLEAN NOT NULL,
    hat_hidden BOOLEAN NOT NULL,
    costume_hat_hidden BOOLEAN NOT NULL,
    PRIMARY KEY (player_token, hunter_id),
    FOREIGN KEY (player_token, hunter_id)
        REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE
);

CREATE TABLE player_hunter_runtime_skill (
    player_token UUID NOT NULL,
    hunter_id BIGINT NOT NULL,
    dictionary_key TEXT NOT NULL,
    source_index INTEGER NOT NULL,
    skill_index INTEGER NOT NULL,
    cool_time DOUBLE PRECISION NOT NULL,
    skill_level INTEGER NOT NULL,
    PRIMARY KEY (player_token, hunter_id, dictionary_key),
    FOREIGN KEY (player_token, hunter_id)
        REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE
);

CREATE TABLE player_hunter_runtime_item (
    player_token UUID NOT NULL,
    hunter_id BIGINT NOT NULL,
    dictionary_key TEXT NOT NULL,
    new_check BOOLEAN NOT NULL,
    source_index INTEGER NOT NULL,
    item_count BIGINT NOT NULL,
    reservation BIGINT NOT NULL,
    infinity_check BOOLEAN NOT NULL,
    PRIMARY KEY (player_token, hunter_id, dictionary_key),
    FOREIGN KEY (player_token, hunter_id)
        REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE
);

CREATE TABLE player_hunter_runtime_gear (
    player_token UUID NOT NULL,
    hunter_id BIGINT NOT NULL,
    dictionary_key TEXT NOT NULL,
    source_index INTEGER NOT NULL,
    gear_index INTEGER NOT NULL,
    inventory_index INTEGER NOT NULL,
    quality INTEGER NOT NULL,
    new_check BOOLEAN NOT NULL,
    gear_level INTEGER NOT NULL,
    rating INTEGER NOT NULL,
    gear_group INTEGER NOT NULL,
    plus_type INTEGER[] NOT NULL,
    plus_value INTEGER[] NOT NULL,
    minus_type INTEGER[] NOT NULL,
    minus_value INTEGER[] NOT NULL,
    additional_plus_type INTEGER[] NOT NULL,
    additional_plus_value INTEGER[] NOT NULL,
    additional_minus_type INTEGER[] NOT NULL,
    additional_minus_value INTEGER[] NOT NULL,
    buy_gold INTEGER NOT NULL,
    buy_date TEXT NOT NULL,
    buy_date_value BIGINT NOT NULL,
    quality_count INTEGER NOT NULL,
    option_count INTEGER NOT NULL,
    lock_count INTEGER NOT NULL,
    potential INTEGER NOT NULL,
    runes_index INTEGER NOT NULL,
    runes_value INTEGER NOT NULL,
    skill_runes_index INTEGER NOT NULL,
    skill_runes_value INTEGER NOT NULL,
    delete_count INTEGER NOT NULL,
    unidentified_option_count INTEGER NOT NULL,
    PRIMARY KEY (player_token, hunter_id, dictionary_key),
    FOREIGN KEY (player_token, hunter_id)
        REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE
);

CREATE TABLE player_hunter_runtime_consumable (
    player_token UUID NOT NULL,
    hunter_id BIGINT NOT NULL,
    dictionary_key TEXT NOT NULL,
    total_count INTEGER NOT NULL,
    nested_values_resolved BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (player_token, hunter_id, dictionary_key),
    FOREIGN KEY (player_token, hunter_id)
        REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE,
    CHECK (NOT nested_values_resolved)
);

CREATE TABLE player_hunter_runtime_growth (
    player_token UUID NOT NULL,
    hunter_id BIGINT NOT NULL,
    source_order SMALLINT NOT NULL CHECK (source_order >= 0),
    property_level INTEGER NOT NULL,
    PRIMARY KEY (player_token, hunter_id, source_order),
    FOREIGN KEY (player_token, hunter_id)
        REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE
);

CREATE TABLE player_hunter_runtime_riding_pet (
    player_token UUID NOT NULL,
    hunter_id BIGINT NOT NULL,
    pasture_index INTEGER NOT NULL,
    source_index INTEGER NOT NULL,
    master_index TEXT NOT NULL,
    rating INTEGER NOT NULL,
    skill_index INTEGER NOT NULL,
    trait_index INTEGER NOT NULL,
    trait_level INTEGER NOT NULL,
    use_soul INTEGER NOT NULL,
    use_growth_stone INTEGER NOT NULL,
    locked BOOLEAN NOT NULL,
    pet_gear_values_resolved BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (player_token, hunter_id),
    FOREIGN KEY (player_token, hunter_id)
        REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE,
    CHECK (NOT pet_gear_values_resolved)
);

COMMENT ON COLUMN player_hunter.source_dictionary_key IS
    'Opaque HunterDataDic key; semantic format remains unresolved.';
COMMENT ON TABLE player_hunter_runtime_appearance IS
    'Raw HunterData indices only; player_hunter_visual_component remains the rebuild render projection.';
COMMENT ON TABLE player_hunter_runtime_consumable IS
    'ConsumTotalData scalar state only; nested ConsumData schema remains unresolved.';
COMMENT ON TABLE player_hunter_runtime_riding_pet IS
    'Exact RidingPetData scalar fields; pet gear values remain unresolved.';
