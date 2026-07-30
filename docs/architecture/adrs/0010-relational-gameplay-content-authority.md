# ADR 0010: Relational Gameplay Content Authority

## Status

Accepted

## Context

Several authoritative runtime paths still decoded packaged JSON or held large
content arrays in Rust. That made content changes require a server build,
duplicated data already present in migration evidence, and would force a future
admin panel to edit source code or unvalidated JSON blobs.

Gameplay formulas and state machines are executable behavior and belong in
Rust. Map geometry, progression thresholds, monster pools/stats/drops, material
ratings, consumable values, gear definitions and recipe bindings are content
objects and need versioning, referential integrity, provenance and rollback.

## Decision

PostgreSQL is the runtime source of truth for stable gameplay content objects.

- Every object belongs to an immutable `content_release`.
- Stable fields use normalized tables and composite foreign keys. JSONB is
  limited to explicitly unresolved evidence that cannot yet be modeled without
  guessing.
- `content_source_manifest` records the source path and SHA-256 used to publish
  generated release rows.
- Production startup loads the active pinned release and fails closed when a
  required catalog is absent, incomplete or conflicts with an already installed
  process catalog.
- Source JSON and reverse-engineering evidence remain immutable import inputs;
  production simulation does not parse them.
- Rust continues to own formulas, RNG streams, validation, transactions and
  state transitions. Database rows supply parameters and object relationships,
  never executable expressions.
- A future admin panel edits a draft release, validates it, records an audit
  event, and atomically promotes it. It must not mutate an active release in
  place.

Player-owned content follows the same boundary. Stacked products are stored in
`player_hunter_item_stack`; individually rolled gear is stored in
`player_hunter_gear_instance` with its own identity and catalog-release
reference. The legacy JSONB ownership field remains only as a read-only
compatibility path during migration and is cleared on authoritative save.

The initial relational runtime catalogs are:

- world maps, density rows and entry waypoints;
- Hunter progression definitions and per-revive EXP rows;
- monsters, ordinary map/difficulty pools and material drop slots;
- materials and Trading Post difficulty ratings;
- gear definitions, ratings, material requirements and recipe bindings;
- consumables, per-level values and recipe bindings.
- Hunter classes, rarities, personalities and basic-skill metadata through the
  existing normalized Hunter definition tables.

## Consequences

Content-only tuning can eventually be published without recompiling the server,
while active sessions remain pinned to a known release. Foreign keys prevent an
admin workflow from publishing orphaned recipes, items, pools or ratings.

Publishing tooling must generate deterministic migrations/import batches and
verify row counts, source hashes, references and release completeness. Rollback
means retiring/promoting releases or applying a reviewed compensating migration,
not editing active rows in place.

Small policy constants, recovered arithmetic constants and deterministic fixture
data may remain in code when they are behavior rather than managed content.
Test-only exact fixtures may decode immutable source catalogs, but production
paths may not silently fall back to them.
