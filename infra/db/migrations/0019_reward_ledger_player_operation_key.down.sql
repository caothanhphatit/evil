ALTER TABLE reward_ledger
    DROP CONSTRAINT IF EXISTS reward_ledger_pkey;

ALTER TABLE reward_ledger
    ADD CONSTRAINT reward_ledger_pkey PRIMARY KEY (operation_id);
