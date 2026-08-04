-- Allow the existing fixture equipment relation to carry an exact rebuild
-- weapon instance reference without introducing a parallel loadout table.
ALTER TABLE player_hunter_fixture_equipment
    DROP CONSTRAINT IF EXISTS player_hunter_fixture_equipment_catalog_kind_check;

ALTER TABLE player_hunter_fixture_equipment
    ADD CONSTRAINT player_hunter_fixture_equipment_catalog_kind_check
    CHECK (
        catalog_kind IN ('gloves', 'boots', 'weapon', 'armor')
        OR catalog_kind LIKE 'rebuild_weapon_instance:%'
    );
