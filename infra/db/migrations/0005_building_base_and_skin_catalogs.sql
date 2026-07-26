CREATE TABLE building_base_catalog (
    building_id TEXT PRIMARY KEY CHECK (building_id ~ '^build_[0-9]+$'),
    registry_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    category TEXT,
    source_type BIGINT NOT NULL,
    max_instances BIGINT NOT NULL CHECK (max_instances > 0),
    max_level BIGINT NOT NULL CHECK (max_level > 0),
    grid_width BIGINT NOT NULL CHECK (grid_width > 0),
    grid_height BIGINT NOT NULL CHECK (grid_height > 0),
    base_sprite_asset_id TEXT,
    seed_by_default BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE building_skin_catalog (
    building_id TEXT NOT NULL REFERENCES building_base_catalog(building_id) ON DELETE CASCADE,
    skin_id BIGINT NOT NULL CHECK (skin_id > 0),
    family TEXT NOT NULL,
    display_name TEXT NOT NULL,
    required_level BIGINT NOT NULL CHECK (required_level > 0),
    visibility BIGINT NOT NULL,
    asset_key TEXT,
    sprite_prefix TEXT,
    visual_resolved BOOLEAN NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (building_id, skin_id),
    CHECK (
        (visual_resolved AND asset_key IS NOT NULL AND sprite_prefix IS NOT NULL)
        OR (NOT visual_resolved AND asset_key IS NULL AND sprite_prefix IS NULL)
    )
);

CREATE INDEX building_skin_catalog_building_idx
    ON building_skin_catalog (building_id, required_level, skin_id);
