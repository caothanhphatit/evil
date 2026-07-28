UPDATE player_hunter
SET xp_to_next_level = NULL,
    dps_milli = NULL,
    critical_rate_bps = NULL,
    attack_speed_milli = NULL,
    evasion_rate_bps = NULL,
    awakening_current = NULL,
    awakening_maximum = NULL,
    reincarnation_current = NULL,
    reincarnation_maximum = NULL,
    is_locked = NULL,
    characteristic_release_id = NULL,
    characteristic_id = NULL,
    secret_points = NULL,
    updated_at = now()
WHERE player_token = '00000000-0000-4000-8000-00000000a001';

UPDATE player_profile
SET seed_key = 'hunter-lab:20260727', seed_version = 2, updated_at = now()
WHERE player_token = '00000000-0000-4000-8000-00000000a001'
  AND seed_key = 'hunter-lab:20260727-full-fixture'
  AND seed_version = 3;
