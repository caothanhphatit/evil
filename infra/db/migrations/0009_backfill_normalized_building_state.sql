-- Project the legacy JSONB snapshot only after 0008 has published all referenced
-- content. The source JSONB remains unchanged for reconciliation and rollback.
INSERT INTO town (
    player_token, release_id, gold, seed_version, next_building_sequence, revision,
    legacy_source_revision, legacy_backfilled_at, created_at, updated_at
)
SELECT
    player_token,
    'evil-hunter-1.411.buildings-v1',
    CASE
        WHEN state #>> '{buildings,town_gold}' ~ '^[0-9]{1,18}$'
            THEN (state #>> '{buildings,town_gold}')::BIGINT
        ELSE 0
    END,
    CASE
        WHEN state #>> '{buildings,town_seed_version}' ~ '^[0-9]{1,9}$'
            THEN (state #>> '{buildings,town_seed_version}')::INTEGER
        ELSE 0
    END,
    CASE
        WHEN state #>> '{buildings,next_building_instance_id}' ~ '^[1-9][0-9]{0,17}$'
            THEN (state #>> '{buildings,next_building_instance_id}')::BIGINT
        ELSE 1
    END,
    revision,
    revision,
    now(),
    created_at,
    updated_at
FROM player_world_state;

INSERT INTO town_economy_summary (
    town_id, hunter_materials, materials, runes, weapons, armor,
    hunter_equipment_purchases
)
SELECT
    town.town_id,
    CASE WHEN state #>> '{buildings,hunter_materials}' ~ '^[0-9]{1,18}$'
        THEN (state #>> '{buildings,hunter_materials}')::BIGINT ELSE 0 END,
    CASE WHEN state #>> '{buildings,materials}' ~ '^[0-9]{1,18}$'
        THEN (state #>> '{buildings,materials}')::BIGINT ELSE 0 END,
    CASE WHEN state #>> '{buildings,runes}' ~ '^[0-9]{1,18}$'
        THEN (state #>> '{buildings,runes}')::BIGINT ELSE 0 END,
    CASE WHEN state #>> '{buildings,weapons}' ~ '^[0-9]{1,18}$'
        THEN (state #>> '{buildings,weapons}')::BIGINT ELSE 0 END,
    CASE WHEN state #>> '{buildings,armor}' ~ '^[0-9]{1,18}$'
        THEN (state #>> '{buildings,armor}')::BIGINT ELSE 0 END,
    CASE WHEN state #>> '{buildings,hunter_equipment_purchases}' ~ '^[0-9]{1,18}$'
        THEN (state #>> '{buildings,hunter_equipment_purchases}')::BIGINT ELSE 0 END
FROM player_world_state AS state
JOIN town USING (player_token);

INSERT INTO town_trade_state (town_id, field_trip_id, settled_field_trip_id)
SELECT
    town.town_id,
    CASE WHEN state #>> '{buildings,field_trip_id}' ~ '^[0-9]{1,18}$'
        THEN (state #>> '{buildings,field_trip_id}')::BIGINT ELSE 0 END,
    CASE WHEN state #>> '{buildings,settled_field_trip_id}' ~ '^[0-9]{1,18}$'
        THEN (state #>> '{buildings,settled_field_trip_id}')::BIGINT ELSE 0 END
FROM player_world_state AS state
JOIN town USING (player_token);

INSERT INTO building_normalization_issue (
    player_token, issue_kind, source_pointer, source_payload
)
SELECT
    state.player_token,
    'duplicate_building_instance_id',
    '/buildings/buildings',
    jsonb_build_object('instance_id', row.value->>'instance_id', 'count', count(*))
FROM player_world_state AS state
CROSS JOIN LATERAL jsonb_array_elements(
    CASE WHEN jsonb_typeof(state.state #> '{buildings,buildings}') = 'array'
        THEN state.state #> '{buildings,buildings}' ELSE '[]'::jsonb END
) AS row(value)
WHERE NULLIF(btrim(row.value->>'instance_id'), '') IS NOT NULL
GROUP BY state.player_token, row.value->>'instance_id'
HAVING count(*) > 1;

WITH serialized_instance AS (
    SELECT
        state.player_token,
        row.value,
        row.ordinality,
        NULLIF(btrim(row.value->>'instance_id'), '') AS legacy_instance_id,
        NULLIF(btrim(row.value->>'id'), '') AS building_id,
        CASE WHEN row.value->>'level' ~ '^[1-9][0-9]{0,8}$'
            THEN (row.value->>'level')::INTEGER END AS current_level,
        CASE WHEN row.value->>'equipped_skin_id' ~ '^[1-9][0-9]{0,17}$'
            THEN (row.value->>'equipped_skin_id')::BIGINT END AS equipped_skin_id,
        CASE WHEN row.value->>'grid_x' ~ '^-?[0-9]{1,9}$'
            THEN (row.value->>'grid_x')::INTEGER END AS grid_x,
        CASE WHEN row.value->>'grid_y' ~ '^-?[0-9]{1,9}$'
            THEN (row.value->>'grid_y')::INTEGER END AS grid_y,
        CASE WHEN row.value->>'uses' ~ '^[0-9]{1,18}$'
            THEN (row.value->>'uses')::BIGINT ELSE 0 END AS use_count,
        NULLIF(btrim(row.value->>'seeded_by'), '') AS seeded_by,
        row_number() OVER (
            PARTITION BY state.player_token, row.value->>'instance_id'
            ORDER BY row.ordinality
        ) AS duplicate_rank
    FROM player_world_state AS state
    CROSS JOIN LATERAL jsonb_array_elements(
        CASE WHEN jsonb_typeof(state.state #> '{buildings,buildings}') = 'array'
            THEN state.state #> '{buildings,buildings}' ELSE '[]'::jsonb END
    ) WITH ORDINALITY AS row(value, ordinality)
), resolved_instance AS (
    SELECT
        serialized.*,
        skin.skin_id AS valid_skin_id
    FROM serialized_instance AS serialized
    JOIN building_level_definition AS level
      ON level.release_id = 'evil-hunter-1.411.buildings-v1'
     AND level.building_id = serialized.building_id
     AND level.level = serialized.current_level
    LEFT JOIN building_skin_definition AS skin
      ON skin.release_id = level.release_id
     AND skin.building_id = level.building_id
     AND skin.skin_id = serialized.equipped_skin_id
    WHERE serialized.duplicate_rank = 1
      AND serialized.legacy_instance_id IS NOT NULL
      AND serialized.grid_x IS NOT NULL
      AND serialized.grid_y IS NOT NULL
)
INSERT INTO player_building (
    town_id, release_id, legacy_instance_id, building_id, current_level,
    equipped_skin_id, grid_x, grid_y, use_count, seeded_by
)
SELECT
    town.town_id, town.release_id, instance.legacy_instance_id,
    instance.building_id, instance.current_level, instance.valid_skin_id,
    instance.grid_x, instance.grid_y, instance.use_count, instance.seeded_by
FROM resolved_instance AS instance
JOIN town USING (player_token);

-- An explicit but invalid skin is recorded and normalized to NULL. No skin is
-- inferred from building level, asset names, family, or visual availability.
INSERT INTO building_normalization_issue (
    player_token, issue_kind, source_pointer, source_payload
)
SELECT
    state.player_token,
    'equipped_skin_not_in_release',
    '/buildings/buildings/' || (row.ordinality - 1)::TEXT || '/equipped_skin_id',
    row.value
FROM player_world_state AS state
CROSS JOIN LATERAL jsonb_array_elements(
    CASE WHEN jsonb_typeof(state.state #> '{buildings,buildings}') = 'array'
        THEN state.state #> '{buildings,buildings}' ELSE '[]'::jsonb END
) WITH ORDINALITY AS row(value, ordinality)
WHERE row.value->>'equipped_skin_id' ~ '^[1-9][0-9]{0,17}$'
  AND NOT EXISTS (
      SELECT 1
      FROM building_skin_definition AS skin
      WHERE skin.release_id = 'evil-hunter-1.411.buildings-v1'
        AND skin.building_id = row.value->>'id'
        AND skin.skin_id = (row.value->>'equipped_skin_id')::BIGINT
  );

INSERT INTO building_normalization_issue (
    player_token, issue_kind, source_pointer, source_payload
)
SELECT
    state.player_token,
    'building_instance_not_backfilled',
    '/buildings/buildings/' || (row.ordinality - 1)::TEXT,
    row.value
FROM player_world_state AS state
CROSS JOIN LATERAL jsonb_array_elements(
    CASE WHEN jsonb_typeof(state.state #> '{buildings,buildings}') = 'array'
        THEN state.state #> '{buildings,buildings}' ELSE '[]'::jsonb END
) WITH ORDINALITY AS row(value, ordinality)
WHERE NOT EXISTS (
    SELECT 1
    FROM town
    JOIN player_building ON player_building.town_id = town.town_id
    WHERE town.player_token = state.player_token
      AND player_building.legacy_instance_id = NULLIF(btrim(row.value->>'instance_id'), '')
);

WITH material_row AS (
    SELECT
        state.player_token,
        row.ordinality,
        NULLIF(btrim(row.value->>'id'), '') AS material_id,
        CASE WHEN row.value->>'town_quantity' ~ '^[0-9]{1,18}$'
            THEN (row.value->>'town_quantity')::BIGINT END AS town_quantity,
        CASE WHEN row.value->>'requested' ~ '^[1-9][0-9]{0,17}$'
            THEN (row.value->>'requested')::BIGINT END AS requested_quantity,
        CASE WHEN row.value->>'unit_price' ~ '^[0-9]{1,18}$'
            THEN (row.value->>'unit_price')::BIGINT END AS unit_price,
        row_number() OVER (
            PARTITION BY state.player_token, row.value->>'id'
            ORDER BY row.ordinality
        ) AS duplicate_rank
    FROM player_world_state AS state
    CROSS JOIN LATERAL jsonb_array_elements(
        CASE WHEN jsonb_typeof(state.state #> '{buildings,material_stocks}') = 'array'
            THEN state.state #> '{buildings,material_stocks}' ELSE '[]'::jsonb END
    ) WITH ORDINALITY AS row(value, ordinality)
)
INSERT INTO town_inventory_stack (town_id, item_id, quantity)
SELECT town.town_id, material.material_id, material.town_quantity
FROM material_row AS material
JOIN town USING (player_token)
WHERE material.duplicate_rank = 1
  AND material.material_id IS NOT NULL
  AND material.town_quantity IS NOT NULL;

WITH material_row AS (
    SELECT
        state.player_token,
        NULLIF(btrim(row.value->>'id'), '') AS material_id,
        CASE WHEN row.value->>'hunter_quantity' ~ '^[0-9]{1,18}$'
            THEN (row.value->>'hunter_quantity')::BIGINT END AS hunter_quantity,
        row_number() OVER (
            PARTITION BY state.player_token, row.value->>'id'
            ORDER BY row.ordinality
        ) AS duplicate_rank
    FROM player_world_state AS state
    CROSS JOIN LATERAL jsonb_array_elements(
        CASE WHEN jsonb_typeof(state.state #> '{buildings,material_stocks}') = 'array'
            THEN state.state #> '{buildings,material_stocks}' ELSE '[]'::jsonb END
    ) WITH ORDINALITY AS row(value, ordinality)
)
INSERT INTO hunter_material_stack (town_id, material_id, quantity)
SELECT town.town_id, material.material_id, material.hunter_quantity
FROM material_row AS material
JOIN town USING (player_token)
WHERE material.duplicate_rank = 1
  AND material.material_id IS NOT NULL
  AND material.hunter_quantity IS NOT NULL;

INSERT INTO town_inventory_ledger (
    town_id, item_id, quantity_delta, balance_after, reason, operation_id
)
SELECT town_id, item_id, quantity, quantity, 'legacy_jsonb_backfill', gen_random_uuid()
FROM town_inventory_stack
WHERE quantity > 0;

WITH material_row AS (
    SELECT
        state.player_token,
        NULLIF(btrim(row.value->>'id'), '') AS material_id,
        CASE WHEN row.value->>'requested' ~ '^[1-9][0-9]{0,17}$'
            THEN (row.value->>'requested')::BIGINT END AS requested_quantity,
        CASE WHEN row.value->>'unit_price' ~ '^[0-9]{1,18}$'
            THEN (row.value->>'unit_price')::BIGINT END AS unit_price,
        row_number() OVER (
            PARTITION BY state.player_token, row.value->>'id'
            ORDER BY row.ordinality
        ) AS duplicate_rank
    FROM player_world_state AS state
    CROSS JOIN LATERAL jsonb_array_elements(
        CASE WHEN jsonb_typeof(state.state #> '{buildings,material_stocks}') = 'array'
            THEN state.state #> '{buildings,material_stocks}' ELSE '[]'::jsonb END
    ) WITH ORDINALITY AS row(value, ordinality)
)
INSERT INTO building_material_order (
    town_id, material_id, requested_quantity, unit_price
)
SELECT town.town_id, material.material_id,
       material.requested_quantity, material.unit_price
FROM material_row AS material
JOIN town USING (player_token)
WHERE material.duplicate_rank = 1
  AND material.material_id IS NOT NULL
  AND material.requested_quantity IS NOT NULL
  AND material.unit_price IS NOT NULL;

WITH settlement_row AS (
    SELECT
        state.player_token,
        NULLIF(btrim(row.value->>'settlement_id'), '') AS settlement_id,
        CASE WHEN row.value->>'field_trip_id' ~ '^[1-9][0-9]{0,17}$'
            THEN (row.value->>'field_trip_id')::BIGINT END AS field_trip_id,
        NULLIF(btrim(row.value->>'material_id'), '') AS material_id,
        CASE WHEN row.value->>'quantity' ~ '^[1-9][0-9]{0,17}$'
            THEN (row.value->>'quantity')::BIGINT END AS quantity,
        CASE WHEN row.value->>'unit_price' ~ '^[0-9]{1,18}$'
            THEN (row.value->>'unit_price')::BIGINT END AS unit_price,
        CASE WHEN row.value->>'total_gold' ~ '^[0-9]{1,18}$'
            THEN (row.value->>'total_gold')::BIGINT END AS total_gold,
        row_number() OVER (
            PARTITION BY state.player_token, row.value->>'settlement_id'
            ORDER BY row.ordinality
        ) AS duplicate_rank
    FROM player_world_state AS state
    CROSS JOIN LATERAL jsonb_array_elements(
        CASE WHEN jsonb_typeof(state.state #> '{buildings,trade_settlements}') = 'array'
            THEN state.state #> '{buildings,trade_settlements}' ELSE '[]'::jsonb END
    ) WITH ORDINALITY AS row(value, ordinality)
)
INSERT INTO hunter_trade_settlement (
    town_id, settlement_id, field_trip_id, material_id,
    quantity, unit_price, total_gold
)
SELECT
    town.town_id, settlement.settlement_id, settlement.field_trip_id,
    settlement.material_id, settlement.quantity, settlement.unit_price,
    settlement.total_gold
FROM settlement_row AS settlement
JOIN town USING (player_token)
WHERE settlement.duplicate_rank = 1
  AND settlement.settlement_id IS NOT NULL
  AND settlement.field_trip_id IS NOT NULL
  AND settlement.material_id IS NOT NULL
  AND settlement.quantity IS NOT NULL
  AND settlement.unit_price IS NOT NULL
  AND settlement.total_gold IS NOT NULL;
