DROP TABLE IF EXISTS player_account;
DROP INDEX IF EXISTS local_identities_player_token_idx;

DELETE FROM local_identities duplicate
USING local_identities retained
WHERE duplicate.player_token = retained.player_token
  AND duplicate.created_at > retained.created_at;

ALTER TABLE local_identities
    ADD CONSTRAINT local_identities_player_token_key UNIQUE (player_token);
