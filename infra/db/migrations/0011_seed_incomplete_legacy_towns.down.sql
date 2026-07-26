DELETE FROM player_building
WHERE seeded_by = 'migration:0011-default-town-v2';

UPDATE town
SET source_template_id = NULL,
    seed_version = 0,
    next_building_sequence = 1,
    updated_at = now()
WHERE legacy_backfilled_at IS NOT NULL
  AND source_template_id = 'default-town-v2';
