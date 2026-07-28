-- Operational Hunter Lab values used to exercise every rebuild-owned Status
-- field. These are deterministic test fixtures, not captured original values.
WITH fixture(
    hunter_id, xp_to_next_level, dps_milli, critical_rate_bps,
    attack_speed_milli, evasion_rate_bps, awakening_current,
    reincarnation_current, is_locked, characteristic_id, secret_points
) AS (
    VALUES
        (1, 212::BIGINT, 119840::BIGINT, 1460, 1473, 760, 1, 1, FALSE, 'characteristic:22', 3),
        (2, 254::BIGINT, 128073::BIGINT, 1075, 1291, 687, 0, 2, FALSE, 'characteristic:30', 1),
        (3, 467::BIGINT,  74166::BIGINT, 1101, 2086, 495, 0, 1, FALSE, 'characteristic:31', 2),
        (4, 220::BIGINT,  95429::BIGINT,  493, 2684, 944, 2, 2, TRUE,  'characteristic:5',  0),
        (5, 241::BIGINT, 151437::BIGINT,  509, 1403, 738, 1, 1, FALSE, 'characteristic:26', 4),
        (6, 318::BIGINT,  71050::BIGINT, 1395, 1209, 506, 1, 2, FALSE, 'characteristic:26', 2),
        (7, 200::BIGINT, 107723::BIGINT,  338, 2377, 293, 0, 2, FALSE, 'characteristic:0',  1),
        (8, 504::BIGINT,  82686::BIGINT,  697, 1204, 533, 1, 2, TRUE,  'characteristic:5',  5)
)
UPDATE player_hunter AS hunter
SET xp_to_next_level = fixture.xp_to_next_level,
    dps_milli = fixture.dps_milli,
    critical_rate_bps = fixture.critical_rate_bps,
    attack_speed_milli = fixture.attack_speed_milli,
    evasion_rate_bps = fixture.evasion_rate_bps,
    awakening_current = fixture.awakening_current,
    awakening_maximum = 4,
    reincarnation_current = fixture.reincarnation_current,
    reincarnation_maximum = 5,
    is_locked = fixture.is_locked,
    characteristic_release_id = 'evil-hunter-1.411.hunter-info-v1',
    characteristic_id = fixture.characteristic_id,
    secret_points = fixture.secret_points
FROM fixture
WHERE hunter.player_token = '00000000-0000-4000-8000-00000000a001'
  AND hunter.hunter_id = fixture.hunter_id;

-- Keep runtime_evidence/source_* columns nullable. No live Hunter value was
-- captured for this shared disposable account.
UPDATE player_profile
SET seed_key = 'hunter-lab:20260727-full-fixture',
    seed_version = GREATEST(seed_version, 3),
    updated_at = now()
WHERE player_token = '00000000-0000-4000-8000-00000000a001';
