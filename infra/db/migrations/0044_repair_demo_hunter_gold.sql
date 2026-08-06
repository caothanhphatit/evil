UPDATE player_hunter hunter
SET gold = GREATEST(hunter.gold, 1000000000)
FROM player_account account
WHERE account.player_token = hunter.player_token
  AND account.is_demo = TRUE;
