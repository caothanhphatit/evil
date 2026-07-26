CREATE TABLE hunter_content_release (
    release_id TEXT PRIMARY KEY,
    game_version TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('fixture', 'inferred', 'confirmed')),
    evidence_note TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE hunter_class_definition (
    release_id TEXT NOT NULL REFERENCES hunter_content_release(release_id) ON DELETE CASCADE,
    class_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    source_job_index SMALLINT NOT NULL CHECK (source_job_index BETWEEN 0 AND 4),
    visual_family TEXT NOT NULL CHECK (visual_family IN ('H1', 'H2', 'H3', 'H4', 'H5')),
    evidence_confidence TEXT NOT NULL CHECK (evidence_confidence IN ('confirmed', 'strongly_inferred', 'tentative', 'unknown')),
    semantics_status TEXT NOT NULL CHECK (semantics_status IN ('resolved', 'visual_only', 'unresolved')),
    PRIMARY KEY (release_id, class_id),
    UNIQUE (release_id, source_job_index),
    UNIQUE (release_id, visual_family)
);

CREATE TABLE hunter_rarity_definition (
    release_id TEXT NOT NULL REFERENCES hunter_content_release(release_id) ON DELETE CASCADE,
    rarity_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    rank SMALLINT NOT NULL CHECK (rank BETWEEN 0 AND 4),
    evidence_confidence TEXT NOT NULL CHECK (evidence_confidence IN ('confirmed', 'strongly_inferred', 'tentative', 'unknown')),
    PRIMARY KEY (release_id, rarity_id),
    UNIQUE (release_id, rank)
);

CREATE TABLE hunter_trait_definition (
    release_id TEXT NOT NULL REFERENCES hunter_content_release(release_id) ON DELETE CASCADE,
    trait_id TEXT NOT NULL,
    class_id TEXT,
    branch SMALLINT NOT NULL CHECK (branch BETWEEN 0 AND 4),
    tier SMALLINT NOT NULL CHECK (tier BETWEEN 1 AND 4),
    display_name TEXT NOT NULL,
    icon_path TEXT NOT NULL,
    evidence_confidence TEXT NOT NULL CHECK (evidence_confidence IN ('confirmed', 'strongly_inferred', 'tentative', 'unknown')),
    semantics_status TEXT NOT NULL CHECK (semantics_status IN ('resolved', 'visual_only', 'unresolved')),
    PRIMARY KEY (release_id, trait_id),
    FOREIGN KEY (release_id, class_id) REFERENCES hunter_class_definition(release_id, class_id)
);

CREATE TABLE hunter_skill_definition (
    release_id TEXT NOT NULL REFERENCES hunter_content_release(release_id) ON DELETE CASCADE,
    skill_id TEXT NOT NULL,
    class_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    icon_path TEXT,
    animation_name TEXT,
    evidence_confidence TEXT NOT NULL CHECK (evidence_confidence IN ('confirmed', 'strongly_inferred', 'tentative', 'unknown')),
    semantics_status TEXT NOT NULL CHECK (semantics_status IN ('resolved', 'visual_only', 'unresolved')),
    PRIMARY KEY (release_id, skill_id),
    FOREIGN KEY (release_id, class_id) REFERENCES hunter_class_definition(release_id, class_id)
);

