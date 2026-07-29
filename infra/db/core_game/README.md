# Core-game catalog SQL

This directory contains the deterministic SQL bundle for static `1.411` core-game catalogs that are not fully represented by the player-state migrations.

Run it from this directory with:

```sh
psql "$DATABASE_URL" -f init.sql
```

The bundle creates/replaces the `core_game` schema and loads monster stats and drops, material market links, unresolved recipe conditions, EXP rows, and gear definitions. It does not create player accounts, inventory ownership, wallets, orders, or ledgers; those remain in `infra/db/migrations/`.

Regenerate after changing source catalogs:

```sh
python3 tools/generate-core-game-sql.py
```

The generated manifest records source SHA-256 values. Unresolved evidence is stored as JSONB and is never converted into guessed gameplay values.
