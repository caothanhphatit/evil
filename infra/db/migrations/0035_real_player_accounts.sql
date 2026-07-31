ALTER TABLE local_identities
    DROP CONSTRAINT IF EXISTS local_identities_player_token_key;

CREATE INDEX IF NOT EXISTS local_identities_player_token_idx
    ON local_identities (player_token);

CREATE TABLE player_account (
    account_id UUID PRIMARY KEY,
    player_token UUID NOT NULL,
    normalized_email TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL CHECK (char_length(display_name) BETWEEN 2 AND 24),
    password_hash TEXT NOT NULL,
    is_demo BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX player_account_player_token_idx
    ON player_account (player_token);

-- Disposable demo credentials all point to the existing fully seeded Hunter Lab world.
-- Password for local development: Demo1234!
INSERT INTO player_account
    (account_id, player_token, normalized_email, display_name, password_hash, is_demo)
VALUES
    ('00000000-0000-4000-8000-00000000d001', '00000000-0000-4000-8000-00000000a001',
     'demo1@evil.local', 'Hunter Lab Demo 1',
     '$pbkdf2-sha256$20000$6576696c2d68756e7465722d64656d6f2d31$93f3141640069161239cf63bd5b771040720a2127f35e746fb7e6b04e7090283', TRUE),
    ('00000000-0000-4000-8000-00000000d002', '00000000-0000-4000-8000-00000000a001',
     'demo2@evil.local', 'Hunter Lab Demo 2',
     '$pbkdf2-sha256$20000$6576696c2d68756e7465722d64656d6f2d32$78212e3eb69324587f865e1b7893b53a98e137efb56190a7adad2412c0f9aab4', TRUE),
    ('00000000-0000-4000-8000-00000000d003', '00000000-0000-4000-8000-00000000a001',
     'demo3@evil.local', 'Hunter Lab Demo 3',
     '$pbkdf2-sha256$20000$6576696c2d68756e7465722d64656d6f2d33$f40e4839fd92edc7855164293d6b8a03232ad6cc496ace07b097ac76b0e99e83', TRUE)
ON CONFLICT (normalized_email) DO UPDATE
SET display_name = EXCLUDED.display_name,
    password_hash = EXCLUDED.password_hash,
    is_demo = TRUE,
    updated_at = now();
