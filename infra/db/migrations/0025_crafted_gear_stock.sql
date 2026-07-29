CREATE TABLE crafted_gear_stock (
    town_id UUID NOT NULL REFERENCES town(town_id) ON DELETE CASCADE,
    gear_instance_id UUID NOT NULL,
    building_instance_id UUID NOT NULL,
    product_id TEXT NOT NULL,
    gear_kind TEXT NOT NULL,
    rating SMALLINT NOT NULL CHECK (rating >= 0),
    quality SMALLINT NOT NULL CHECK (quality BETWEEN 0 AND 4),
    primary_stat BIGINT NOT NULL CHECK (primary_stat >= 0),
    option_type SMALLINT NOT NULL CHECK (option_type >= 0),
    option_value INTEGER NOT NULL CHECK (option_value >= 0),
    icon_path TEXT NOT NULL,
    ruleset TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (town_id, gear_instance_id),
    FOREIGN KEY (town_id, building_instance_id)
        REFERENCES player_building(town_id, instance_id) ON DELETE CASCADE
);

CREATE INDEX crafted_gear_stock_shop_idx
    ON crafted_gear_stock (town_id, building_instance_id, product_id, created_at);
