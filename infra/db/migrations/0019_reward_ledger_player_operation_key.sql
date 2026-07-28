-- Align the legacy local schema with the server's per-player reward idempotency key.
ALTER TABLE reward_ledger
    DROP CONSTRAINT IF EXISTS reward_ledger_pkey;

ALTER TABLE reward_ledger
    ADD CONSTRAINT reward_ledger_pkey PRIMARY KEY (player_token, operation_id);
