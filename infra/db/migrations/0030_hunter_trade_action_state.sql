-- A Hunter dispatched to the Trading Post remains a durable server-owned task
-- until the authoritative world agent reaches the building interaction point.
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
        'configuring_enhancement',
        'entering_region',
        'returning_for_infirmary',
        'using_healing_potion',
        'returning_to_trade'
    ));
