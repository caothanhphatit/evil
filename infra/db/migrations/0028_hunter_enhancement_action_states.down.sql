UPDATE player_hunter
SET action_state = 'idle'
WHERE action_state IN (
    'traveling_to_enhancement_forge',
    'waiting_for_enhancement_interaction',
    'configuring_enhancement'
);

ALTER TABLE player_hunter
    DROP CONSTRAINT IF EXISTS player_hunter_action_state_check;

ALTER TABLE player_hunter
    ADD CONSTRAINT player_hunter_action_state_check
    CHECK (action_state IN ('idle', 'walking', 'serving', 'waiting', 'banished'));