CREATE TABLE player_profile (
    player_token UUID PRIMARY KEY REFERENCES player_world_state(player_token) ON DELETE CASCADE,
    account_kind TEXT NOT NULL CHECK (account_kind IN ('local', 'demo')),
    display_name TEXT NOT NULL,
    seed_key TEXT,
    seed_version INTEGER NOT NULL DEFAULT 0 CHECK (seed_version >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((account_kind = 'demo' AND seed_key IS NOT NULL) OR account_kind = 'local')
);

ALTER TABLE player_hunter
    ADD COLUMN hunter_instance_id UUID NOT NULL DEFAULT gen_random_uuid(),
    ADD COLUMN content_release_id TEXT NOT NULL DEFAULT 'migration.hunter-demo-v1',
    ADD COLUMN display_name TEXT NOT NULL DEFAULT 'Hunter',
    ADD COLUMN portrait_asset_id TEXT,
    ADD COLUMN class_id TEXT NOT NULL DEFAULT 'h1',
    ADD COLUMN rarity_id TEXT NOT NULL DEFAULT 'normal',
    ADD COLUMN level INTEGER NOT NULL DEFAULT 1 CHECK (level BETWEEN 1 AND 999),
    ADD COLUMN xp BIGINT NOT NULL DEFAULT 0 CHECK (xp >= 0),
    ADD COLUMN attack BIGINT NOT NULL DEFAULT 10 CHECK (attack >= 0),
    ADD COLUMN defense BIGINT NOT NULL DEFAULT 10 CHECK (defense >= 0),
    ADD COLUMN action_state TEXT NOT NULL DEFAULT 'idle' CHECK (action_state IN ('idle', 'walking', 'serving', 'waiting', 'banished')),
    ADD COLUMN animation_name TEXT NOT NULL DEFAULT 'hunter_stay',
    ADD COLUMN state_revision BIGINT NOT NULL DEFAULT 0 CHECK (state_revision >= 0),
    ADD COLUMN seed_ordinal INTEGER CHECK (seed_ordinal IS NULL OR seed_ordinal >= 0),
    ADD CONSTRAINT player_hunter_instance_unique UNIQUE (hunter_instance_id);

CREATE TABLE player_hunter_trait (
    player_token UUID NOT NULL,
    hunter_id BIGINT NOT NULL,
    content_release_id TEXT NOT NULL,
    trait_id TEXT NOT NULL,
    unlocked_rank SMALLINT NOT NULL DEFAULT 1 CHECK (unlocked_rank BETWEEN 1 AND 4),
    equipped BOOLEAN NOT NULL DEFAULT TRUE,
    PRIMARY KEY (player_token, hunter_id, trait_id),
    FOREIGN KEY (player_token, hunter_id) REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE,
    FOREIGN KEY (content_release_id, trait_id) REFERENCES hunter_trait_definition(release_id, trait_id)
);

CREATE TABLE player_hunter_skill (
    player_token UUID NOT NULL,
    hunter_id BIGINT NOT NULL,
    content_release_id TEXT NOT NULL,
    skill_id TEXT NOT NULL,
    skill_level SMALLINT NOT NULL DEFAULT 1 CHECK (skill_level BETWEEN 1 AND 99),
    equipped_slot SMALLINT CHECK (equipped_slot BETWEEN 0 AND 7),
    cooldown_ready_at TIMESTAMPTZ,
    PRIMARY KEY (player_token, hunter_id, skill_id),
    FOREIGN KEY (player_token, hunter_id) REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE,
    FOREIGN KEY (content_release_id, skill_id) REFERENCES hunter_skill_definition(release_id, skill_id)
);

CREATE UNIQUE INDEX player_hunter_skill_slot_unique
    ON player_hunter_skill(player_token, hunter_id, equipped_slot)
    WHERE equipped_slot IS NOT NULL;

INSERT INTO hunter_content_release(release_id, game_version, status, evidence_note) VALUES
    ('migration.hunter-demo-v1', '1.411', 'fixture',
     'Test-only definitions. H1-H5 visual families and source asset names are recovered; class, rarity, trait and skill gameplay semantics remain inferred or unresolved.');

INSERT INTO hunter_class_definition
    (release_id, class_id, display_name, source_job_index, visual_family, evidence_confidence, semantics_status)
VALUES
    ('migration.hunter-demo-v1', 'h1', 'Berserker', 0, 'H1', 'strongly_inferred', 'visual_only'),
    ('migration.hunter-demo-v1', 'h2', 'Paladin',   1, 'H2', 'strongly_inferred', 'visual_only'),
    ('migration.hunter-demo-v1', 'h3', 'Ranger',    2, 'H3', 'strongly_inferred', 'visual_only'),
    ('migration.hunter-demo-v1', 'h4', 'Sorcerer',  3, 'H4', 'strongly_inferred', 'visual_only'),
    ('migration.hunter-demo-v1', 'h5', 'Lancer',    4, 'H5', 'strongly_inferred', 'visual_only');

INSERT INTO hunter_rarity_definition
    (release_id, rarity_id, display_name, rank, evidence_confidence)
VALUES
    ('migration.hunter-demo-v1', 'normal',    'Normal',    0, 'strongly_inferred'),
    ('migration.hunter-demo-v1', 'rare',      'Rare',      1, 'strongly_inferred'),
    ('migration.hunter-demo-v1', 'superior',  'Superior',  2, 'strongly_inferred'),
    ('migration.hunter-demo-v1', 'heroic',    'Heroic',    3, 'strongly_inferred'),
    ('migration.hunter-demo-v1', 'legendary', 'Legendary', 4, 'strongly_inferred');

ALTER TABLE player_hunter
    ADD CONSTRAINT player_hunter_class_fk FOREIGN KEY (content_release_id, class_id)
        REFERENCES hunter_class_definition(release_id, class_id),
    ADD CONSTRAINT player_hunter_rarity_fk FOREIGN KEY (content_release_id, rarity_id)
        REFERENCES hunter_rarity_definition(release_id, rarity_id);

INSERT INTO hunter_trait_definition
    (release_id, trait_id, class_id, branch, tier, display_name, icon_path, evidence_confidence, semantics_status)
VALUES
    ('migration.hunter-demo-v1', 'job_trait_h1_s1_01', 'h1', 1, 1, 'H1 Trait S1-01', '/content/releases/evil-hunter-1.411/hunter-assets/ui/traits/job_trait_h1_s1_01__3473.png', 'confirmed', 'unresolved'),
    ('migration.hunter-demo-v1', 'job_trait_h2_s2_01', 'h2', 2, 1, 'H2 Trait S2-01', '/content/releases/evil-hunter-1.411/hunter-assets/ui/traits/job_trait_h2_s2_01__5753.png', 'confirmed', 'unresolved'),
    ('migration.hunter-demo-v1', 'job_trait_h3_s3_01', 'h3', 3, 1, 'H3 Trait S3-01', '/content/releases/evil-hunter-1.411/hunter-assets/ui/traits/job_trait_h3_s3_01__2437.png', 'confirmed', 'unresolved'),
    ('migration.hunter-demo-v1', 'job_trait_h4_s4_01', 'h4', 4, 1, 'H4 Trait S4-01', '/content/releases/evil-hunter-1.411/hunter-assets/ui/traits/job_trait_h4_s4_01__7351.png', 'confirmed', 'unresolved'),
    ('migration.hunter-demo-v1', 'job_trait_h5_s1_01', 'h5', 1, 1, 'H5 Trait S1-01', '/content/releases/evil-hunter-1.411/hunter-assets/ui/traits/job_trait_h5_s1_01__5097.png', 'confirmed', 'unresolved'),
    ('migration.hunter-demo-v1', 'job_trait_h1_s3_02', 'h1', 3, 2, 'H1 Trait S3-02', '/content/releases/evil-hunter-1.411/hunter-assets/ui/traits/job_trait_h1_s3_02__4984.png', 'confirmed', 'unresolved'),
    ('migration.hunter-demo-v1', 'job_trait_h2_s4_03', 'h2', 4, 3, 'H2 Trait S4-03', '/content/releases/evil-hunter-1.411/hunter-assets/ui/traits/job_trait_h2_s4_03__6615.png', 'confirmed', 'unresolved'),
    ('migration.hunter-demo-v1', 'job_trait_h4_s2_03', 'h4', 2, 3, 'H4 Trait S2-03', '/content/releases/evil-hunter-1.411/hunter-assets/ui/traits/job_trait_h4_s2_03__3230.png', 'confirmed', 'unresolved');

INSERT INTO hunter_skill_definition
    (release_id, skill_id, class_id, display_name, animation_name, evidence_confidence, semantics_status)
VALUES
    ('migration.hunter-demo-v1', 'h1_whirlwind',    'h1', 'Whirlwind',     'h1_hit_whirlwind', 'confirmed', 'visual_only'),
    ('migration.hunter-demo-v1', 'h2_executor',     'h2', 'Executor',      'h2_hit_executor', 'confirmed', 'visual_only'),
    ('migration.hunter-demo-v1', 'h3_arcane',       'h3', 'Arcane',        'h3_hit_arcane', 'confirmed', 'visual_only'),
    ('migration.hunter-demo-v1', 'h4_darkload',     'h4', 'Darkload',      'h4_hit_darkload', 'confirmed', 'visual_only'),
    ('migration.hunter-demo-v1', 'h5_roundslash',   'h5', 'Round Slash',   'h5_hit_roundslash', 'confirmed', 'visual_only'),
    ('migration.hunter-demo-v1', 'h5_dragonbreath', 'h5', 'Dragon Breath', 'h5_hit_dragonbreath_vehicle', 'confirmed', 'visual_only');

-- Shared public test account. The cookie token is intentionally non-secret because all state is disposable demo data.
INSERT INTO player_world_state(player_token, state)
VALUES ('00000000-0000-4000-8000-00000000a001', '{"schema_version":11}'::jsonb)
ON CONFLICT (player_token) DO NOTHING;

INSERT INTO local_identities(token_hash, player_token)
VALUES (decode('19630c7f4811fdf6fe56d1c9978ec156d2b13b1e04f7017bc3a07e347baa943d', 'hex'),
        '00000000-0000-4000-8000-00000000a001')
ON CONFLICT (token_hash) DO UPDATE SET player_token = EXCLUDED.player_token;

INSERT INTO player_profile(player_token, account_kind, display_name, seed_key, seed_version)
VALUES ('00000000-0000-4000-8000-00000000a001', 'demo', 'Hunter Lab', 'hunter-lab:20260724', 1)
ON CONFLICT (player_token) DO UPDATE
SET account_kind = EXCLUDED.account_kind,
    display_name = EXCLUDED.display_name,
    seed_key = EXCLUDED.seed_key,
    seed_version = EXCLUDED.seed_version,
    updated_at = now();

INSERT INTO player_hunter_roster(player_token, roster_resolved, wallets_resolved, next_arrival_sequence)
VALUES ('00000000-0000-4000-8000-00000000a001', TRUE, TRUE, 1)
ON CONFLICT (player_token) DO UPDATE
SET roster_resolved = TRUE, wallets_resolved = TRUE, next_arrival_sequence = 1, updated_at = now();

INSERT INTO player_hunter
    (player_token, hunter_id, roster_state, roster_position, gold, current_hp, max_hp,
     stamina_current, stamina_maximum, satiety_current, satiety_maximum, mood_current, mood_maximum,
     content_release_id, display_name, portrait_asset_id, class_id, rarity_id, level, xp,
     attack, defense, action_state, animation_name, seed_ordinal)
VALUES
    ('00000000-0000-4000-8000-00000000a001', 1, 'active', 0, 1800, 128, 140, 88,100, 76,100, 91,100, 'migration.hunter-demo-v1', 'Astra', '/content/releases/evil-hunter-1.411/hunter-assets/portraits/hunter_f_01__1728.png',  'h4', 'legendary', 18, 4200, 38, 17, 'idle', 'hunter_stay', 0),
    ('00000000-0000-4000-8000-00000000a001', 2, 'active', 1, 1250, 165, 180, 72,100, 83,100, 68,100, 'migration.hunter-demo-v1', 'Bram',  '/content/releases/evil-hunter-1.411/hunter-assets/portraits/hunter_m_21__5966.png',  'h1', 'heroic',    16, 3500, 42, 20, 'idle', 'hunter_stay', 1),
    ('00000000-0000-4000-8000-00000000a001', 3, 'active', 2, 920,  142, 150, 94,100, 65,100, 85,100, 'migration.hunter-demo-v1', 'Celine','/content/releases/evil-hunter-1.411/hunter-assets/portraits/hunter_f_41__4928.png',  'h3', 'superior',  14, 2800, 34, 15, 'idle', 'hunter_stay', 2),
    ('00000000-0000-4000-8000-00000000a001', 4, 'active', 3, 2100, 196, 210, 61,100, 92,100, 74,100, 'migration.hunter-demo-v1', 'Doran', '/content/releases/evil-hunter-1.411/hunter-assets/portraits/hunter_m_61__3509.png',  'h2', 'legendary', 20, 5100, 31, 44, 'idle', 'hunter_stay', 3),
    ('00000000-0000-4000-8000-00000000a001', 5, 'active', 4, 760,  118, 130, 80,100, 88,100, 96,100, 'migration.hunter-demo-v1', 'Elara', '/content/releases/evil-hunter-1.411/hunter-assets/portraits/hunter_f_81__6230.png',  'h5', 'rare',      11, 1700, 29, 18, 'idle', 'hunter_stay', 4),
    ('00000000-0000-4000-8000-00000000a001', 6, 'active', 5, 1050, 151, 160, 69,100, 71,100, 79,100, 'migration.hunter-demo-v1', 'Finn',  '/content/releases/evil-hunter-1.411/hunter-assets/portraits/hunter_m_101__7164.png', 'h1', 'superior',  15, 3100, 39, 19, 'idle', 'hunter_stay', 5),
    ('00000000-0000-4000-8000-00000000a001', 7, 'active', 6, 630,  104, 120, 97,100, 58,100, 72,100, 'migration.hunter-demo-v1', 'Gwen',  '/content/releases/evil-hunter-1.411/hunter-assets/portraits/hunter_f_121__5341.png', 'h2', 'normal',     9,  900,  24, 31, 'idle', 'hunter_stay', 6),
    ('00000000-0000-4000-8000-00000000a001', 8, 'active', 7, 1450, 136, 145, 84,100, 79,100, 89,100, 'migration.hunter-demo-v1', 'Hale',  '/content/releases/evil-hunter-1.411/hunter-assets/portraits/hunter_m_141__3522.png', 'h4', 'heroic',    17, 3900, 41, 16, 'idle', 'hunter_stay', 7)
ON CONFLICT (player_token, hunter_id) DO UPDATE
SET roster_state = EXCLUDED.roster_state,
    roster_position = EXCLUDED.roster_position,
    arrival_sequence = NULL,
    gold = EXCLUDED.gold,
    current_hp = EXCLUDED.current_hp,
    max_hp = EXCLUDED.max_hp,
    stamina_current = EXCLUDED.stamina_current,
    stamina_maximum = EXCLUDED.stamina_maximum,
    satiety_current = EXCLUDED.satiety_current,
    satiety_maximum = EXCLUDED.satiety_maximum,
    mood_current = EXCLUDED.mood_current,
    mood_maximum = EXCLUDED.mood_maximum,
    content_release_id = EXCLUDED.content_release_id,
    display_name = EXCLUDED.display_name,
    portrait_asset_id = EXCLUDED.portrait_asset_id,
    class_id = EXCLUDED.class_id,
    rarity_id = EXCLUDED.rarity_id,
    level = EXCLUDED.level,
    xp = EXCLUDED.xp,
    attack = EXCLUDED.attack,
    defense = EXCLUDED.defense,
    action_state = EXCLUDED.action_state,
    animation_name = EXCLUDED.animation_name,
    seed_ordinal = EXCLUDED.seed_ordinal;

INSERT INTO player_hunter_trait(player_token, hunter_id, content_release_id, trait_id, unlocked_rank, equipped)
VALUES
    ('00000000-0000-4000-8000-00000000a001',1,'migration.hunter-demo-v1','job_trait_h4_s4_01',1,TRUE),
    ('00000000-0000-4000-8000-00000000a001',2,'migration.hunter-demo-v1','job_trait_h1_s1_01',1,TRUE),
    ('00000000-0000-4000-8000-00000000a001',3,'migration.hunter-demo-v1','job_trait_h3_s3_01',1,TRUE),
    ('00000000-0000-4000-8000-00000000a001',4,'migration.hunter-demo-v1','job_trait_h2_s2_01',1,TRUE),
    ('00000000-0000-4000-8000-00000000a001',5,'migration.hunter-demo-v1','job_trait_h5_s1_01',1,TRUE),
    ('00000000-0000-4000-8000-00000000a001',6,'migration.hunter-demo-v1','job_trait_h1_s3_02',2,TRUE),
    ('00000000-0000-4000-8000-00000000a001',7,'migration.hunter-demo-v1','job_trait_h2_s4_03',3,TRUE),
    ('00000000-0000-4000-8000-00000000a001',8,'migration.hunter-demo-v1','job_trait_h4_s2_03',3,TRUE)
ON CONFLICT (player_token, hunter_id, trait_id) DO UPDATE
SET unlocked_rank = EXCLUDED.unlocked_rank, equipped = EXCLUDED.equipped;

INSERT INTO player_hunter_skill(player_token, hunter_id, content_release_id, skill_id, skill_level, equipped_slot)
VALUES
    ('00000000-0000-4000-8000-00000000a001',1,'migration.hunter-demo-v1','h4_darkload',2,0),
    ('00000000-0000-4000-8000-00000000a001',2,'migration.hunter-demo-v1','h1_whirlwind',2,0),
    ('00000000-0000-4000-8000-00000000a001',3,'migration.hunter-demo-v1','h3_arcane',1,0),
    ('00000000-0000-4000-8000-00000000a001',4,'migration.hunter-demo-v1','h2_executor',3,0),
    ('00000000-0000-4000-8000-00000000a001',5,'migration.hunter-demo-v1','h5_roundslash',1,0),
    ('00000000-0000-4000-8000-00000000a001',5,'migration.hunter-demo-v1','h5_dragonbreath',1,1),
    ('00000000-0000-4000-8000-00000000a001',6,'migration.hunter-demo-v1','h1_whirlwind',2,0),
    ('00000000-0000-4000-8000-00000000a001',7,'migration.hunter-demo-v1','h2_executor',1,0),
    ('00000000-0000-4000-8000-00000000a001',8,'migration.hunter-demo-v1','h4_darkload',2,0)
ON CONFLICT (player_token, hunter_id, skill_id) DO UPDATE
SET skill_level = EXCLUDED.skill_level, equipped_slot = EXCLUDED.equipped_slot;
