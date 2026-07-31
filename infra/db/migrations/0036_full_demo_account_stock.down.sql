DELETE FROM player_hunter_item_stack
WHERE player_token = '00000000-0000-4000-8000-00000000a001'
  AND quantity = 999;

DELETE FROM crafted_gear_stock
WHERE ruleset = 'demo_full_stock_v1';
