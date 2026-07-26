ALTER TABLE building_skin_catalog
    DROP CONSTRAINT building_skin_catalog_required_level_check;

ALTER TABLE building_skin_catalog
    ADD CONSTRAINT building_skin_catalog_required_level_check
    CHECK (required_level >= 0);
