CREATE TABLE local_identities (
    token_hash BYTEA PRIMARY KEY CHECK (octet_length(token_hash) = 32),
    player_token UUID NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX local_identities_last_seen_idx
    ON local_identities (last_seen_at DESC);
