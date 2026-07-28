-- Operational rebuild fixture ownership. Catalog identities and icons come from
-- gear-catalog.json; these assignments are test data, not recovered player state.
CREATE TABLE player_hunter_fixture_equipment (
    player_token UUID NOT NULL,
    hunter_id BIGINT NOT NULL,
    slot_id TEXT NOT NULL CHECK (slot_id IN ('gloves', 'boots', 'weapon', 'armor')),
    slot_order SMALLINT NOT NULL CHECK (slot_order BETWEEN 0 AND 7),
    catalog_kind TEXT NOT NULL CHECK (catalog_kind IN ('gloves', 'boots', 'weapon', 'armor')),
    catalog_index INTEGER NOT NULL CHECK (catalog_index >= 0),
    display_name TEXT NOT NULL,
    icon_path TEXT NOT NULL,
    presentation_gender TEXT NOT NULL CHECK (presentation_gender IN ('female', 'male')),
    required_class_id TEXT,
    locked BOOLEAN NOT NULL DEFAULT FALSE,
    evidence_state TEXT NOT NULL CHECK (evidence_state = 'web_rebuild_test_fixture'),
    PRIMARY KEY (player_token, hunter_id, slot_id),
    UNIQUE (player_token, hunter_id, slot_order),
    FOREIGN KEY (player_token, hunter_id)
        REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE,
    CHECK ((catalog_kind = 'weapon') = (required_class_id IS NOT NULL))
);

COMMENT ON TABLE player_hunter_fixture_equipment IS
    'Disposable operational equipment assignments; never runtime_evidence/source_* data.';

WITH fixture_hunter AS (
    SELECT player_token, hunter_id, class_id,
           CASE WHEN hunter_id % 2 = 0 THEN 'male' ELSE 'female' END AS gender
    FROM player_hunter
    WHERE player_token = '00000000-0000-4000-8000-00000000a001'
), fixture_row(slot_id, slot_order, catalog_kind, catalog_index, display_name, icon_file) AS (
    VALUES
        ('gloves', 0, 'gloves', 0, 'Tattered Gloves', 'gloves-0.png'),
        ('boots',   3, 'boots',  0, 'Tattered Shoes',  'boots-0.png'),
        ('armor',   6, 'armor',  0, 'Tattered Armor',  'armor-0.png')
)
INSERT INTO player_hunter_fixture_equipment
    (player_token, hunter_id, slot_id, slot_order, catalog_kind, catalog_index,
     display_name, icon_path, presentation_gender, required_class_id, evidence_state)
SELECT h.player_token, h.hunter_id, r.slot_id, r.slot_order, r.catalog_kind, r.catalog_index,
       r.display_name, '/content/releases/evil-hunter-1.411/gear-icons/' || r.icon_file,
       h.gender, NULL, 'web_rebuild_test_fixture'
FROM fixture_hunter h CROSS JOIN fixture_row r
ON CONFLICT (player_token, hunter_id, slot_id) DO UPDATE SET
    slot_order = EXCLUDED.slot_order,
    catalog_kind = EXCLUDED.catalog_kind,
    catalog_index = EXCLUDED.catalog_index,
    display_name = EXCLUDED.display_name,
    icon_path = EXCLUDED.icon_path,
    presentation_gender = EXCLUDED.presentation_gender,
    required_class_id = EXCLUDED.required_class_id,
    evidence_state = EXCLUDED.evidence_state;

WITH fixture_weapon(class_id, catalog_index, display_name, icon_file) AS (
    VALUES
        ('h1', 0,   'Junk Sword',  'weapon-0.png'),
        ('h2', 9,   'Junk Hammer', 'weapon-9.png'),
        ('h3', 18,  'Junk Bow',    'weapon-18.png'),
        ('h4', 27,  'Junk Staff',  'weapon-27.png'),
        ('h5', 252, 'Rusty Spear', 'weapon-252.png')
)
INSERT INTO player_hunter_fixture_equipment
    (player_token, hunter_id, slot_id, slot_order, catalog_kind, catalog_index,
     display_name, icon_path, presentation_gender, required_class_id, evidence_state)
SELECT h.player_token, h.hunter_id, 'weapon', 5, 'weapon', w.catalog_index,
       w.display_name, '/content/releases/evil-hunter-1.411/gear-icons/' || w.icon_file,
       CASE WHEN h.hunter_id % 2 = 0 THEN 'male' ELSE 'female' END,
       h.class_id, 'web_rebuild_test_fixture'
FROM player_hunter h
JOIN fixture_weapon w ON w.class_id = h.class_id
WHERE h.player_token = '00000000-0000-4000-8000-00000000a001'
ON CONFLICT (player_token, hunter_id, slot_id) DO UPDATE SET
    slot_order = EXCLUDED.slot_order,
    catalog_kind = EXCLUDED.catalog_kind,
    catalog_index = EXCLUDED.catalog_index,
    display_name = EXCLUDED.display_name,
    icon_path = EXCLUDED.icon_path,
    presentation_gender = EXCLUDED.presentation_gender,
    required_class_id = EXCLUDED.required_class_id,
    evidence_state = EXCLUDED.evidence_state;

UPDATE player_profile
SET seed_key = 'hunter-lab:20260727', seed_version = GREATEST(seed_version, 2), updated_at = now()
WHERE player_token = '00000000-0000-4000-8000-00000000a001';
