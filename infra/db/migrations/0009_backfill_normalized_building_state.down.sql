-- Only rows projected by 0009 are removed. Towns created by the normalized
-- runtime after deployment are never treated as migration scratch data.
DELETE FROM building_normalization_issue
WHERE player_token IN (
    SELECT player_token FROM town WHERE legacy_backfilled_at IS NOT NULL
);
DELETE FROM town
WHERE legacy_backfilled_at IS NOT NULL;
