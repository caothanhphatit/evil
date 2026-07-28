ALTER TABLE player_hunter
    ADD COLUMN hunt_state JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE TABLE player_hunter_action_command (
    player_token UUID NOT NULL REFERENCES player_hunter_roster(player_token) ON DELETE CASCADE,
    command_id UUID NOT NULL,
    command_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_token, command_id)
);
