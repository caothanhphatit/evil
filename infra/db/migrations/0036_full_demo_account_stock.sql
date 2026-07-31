-- Keep the disposable Hunter Lab account fully stocked for end-to-end testing.
WITH demo_town AS (
    SELECT town_id, release_id
    FROM town
    WHERE player_token = '00000000-0000-4000-8000-00000000a001'
)
UPDATE town
SET gold = 100000000, updated_at = now()
WHERE town_id IN (SELECT town_id FROM demo_town);

UPDATE player_hunter
SET gold = 1000000
WHERE player_token = '00000000-0000-4000-8000-00000000a001';

INSERT INTO town_inventory_stack (town_id, item_id, quantity)
SELECT town.town_id, resource.resource_id, 999999
FROM town
JOIN economy_resource_definition resource ON resource.release_id = town.release_id
WHERE town.player_token = '00000000-0000-4000-8000-00000000a001'
  AND resource.resource_kind <> 'currency'
ON CONFLICT (town_id, item_id) DO UPDATE
SET quantity = GREATEST(town_inventory_stack.quantity, EXCLUDED.quantity),
    updated_at = now();

WITH routed_product AS (
    SELECT product.release_id,
           product.product_id,
           CASE
               WHEN product.product_id LIKE 'recipe:weapon:%' THEN 'build_7'
               WHEN product.product_id ~ '^recipe:(armor|gloves|boots):' THEN 'build_8'
               WHEN product.product_id ~ '^recipe:(necklace|ring|belt):' THEN 'build_20'
               WHEN product.product_id LIKE 'recipe:consumable:%' THEN 'build_11'
               ELSE product.building_id
           END AS stock_building_id
    FROM economy_product_definition product
), target_stock AS (
    SELECT town.town_id, building.instance_id, town.release_id,
           building.building_id, product.product_id
    FROM town
    JOIN routed_product product ON product.release_id = town.release_id
    JOIN LATERAL (
        SELECT candidate.instance_id, candidate.building_id
        FROM player_building candidate
        WHERE candidate.town_id = town.town_id
          AND candidate.building_id = product.stock_building_id
        ORDER BY candidate.instance_id
        LIMIT 1
    ) building ON TRUE
    WHERE town.player_token = '00000000-0000-4000-8000-00000000a001'
)
INSERT INTO building_product_stock
    (town_id, building_instance_id, release_id, building_id, product_id, quantity)
SELECT town_id, instance_id, release_id, building_id, product_id, 9999
FROM target_stock
ON CONFLICT (town_id, building_instance_id, product_id) DO UPDATE
SET quantity = GREATEST(building_product_stock.quantity, EXCLUDED.quantity),
    updated_at = now();

DELETE FROM crafted_gear_stock
WHERE ruleset = 'demo_full_stock_v1';

WITH gear_product AS (
    SELECT product.release_id,
           product.product_id,
           split_part(product.product_id, ':', 2) AS gear_kind,
           split_part(product.product_id, ':', 3)::INTEGER AS gear_index,
           split_part(product.product_id, ':', 5)::SMALLINT AS rating,
           CASE
               WHEN product.product_id LIKE 'recipe:weapon:%' THEN 'build_7'
               WHEN product.product_id ~ '^recipe:(armor|gloves|boots):' THEN 'build_8'
               ELSE 'build_20'
           END AS stock_building_id
    FROM economy_product_definition product
    WHERE product.product_id ~ '^recipe:(weapon|armor|gloves|boots|necklace|ring|belt):[0-9]+:rating:[0-4]$'
), target_gear AS (
    SELECT town.town_id, building.instance_id, product.*
    FROM town
    JOIN gear_product product ON product.release_id = town.release_id
    JOIN LATERAL (
        SELECT candidate.instance_id
        FROM player_building candidate
        WHERE candidate.town_id = town.town_id
          AND candidate.building_id = product.stock_building_id
        ORDER BY candidate.instance_id
        LIMIT 1
    ) building ON TRUE
    WHERE town.player_token = '00000000-0000-4000-8000-00000000a001'
)
INSERT INTO crafted_gear_stock
    (town_id, gear_instance_id, building_instance_id, product_id, gear_kind,
     rating, quality, primary_stat, option_type, option_value, icon_path, ruleset)
SELECT town_id, gen_random_uuid(), instance_id, product_id, gear_kind,
       rating, rating, 1000 + gear_index * 10 + rating, 0, 100,
       '/content/releases/evil-hunter-1.411/gear-icons/' || gear_kind || '-' || gear_index || '.png',
       'demo_full_stock_v1'
FROM target_gear
ON CONFLICT (town_id, gear_instance_id) DO NOTHING;

INSERT INTO player_hunter_item_stack
    (player_token, hunter_id, content_release_id, product_id, quantity)
SELECT '00000000-0000-4000-8000-00000000a001'::UUID,
       1, product.release_id, product.product_id, 999
FROM economy_product_definition product
JOIN player_hunter hunter
  ON hunter.player_token = '00000000-0000-4000-8000-00000000a001'
 AND hunter.hunter_id = 1
WHERE product.release_id = 'evil-hunter-1.411.buildings-v1'
ON CONFLICT (player_token, hunter_id, content_release_id, product_id) DO UPDATE
SET quantity = GREATEST(player_hunter_item_stack.quantity, EXCLUDED.quantity);
