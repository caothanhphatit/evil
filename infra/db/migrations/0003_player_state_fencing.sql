ALTER TABLE player_world_state
    ADD COLUMN lease_fence BIGINT NOT NULL DEFAULT 0 CHECK (lease_fence >= 0);
