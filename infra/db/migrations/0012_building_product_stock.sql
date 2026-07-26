CREATE TABLE building_product_stock (
    town_id UUID NOT NULL,
    building_instance_id UUID NOT NULL,
    release_id TEXT NOT NULL,
    building_id TEXT NOT NULL,
    product_id TEXT NOT NULL CHECK (btrim(product_id) <> ''),
    quantity BIGINT NOT NULL DEFAULT 0 CHECK (quantity >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (town_id, building_instance_id, product_id),
    FOREIGN KEY (town_id, building_instance_id, release_id, building_id)
        REFERENCES player_building(town_id, instance_id, release_id, building_id)
        ON DELETE CASCADE,
    FOREIGN KEY (release_id, product_id)
        REFERENCES economy_product_definition(release_id, product_id)
        ON DELETE RESTRICT
);

CREATE INDEX building_product_stock_product_idx
    ON building_product_stock (town_id, product_id)
    WHERE quantity > 0;

COMMENT ON TABLE building_product_stock IS
    'Authoritative per-product stock owned by one concrete building instance.';
