# ADR 0008: Relational Building Authority

## Status

Accepted

## Context

The first building migration kept player building state inside the
`player_world_state.state` JSONB aggregate and copied an embedded content
registry into mutable catalog tables during application startup. Session
construction also repaired saves, selected starter buildings by numeric ID,
and identified building functions through magic IDs.

That model cannot enforce that a skin belongs to a base building, cannot safely
address multiple instances of the same building, and makes application startup
an implicit content-import and data-migration path.

## Decision

PostgreSQL is authoritative for the building domain.

- Content is immutable and versioned by `content_release`.
- Base definitions, levels, costs, prerequisites, skins, capabilities, and town
  templates are separate relational records under a pinned release.
- A town pins one content release even when it contains no buildings.
- Player buildings use UUID instance IDs. Commands address instances, not base
  building IDs.
- The selected skin is nullable and protected by a composite foreign key that
  includes release and base building identity.
- Town inventory, purchase orders, production jobs, upgrade jobs, and ledgers
  are durable relational state owned by the town aggregate.
- A building command commits its town revision, costs, inventory changes,
  instance changes, ledger entries, and idempotent result in one transaction.
- Starter layouts are data in a versioned town template. Runtime code must not
  infer starters from ID ranges or asset availability.
- Content import is an explicit versioned migration/admin operation. Server
  startup only loads and validates an active release; it never truncates or
  rewrites catalog tables.
- Legacy JSONB is backfilled once into normalized rows. Invalid records are
  quarantined for inspection instead of being deleted, moved, or assigned a
  fallback skin during session construction.

## Consequences

The websocket and simulation layers may still use an in-memory town aggregate,
but repository load/save maps that aggregate to normalized rows. Building
fields are removed from the authoritative JSONB persistence path after the
backfill verification gate passes.

Construct, move, upgrade, use, and equip-skin protocol commands require an
instance ID. Price and capability decisions are server-owned and loaded from
the town's pinned content release.

The temporary `building_base_catalog` and `building_skin_catalog` tables remain
only as legacy import sources until the normalized release is verified. They
must not be queried by gameplay code.
