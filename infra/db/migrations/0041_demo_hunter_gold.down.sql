DROP FUNCTION IF EXISTS seed_full_demo_account_stock(UUID);

ALTER FUNCTION seed_full_demo_account_stock_base(UUID)
    RENAME TO seed_full_demo_account_stock;
