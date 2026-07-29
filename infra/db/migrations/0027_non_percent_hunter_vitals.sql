-- Deterministic rebuild fixture correction: these are current/max pairs, not
-- percentage bars. Values remain fixture data until live constructor capture.
UPDATE player_hunter AS hunter
SET current_hp = fixture.current_hp,
    max_hp = fixture.max_hp,
    stamina_current = fixture.stamina_current,
    stamina_maximum = fixture.stamina_maximum,
    satiety_current = fixture.satiety_current,
    satiety_maximum = fixture.satiety_maximum,
    mood_current = fixture.mood_current,
    mood_maximum = fixture.mood_maximum
FROM (VALUES
    (1, 5804::BIGINT, 6037::BIGINT, 92::BIGINT, 107::BIGINT, 99::BIGINT, 118::BIGINT, 102::BIGINT, 114::BIGINT),
    (2, 5788, 6074, 104, 124, 115, 141, 127, 143),
    (3, 5372, 5711, 116, 141, 65, 98, 81, 101),
    (4, 5356, 5748, 67, 97, 81, 121, 106, 130),
    (5, 5740, 6185, 79, 114, 97, 144, 60, 88),
    (6, 5523, 6021, 91, 131, 83, 101, 85, 117),
    (7, 5507, 6058, 134, 148, 99, 124, 137, 146),
    (8, 5511, 5695, 85, 104, 115, 147, 91, 104)
) AS fixture(hunter_id, current_hp, max_hp, stamina_current, stamina_maximum, satiety_current, satiety_maximum, mood_current, mood_maximum)
WHERE hunter.player_token = '00000000-0000-4000-8000-00000000a001'
  AND hunter.hunter_id = fixture.hunter_id;

UPDATE player_profile
SET seed_key = 'hunter-lab:20260728-vitals-fixture',
    seed_version = GREATEST(seed_version, 4),
    updated_at = now()
WHERE player_token = '00000000-0000-4000-8000-00000000a001';
