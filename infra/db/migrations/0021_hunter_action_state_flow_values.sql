ALTER TABLE player_hunter
    DROP CONSTRAINT IF EXISTS player_hunter_action_state_check;

ALTER TABLE player_hunter
    ADD CONSTRAINT player_hunter_action_state_check
    CHECK (action_state IN (
        'idle',
        'walking',
        'serving',
        'waiting',
        'banished',
        'hunting',
        'returning',
        'dead'
    ));
