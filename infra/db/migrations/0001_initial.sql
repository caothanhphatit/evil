CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    external_subject TEXT UNIQUE,
    display_name TEXT NOT NULL CHECK (char_length(display_name) BETWEEN 1 AND 32),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'deleted')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE player_profiles (
    account_id UUID PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0),
    gold BIGINT NOT NULL DEFAULT 0 CHECK (gold >= 0),
    progression JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE game_sessions (
    id UUID PRIMARY KEY,
    account_id UUID REFERENCES accounts(id) ON DELETE SET NULL,
    simulation_seed BIGINT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ended_at TIMESTAMPTZ
);

CREATE INDEX game_sessions_account_started_idx
    ON game_sessions (account_id, started_at DESC);

CREATE TABLE game_events (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES game_sessions(id) ON DELETE CASCADE,
    server_tick BIGINT NOT NULL CHECK (server_tick >= 0),
    event_sequence SMALLINT NOT NULL CHECK (event_sequence >= 0),
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (session_id, server_tick, event_sequence)
);

CREATE INDEX game_events_session_tick_idx
    ON game_events (session_id, server_tick);
