ALTER TABLE player_hunter
    ADD COLUMN owned_items JSONB NOT NULL DEFAULT '[]'::jsonb;
