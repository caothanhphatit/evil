ALTER TABLE player_hunter_fixture_equipment
    DROP CONSTRAINT IF EXISTS player_hunter_fixture_equipment_check,
    DROP CONSTRAINT IF EXISTS player_hunter_fixture_equipment_evidence_state_check;

ALTER TABLE player_hunter_fixture_equipment
    ADD CONSTRAINT player_hunter_fixture_equipment_check
    CHECK (
        (catalog_kind = 'weapon' OR catalog_kind LIKE 'rebuild_weapon_instance:%')
        = (required_class_id IS NOT NULL)
    ),
    ADD CONSTRAINT player_hunter_fixture_equipment_evidence_state_check
    CHECK (evidence_state IN ('web_rebuild_test_fixture', 'web_rebuild_weapon_v1'));
