UPDATE player_account
SET player_token = '00000000-0000-4000-8000-00000000a001',
    updated_at = now()
WHERE normalized_email IN ('demo2@evil.local', 'demo3@evil.local');

DROP FUNCTION IF EXISTS seed_full_demo_account_stock(UUID);
