UPDATE player_hunter
SET action_state = 'hunting'
WHERE action_state IN (
    'entering_region',
    'returning_for_infirmary',
    'using_healing_potion'
);

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
        'dead',
        'traveling_to_enhancement_forge',
        'waiting_for_enhancement_interaction',
        'configuring_enhancement'
    ));
