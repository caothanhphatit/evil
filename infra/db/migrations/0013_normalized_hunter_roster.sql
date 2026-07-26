CREATE TABLE player_hunter_roster (
    player_token UUID PRIMARY KEY REFERENCES player_world_state(player_token) ON DELETE CASCADE,
    roster_resolved BOOLEAN NOT NULL DEFAULT FALSE,
    wallets_resolved BOOLEAN NOT NULL DEFAULT FALSE,
    next_arrival_sequence BIGINT NOT NULL DEFAULT 1 CHECK (next_arrival_sequence >= 1),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE player_hunter (
    player_token UUID NOT NULL REFERENCES player_hunter_roster(player_token) ON DELETE CASCADE,
    hunter_id BIGINT NOT NULL CHECK (hunter_id > 0 AND hunter_id <= 4294967295),
    roster_state TEXT NOT NULL CHECK (roster_state IN ('active', 'waiting')),
    roster_position INTEGER NOT NULL CHECK (roster_position >= 0),
    arrival_sequence BIGINT,
    gold BIGINT NOT NULL CHECK (gold >= 0),
    current_hp BIGINT NOT NULL CHECK (current_hp >= 0),
    max_hp BIGINT NOT NULL CHECK (max_hp >= 0 AND current_hp <= max_hp),
    stamina_current BIGINT NOT NULL CHECK (stamina_current >= 0),
    stamina_maximum BIGINT NOT NULL CHECK (stamina_maximum >= 0 AND stamina_current <= stamina_maximum),
    satiety_current BIGINT NOT NULL CHECK (satiety_current >= 0),
    satiety_maximum BIGINT NOT NULL CHECK (satiety_maximum >= 0 AND satiety_current <= satiety_maximum),
    mood_current BIGINT NOT NULL CHECK (mood_current >= 0),
    mood_maximum BIGINT NOT NULL CHECK (mood_maximum >= 0 AND mood_current <= mood_maximum),
    PRIMARY KEY (player_token, hunter_id),
    CHECK (
        (roster_state = 'active' AND roster_position < 8 AND arrival_sequence IS NULL)
        OR (roster_state = 'waiting' AND arrival_sequence IS NOT NULL AND arrival_sequence >= 1)
    )
);

CREATE UNIQUE INDEX player_hunter_active_slot_unique
    ON player_hunter (player_token, roster_position)
    WHERE roster_state = 'active';

CREATE UNIQUE INDEX player_hunter_waiting_position_unique
    ON player_hunter (player_token, roster_position)
    WHERE roster_state = 'waiting';

CREATE UNIQUE INDEX player_hunter_waiting_sequence_unique
    ON player_hunter (player_token, arrival_sequence)
    WHERE roster_state = 'waiting';

CREATE TABLE player_hunter_roster_command (
    player_token UUID NOT NULL REFERENCES player_hunter_roster(player_token) ON DELETE CASCADE,
    command_id UUID NOT NULL,
    banished_hunter_id BIGINT NOT NULL CHECK (banished_hunter_id > 0 AND banished_hunter_id <= 4294967295),
    promoted_hunter_id BIGINT CHECK (promoted_hunter_id > 0 AND promoted_hunter_id <= 4294967295),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_token, command_id)
);

-- Legacy JSONB rosters are loaded as a compatibility fallback and normalized on
-- their next authoritative save; the migration intentionally does not guess at
-- malformed or over-capacity historical snapshots.
