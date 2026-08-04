# Core-game catalog SQL

This directory contains the deterministic SQL bundle for static `1.411` core-game catalogs that are not fully represented by the player-state migrations.

Run it from this directory with:

```sh
psql "$DATABASE_URL" -f init.sql
```

Docker development and production apply this exact bundle once through
`infra/db/run-migrations.sh`. The runner records a combined checksum under the
versioned `core_game_evil_hunter_rebuild_v1_weapon_core_v1` migration key and
fails closed if the published SQL changes in place.

The bundle creates/replaces the `core_game` schema and loads monster stats and
drops, material market links, unresolved recipe conditions, EXP rows, legacy
gear definitions, and the versioned rebuild weapon-core catalog. The rebuild
catalog includes 40 bilingual weapon bases, difficulty/rarity rules, all 125
mined gear properties, one rebuild flat-attack property, 160 affix tiers, 20
weighted weapon-pool rows, five Virtue effects, and all 61 collection-set rows.
Transformation acquisition and collection-set semantics remain explicitly
disabled.

It does not create player accounts, inventory ownership, wallets, orders, or
ledgers; those remain in `infra/db/migrations/`.

Regenerate after changing source catalogs:

```sh
python3 tools/generate-core-game-sql.py
```

The generated manifest records source SHA-256 values. Unresolved evidence is stored as JSONB and is never converted into guessed gameplay values.
