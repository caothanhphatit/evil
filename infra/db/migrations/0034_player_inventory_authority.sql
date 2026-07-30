-- Durable player inventory authority.
--
-- `player_hunter.owned_items` was an aggregate compatibility field. New
-- writes use these relations so stacked products and individually rolled gear
-- have different integrity rules. The legacy JSONB column remains readable
-- until all existing accounts have been rewritten by an authoritative save.

CREATE TABLE player_hunter_item_stack (
    player_token UUID NOT NULL,
    hunter_id BIGINT NOT NULL,
    content_release_id TEXT NOT NULL,
    product_id TEXT NOT NULL,
    quantity BIGINT NOT NULL CHECK (quantity > 0),
    PRIMARY KEY (player_token, hunter_id, content_release_id, product_id),
    FOREIGN KEY (player_token, hunter_id)
        REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE,
    FOREIGN KEY (content_release_id, product_id)
        REFERENCES economy_product_definition(release_id, product_id) ON DELETE RESTRICT
);

CREATE TABLE player_hunter_gear_instance (
    gear_instance_id UUID PRIMARY KEY,
    player_token UUID NOT NULL,
    hunter_id BIGINT NOT NULL,
    content_release_id TEXT NOT NULL,
    product_id TEXT NOT NULL,
    gear_kind TEXT NOT NULL CHECK (btrim(gear_kind) <> ''),
    gear_index INTEGER NOT NULL CHECK (gear_index >= 0),
    rating SMALLINT NOT NULL CHECK (rating BETWEEN 0 AND 4),
    enhancement_level SMALLINT NOT NULL DEFAULT 0 CHECK (enhancement_level BETWEEN 0 AND 20),
    quality SMALLINT CHECK (quality IS NULL OR quality BETWEEN 0 AND 4),
    primary_stat BIGINT CHECK (primary_stat IS NULL OR primary_stat >= 0),
    option_type SMALLINT CHECK (option_type IS NULL OR option_type >= 0),
    option_value INTEGER CHECK (option_value IS NULL OR option_value >= 0),
    ruleset TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (player_token, hunter_id)
        REFERENCES player_hunter(player_token, hunter_id) ON DELETE CASCADE,
    FOREIGN KEY (content_release_id, product_id)
        REFERENCES economy_product_definition(release_id, product_id) ON DELETE RESTRICT,
    UNIQUE (player_token, gear_instance_id)
);

CREATE INDEX player_hunter_item_stack_lookup_idx
    ON player_hunter_item_stack (player_token, hunter_id, product_id);

CREATE INDEX player_hunter_gear_instance_lookup_idx
    ON player_hunter_gear_instance (player_token, hunter_id, product_id);

COMMENT ON COLUMN player_hunter.owned_items IS
    'Legacy compatibility snapshot. Authoritative writes use player_hunter_item_stack and player_hunter_gear_instance.';
