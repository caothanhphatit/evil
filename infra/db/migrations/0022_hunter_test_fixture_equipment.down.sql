UPDATE player_profile
SET seed_key = 'hunter-lab:20260724', seed_version = 1, updated_at = now()
WHERE player_token = '00000000-0000-4000-8000-00000000a001'
  AND seed_key = 'hunter-lab:20260727'
  AND seed_version = 2;

DROP TABLE IF EXISTS player_hunter_fixture_equipment;
