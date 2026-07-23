CREATE TABLE player_world_state (
    player_token UUID PRIMARY KEY,
    revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0),
    state JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE reward_ledger (
    operation_id UUID NOT NULL,
    player_token UUID NOT NULL REFERENCES player_world_state(player_token) ON DELETE CASCADE,
    reason TEXT NOT NULL,
    gold_delta BIGINT NOT NULL CHECK (gold_delta >= 0),
    item_id BIGINT NOT NULL CHECK (item_id > 0),
    quantity BIGINT NOT NULL CHECK (quantity > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_token, operation_id)
);

CREATE INDEX reward_ledger_player_created_idx
    ON reward_ledger (player_token, created_at DESC);

CREATE TABLE command_ledger (
    command_id UUID NOT NULL,
    player_token UUID NOT NULL REFERENCES player_world_state(player_token) ON DELETE CASCADE,
    command_type TEXT NOT NULL,
    result JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_token, command_id)
);

CREATE INDEX command_ledger_player_created_idx
    ON command_ledger (player_token, created_at DESC);
