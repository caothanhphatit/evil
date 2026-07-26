-- Authoritative building schema. Content rows are immutable and versioned;
-- 0008 publishes the recovered release and 0009 projects legacy player JSONB.
CREATE TABLE content_release (
    release_id TEXT PRIMARY KEY CHECK (btrim(release_id) <> ''),
    registry_sha256 BYTEA NOT NULL UNIQUE CHECK (octet_length(registry_sha256) = 32),
    lifecycle TEXT NOT NULL CHECK (lifecycle IN ('draft', 'active', 'retired')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    activated_at TIMESTAMPTZ,
    CHECK (lifecycle <> 'active' OR activated_at IS NOT NULL)
);

CREATE UNIQUE INDEX content_release_one_active_idx
    ON content_release ((lifecycle)) WHERE lifecycle = 'active';

CREATE TABLE building_definition (
    release_id TEXT NOT NULL REFERENCES content_release(release_id) ON DELETE RESTRICT,
    building_id TEXT NOT NULL CHECK (building_id ~ '^build_[0-9]+$'),
    display_name TEXT NOT NULL,
    category TEXT,
    source_type BIGINT NOT NULL,
    max_instances INTEGER NOT NULL CHECK (max_instances > 0),
    grid_width INTEGER NOT NULL CHECK (grid_width > 0),
    grid_height INTEGER NOT NULL CHECK (grid_height > 0),
    movable BOOLEAN,
    constructible BOOLEAN,
    base_sprite_asset_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (release_id, building_id)
);

CREATE TABLE building_level_definition (
    release_id TEXT NOT NULL,
    building_id TEXT NOT NULL,
    level INTEGER NOT NULL CHECK (level > 0),
    upgrade_duration_ms BIGINT CHECK (upgrade_duration_ms IS NULL OR upgrade_duration_ms >= 0),
    inventory_capacity BIGINT CHECK (inventory_capacity IS NULL OR inventory_capacity >= 0),
    production_slots INTEGER CHECK (production_slots IS NULL OR production_slots >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (release_id, building_id, level),
    FOREIGN KEY (release_id, building_id)
        REFERENCES building_definition(release_id, building_id) ON DELETE CASCADE
);

CREATE TABLE building_level_cost (
    release_id TEXT NOT NULL,
    building_id TEXT NOT NULL,
    level INTEGER NOT NULL,
    item_id TEXT NOT NULL CHECK (btrim(item_id) <> ''),
    quantity BIGINT NOT NULL CHECK (quantity >= 0),
    PRIMARY KEY (release_id, building_id, level, item_id),
    FOREIGN KEY (release_id, building_id, level)
        REFERENCES building_level_definition(release_id, building_id, level) ON DELETE CASCADE
);

CREATE TABLE building_level_prerequisite (
    release_id TEXT NOT NULL,
    building_id TEXT NOT NULL,
    level INTEGER NOT NULL,
    required_building_id TEXT NOT NULL,
    required_level INTEGER NOT NULL CHECK (required_level > 0),
    PRIMARY KEY (release_id, building_id, level, required_building_id),
    FOREIGN KEY (release_id, building_id, level)
        REFERENCES building_level_definition(release_id, building_id, level) ON DELETE CASCADE,
    FOREIGN KEY (release_id, required_building_id, required_level)
        REFERENCES building_level_definition(release_id, building_id, level) ON DELETE RESTRICT
);

CREATE TABLE building_skin_definition (
    release_id TEXT NOT NULL,
    building_id TEXT NOT NULL,
    skin_id BIGINT NOT NULL CHECK (skin_id > 0),
    family TEXT NOT NULL,
    display_name TEXT NOT NULL,
    required_level INTEGER NOT NULL CHECK (required_level >= 0),
    visibility BIGINT NOT NULL,
    asset_key TEXT,
    sprite_prefix TEXT,
    visual_resolved BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (release_id, building_id, skin_id),
    FOREIGN KEY (release_id, building_id)
        REFERENCES building_definition(release_id, building_id) ON DELETE CASCADE,
    CHECK (
        (visual_resolved AND asset_key IS NOT NULL AND sprite_prefix IS NOT NULL)
        OR (NOT visual_resolved AND asset_key IS NULL AND sprite_prefix IS NULL)
    )
);

CREATE INDEX building_skin_definition_unlock_idx
    ON building_skin_definition (release_id, building_id, required_level, skin_id);

CREATE TABLE town_template (
    template_id TEXT PRIMARY KEY CHECK (btrim(template_id) <> ''),
    release_id TEXT NOT NULL REFERENCES content_release(release_id) ON DELETE RESTRICT,
    display_name TEXT NOT NULL,
    seed_version INTEGER NOT NULL CHECK (seed_version >= 0),
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (template_id, release_id)
);

CREATE UNIQUE INDEX town_template_one_default_per_release_idx
    ON town_template (release_id) WHERE is_default;

CREATE TABLE town_template_building (
    template_id TEXT NOT NULL,
    release_id TEXT NOT NULL,
    slot INTEGER NOT NULL CHECK (slot >= 0),
    building_id TEXT NOT NULL,
    level INTEGER NOT NULL CHECK (level > 0),
    equipped_skin_id BIGINT,
    grid_x INTEGER NOT NULL,
    grid_y INTEGER NOT NULL,
    PRIMARY KEY (template_id, slot),
    FOREIGN KEY (template_id, release_id)
        REFERENCES town_template(template_id, release_id) ON DELETE CASCADE,
    FOREIGN KEY (release_id, building_id, level)
        REFERENCES building_level_definition(release_id, building_id, level) ON DELETE RESTRICT,
    FOREIGN KEY (release_id, building_id, equipped_skin_id)
        REFERENCES building_skin_definition(release_id, building_id, skin_id) ON DELETE RESTRICT
);

CREATE TABLE town (
    town_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_token UUID NOT NULL UNIQUE REFERENCES player_world_state(player_token) ON DELETE CASCADE,
    release_id TEXT NOT NULL REFERENCES content_release(release_id) ON DELETE RESTRICT,
    source_template_id TEXT,
    gold BIGINT NOT NULL DEFAULT 0 CHECK (gold >= 0),
    seed_version INTEGER NOT NULL DEFAULT 0 CHECK (seed_version >= 0),
    next_building_sequence BIGINT NOT NULL DEFAULT 1 CHECK (next_building_sequence > 0),
    revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0),
    legacy_source_revision BIGINT CHECK (legacy_source_revision IS NULL OR legacy_source_revision >= 0),
    legacy_backfilled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (town_id, release_id),
    FOREIGN KEY (source_template_id, release_id)
        REFERENCES town_template(template_id, release_id) ON DELETE RESTRICT
);

CREATE TABLE player_building (
    instance_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    town_id UUID NOT NULL,
    release_id TEXT NOT NULL,
    legacy_instance_id TEXT,
    building_id TEXT NOT NULL,
    current_level INTEGER NOT NULL CHECK (current_level > 0),
    equipped_skin_id BIGINT,
    grid_x INTEGER NOT NULL,
    grid_y INTEGER NOT NULL,
    use_count BIGINT NOT NULL DEFAULT 0 CHECK (use_count >= 0),
    seeded_by TEXT,
    constructed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (town_id, instance_id),
    UNIQUE (town_id, instance_id, release_id, building_id),
    UNIQUE (town_id, legacy_instance_id),
    FOREIGN KEY (town_id, release_id)
        REFERENCES town(town_id, release_id) ON DELETE CASCADE,
    FOREIGN KEY (release_id, building_id, current_level)
        REFERENCES building_level_definition(release_id, building_id, level) ON DELETE RESTRICT,
    FOREIGN KEY (release_id, building_id, equipped_skin_id)
        REFERENCES building_skin_definition(release_id, building_id, skin_id) ON DELETE RESTRICT
);

CREATE INDEX player_building_definition_idx
    ON player_building (town_id, release_id, building_id);

CREATE TABLE town_economy_summary (
    town_id UUID PRIMARY KEY REFERENCES town(town_id) ON DELETE CASCADE,
    hunter_materials BIGINT NOT NULL DEFAULT 0 CHECK (hunter_materials >= 0),
    materials BIGINT NOT NULL DEFAULT 0 CHECK (materials >= 0),
    runes BIGINT NOT NULL DEFAULT 0 CHECK (runes >= 0),
    weapons BIGINT NOT NULL DEFAULT 0 CHECK (weapons >= 0),
    armor BIGINT NOT NULL DEFAULT 0 CHECK (armor >= 0),
    hunter_equipment_purchases BIGINT NOT NULL DEFAULT 0
        CHECK (hunter_equipment_purchases >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE town_trade_state (
    town_id UUID PRIMARY KEY REFERENCES town(town_id) ON DELETE CASCADE,
    field_trip_id BIGINT NOT NULL DEFAULT 0 CHECK (field_trip_id >= 0),
    settled_field_trip_id BIGINT NOT NULL DEFAULT 0 CHECK (settled_field_trip_id >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (settled_field_trip_id <= field_trip_id)
);

CREATE TABLE hunter_material_stack (
    town_id UUID NOT NULL REFERENCES town(town_id) ON DELETE CASCADE,
    material_id TEXT NOT NULL CHECK (btrim(material_id) <> ''),
    quantity BIGINT NOT NULL CHECK (quantity >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (town_id, material_id)
);

CREATE TABLE hunter_trade_settlement (
    town_id UUID NOT NULL REFERENCES town(town_id) ON DELETE CASCADE,
    settlement_id TEXT NOT NULL CHECK (btrim(settlement_id) <> ''),
    field_trip_id BIGINT NOT NULL CHECK (field_trip_id > 0),
    material_id TEXT NOT NULL CHECK (btrim(material_id) <> ''),
    quantity BIGINT NOT NULL CHECK (quantity > 0),
    unit_price BIGINT NOT NULL CHECK (unit_price >= 0),
    total_gold BIGINT NOT NULL CHECK (total_gold >= 0),
    settled_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (town_id, settlement_id)
);

CREATE INDEX hunter_trade_settlement_trip_idx
    ON hunter_trade_settlement (town_id, field_trip_id);

CREATE TABLE town_inventory_stack (
    town_id UUID NOT NULL REFERENCES town(town_id) ON DELETE CASCADE,
    item_id TEXT NOT NULL CHECK (btrim(item_id) <> ''),
    quantity BIGINT NOT NULL CHECK (quantity >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (town_id, item_id)
);

CREATE TABLE town_inventory_ledger (
    entry_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    town_id UUID NOT NULL,
    item_id TEXT NOT NULL,
    quantity_delta BIGINT NOT NULL CHECK (quantity_delta <> 0),
    balance_after BIGINT NOT NULL CHECK (balance_after >= 0),
    reason TEXT NOT NULL CHECK (btrim(reason) <> ''),
    operation_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (town_id, item_id)
        REFERENCES town_inventory_stack(town_id, item_id) ON DELETE RESTRICT,
    UNIQUE (town_id, operation_id, item_id)
);

CREATE INDEX town_inventory_ledger_created_idx
    ON town_inventory_ledger (town_id, created_at DESC);

CREATE TABLE building_material_order (
    order_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    town_id UUID NOT NULL REFERENCES town(town_id) ON DELETE CASCADE,
    material_id TEXT NOT NULL CHECK (btrim(material_id) <> ''),
    requested_quantity BIGINT NOT NULL CHECK (requested_quantity > 0),
    fulfilled_quantity BIGINT NOT NULL DEFAULT 0 CHECK (fulfilled_quantity >= 0),
    unit_price BIGINT NOT NULL CHECK (unit_price >= 0),
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'fulfilled', 'cancelled')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (fulfilled_quantity <= requested_quantity),
    CHECK (status <> 'fulfilled' OR fulfilled_quantity = requested_quantity)
);

CREATE UNIQUE INDEX building_material_order_one_open_idx
    ON building_material_order (town_id, material_id) WHERE status = 'open';

CREATE TABLE building_production_job (
    job_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    town_id UUID NOT NULL,
    building_instance_id UUID NOT NULL,
    release_id TEXT NOT NULL,
    building_id TEXT NOT NULL,
    product_id TEXT NOT NULL CHECK (btrim(product_id) <> ''),
    quantity BIGINT NOT NULL CHECK (quantity > 0),
    queue_position INTEGER NOT NULL CHECK (queue_position >= 0),
    status TEXT NOT NULL DEFAULT 'queued'
        CHECK (status IN ('queued', 'running', 'completed', 'cancelled')),
    queued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at TIMESTAMPTZ,
    completes_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    FOREIGN KEY (town_id, building_instance_id, release_id, building_id)
        REFERENCES player_building(town_id, instance_id, release_id, building_id)
        ON DELETE CASCADE,
    CHECK (started_at IS NULL OR started_at >= queued_at),
    CHECK (completes_at IS NULL OR (started_at IS NOT NULL AND completes_at >= started_at)),
    CHECK (completed_at IS NULL OR status IN ('completed', 'cancelled'))
);

CREATE UNIQUE INDEX building_production_job_live_position_idx
    ON building_production_job (town_id, building_instance_id, queue_position)
    WHERE status IN ('queued', 'running');

CREATE TABLE building_upgrade_job (
    job_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    town_id UUID NOT NULL,
    building_instance_id UUID NOT NULL,
    release_id TEXT NOT NULL,
    building_id TEXT NOT NULL,
    target_level INTEGER NOT NULL CHECK (target_level > 0),
    status TEXT NOT NULL DEFAULT 'queued'
        CHECK (status IN ('queued', 'running', 'completed', 'cancelled')),
    queued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at TIMESTAMPTZ,
    completes_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    FOREIGN KEY (town_id, building_instance_id, release_id, building_id)
        REFERENCES player_building(town_id, instance_id, release_id, building_id)
        ON DELETE CASCADE,
    FOREIGN KEY (release_id, building_id, target_level)
        REFERENCES building_level_definition(release_id, building_id, level) ON DELETE RESTRICT,
    CHECK (started_at IS NULL OR started_at >= queued_at),
    CHECK (completes_at IS NULL OR (started_at IS NOT NULL AND completes_at >= started_at)),
    CHECK (completed_at IS NULL OR status IN ('completed', 'cancelled'))
);

CREATE UNIQUE INDEX building_upgrade_job_one_live_idx
    ON building_upgrade_job (town_id, building_instance_id)
    WHERE status IN ('queued', 'running');

CREATE TABLE building_normalization_issue (
    issue_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    player_token UUID NOT NULL REFERENCES player_world_state(player_token) ON DELETE CASCADE,
    issue_kind TEXT NOT NULL,
    source_pointer TEXT NOT NULL,
    source_payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX building_normalization_issue_player_idx
    ON building_normalization_issue (player_token, issue_id);

COMMENT ON TABLE building_base_catalog IS
    'Deprecated import staging from migrations 0005-0006; not authoritative runtime state.';
COMMENT ON TABLE building_skin_catalog IS
    'Deprecated import staging from migrations 0005-0006; not authoritative runtime state.';
