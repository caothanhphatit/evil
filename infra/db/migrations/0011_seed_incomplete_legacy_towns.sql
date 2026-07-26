-- Legacy production snapshots predate the complete 28-building town seed. Fill
-- only missing core bases for backfilled, pre-v2 towns and mark every inserted
-- row so the repair is explicit and reversible. The legacy JSONB is untouched.
INSERT INTO player_building (
    town_id, release_id, building_id, current_level, equipped_skin_id,
    grid_x, grid_y, use_count, seeded_by
)
SELECT
    town.town_id,
    template.release_id,
    template.building_id,
    template.level,
    template.equipped_skin_id,
    template.grid_x,
    template.grid_y,
    0,
    'migration:0011-default-town-v2'
FROM town
JOIN town_template_building AS template
  ON template.template_id = 'default-town-v2'
 AND template.release_id = town.release_id
WHERE town.legacy_backfilled_at IS NOT NULL
  AND town.seed_version < 2
  AND NOT EXISTS (
      SELECT 1
      FROM player_building AS existing
      WHERE existing.town_id = town.town_id
        AND existing.building_id = template.building_id
  );

UPDATE town
SET source_template_id = 'default-town-v2',
    seed_version = 2,
    next_building_sequence = GREATEST(next_building_sequence, 29),
    updated_at = now()
WHERE legacy_backfilled_at IS NOT NULL
  AND seed_version < 2;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM town
        WHERE legacy_backfilled_at IS NOT NULL
          AND seed_version = 2
          AND (
              SELECT count(*)
              FROM player_building
              WHERE player_building.town_id = town.town_id
                AND player_building.building_id ~ '^build_([1-9]|1[0-9]|2[0-8])$'
          ) <> 28
    ) THEN
        RAISE EXCEPTION 'legacy default town repair did not produce 28 core buildings';
    END IF;
END
$$;
