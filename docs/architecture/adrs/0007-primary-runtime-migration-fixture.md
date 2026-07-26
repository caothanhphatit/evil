# ADR-0007: Primary Runtime Migration Fixture

- Status: Accepted for migration validation
- Date: 2026-07-23

## Context

The deterministic Slice 1 combat simulation previously existed only as a unit-test fixture while the primary WebSocket runtime exposed navigation and presentation-only roaming. That split prevented browser-to-server validation of authoritative ticks, reconnect recovery, reward persistence, and command idempotency.

The recovered assets and static evidence do not yet prove the original game's combat formulas, balance values, starter equipment, or first-monster rules. Runtime integration must therefore remain useful without presenting fixture constants as reconstructed legacy behavior.

## Decision

Protocol v6 exposes the deterministic simulation under the explicit content ID `migration-fixture.slice1-combat-v1` and evidence label `deterministic_migration_fixture_not_legacy_balance`. The existing legacy field gate remains unresolved; `field.gameplay_runnable` stays false. The separate fixture projection is active only while the player is on the field screen.

The server advances the fixture at `SIMULATION_TICK_RATE`, owns movement, targeting, damage, RNG, death, revival, drops, collection, gold, inventory, and equipment effects, and emits authoritative snapshots and events. The browser may submit equipment intent but never damage, rewards, random outcomes, or revival completion.

Durable player state is a versioned aggregate containing navigation and fixture combat state, including RNG and bounded command outcome data required for deterministic reconnect. PostgreSQL checkpoints aggregate state and fixture reward/command ledger entries in one transaction. Reward operation IDs and equipment correlation IDs are idempotency keys.

## Consequences

- The browser can exercise a real authoritative loop while legacy evidence work continues.
- Reconnect restores combat position, health, timers, drops, inventory, equipment, RNG, and prior equipment command outcomes.
- Fixture constants, IDs, and balance must never be relabeled as confirmed original-game behavior.
- Promotion to reconstructed gameplay requires a new evidence-backed content release and replacement of the fixture contract rather than silently changing its meaning.
