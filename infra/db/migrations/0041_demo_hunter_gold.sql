ALTER FUNCTION seed_full_demo_account_stock(UUID)
    RENAME TO seed_full_demo_account_stock_base;

CREATE FUNCTION seed_full_demo_account_stock(target_player UUID)
RETURNS VOID
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM seed_full_demo_account_stock_base(target_player);

    UPDATE player_hunter
    SET gold = GREATEST(gold, 1000000000)
    WHERE player_token = target_player;
END;
$$;

UPDATE player_hunter hunter
SET gold = GREATEST(hunter.gold, 1000000000)
FROM player_account account
WHERE account.player_token = hunter.player_token
  AND account.is_demo = TRUE;
