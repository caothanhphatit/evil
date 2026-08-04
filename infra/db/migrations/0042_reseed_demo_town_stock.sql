DO $$
DECLARE
    target_player UUID;
BEGIN
    FOR target_player IN
        SELECT account.player_token
        FROM player_account account
        JOIN town ON town.player_token = account.player_token
        WHERE account.is_demo = TRUE
    LOOP
        PERFORM seed_full_demo_account_stock(target_player);
    END LOOP;
END;
$$;
