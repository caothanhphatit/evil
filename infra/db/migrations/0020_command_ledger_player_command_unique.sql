DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'command_ledger_player_command_key'
          AND conrelid = 'command_ledger'::regclass
    ) THEN
        ALTER TABLE command_ledger
            ADD CONSTRAINT command_ledger_player_command_key
            UNIQUE (player_token, command_id);
    END IF;
END $$;
