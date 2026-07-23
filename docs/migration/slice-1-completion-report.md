# Slice 1 Completion And Known Gaps

- Report date: 2026-07-23
- Content release: `slice-001-combat-v1`
- Protocol schema: version 2, `json-debug`
- Status: playable migration fixture implemented; production and legacy-compatibility acceptance remain conditional

## Scope Delivered

Slice 1 now supplies one server-authoritative vertical path: a hunter and `mon_a_01_1` move, fight, die, revive, create and collect a deterministic drop, equip the fixture item, improve authoritative damage, and restore durable state after reconnect. The browser renders server state with PixiJS and the official Spine 4.2 runtime, verifies the content release by SHA-256, and sends intents rather than damage, loot, currency, or revival outcomes.

PostgreSQL stores the encounter, inventory, equipment, reward ledger, command ledger, timers, ground drops, and RNG state. Docker Compose includes a dedicated forward migration service. The WebSocket contract is generated from one schema and uses bounded, versioned, sequenced envelopes.

This is a technical migration fixture. It does not establish that the chosen monster, item ID `2001`, stats, damage, cadence, drop, revival timing, map, or rates match the legacy game. Goldblin remains the primary evidence actor, while `mon_a_01_1` is the recorded substitution used because it has a verified attack animation family.

## Coverage Metrics

Metrics use separate denominators so copied files are not counted as migrated gameplay.

| Dimension | Result | Interpretation |
| --- | ---: | --- |
| Raw Unity source copy | 415/415 files (100%) | Byte-copy coverage recorded by the source verification inventory; not runtime migration coverage. |
| Exported derivative inventory | 9,359 files; 190,429,626 bytes | Indexed evidence available for classification and later slices. |
| Slice 1 published release | 19 files; 9,960,953 bytes | 0.203% of exported objects and 5.732% of exported bytes. |
| Slice 1 runtime-bound assets | 11 files; 9,544,506 bytes | Hunter, fixture monster, grave frames, and equipment icon; 8 additional published files remain unbound candidates. |
| Renderer animation mapping | 16/16 referenced clips resolve (100%) | Eight hunter and eight fixture-monster front/back or state clips used by the current projection. This is not total hunter-skeleton coverage. |
| Recovered legacy numerical rules | 0 verified bindings | No recovered HP, damage, cadence, RNG, drop, equipment, economy, or revival formula is claimed. A percentage is not meaningful until the legacy rule denominator is known. |
| Fixture behavior path | 7/7 stages implemented | Move, combat, death/revival, drop, pickup, equip/stat improvement, and durable reconnect restoration. |
| Recorded automated checks | 22/22 passing | 10 Rust, 9 web, and 3 asset-pipeline tests at report authoring time. |
| Historical fixture Docker journey | 1/1 recorded | The former v2 technical fixture passed its isolated journey. Its script is retained only as `smoke:fixture` evidence and does not validate protocol v4 or the original-flow runtime. |
| Functional viewport checks | 2/2 passing | Desktop and 390 x 844 mobile layouts render one Pixi canvas, expose combat/inventory controls, stay within viewport width, and report no console warnings/errors. Performance tiers are not yet measured. |

The raw copy is complete for the inventoried source package, but classification and browser-runnable coverage across the whole game are not complete. Slice 7, not Slice 1, owns the 100% approved-source runnable target.

## Authority And Integrity

- The server owns movement, target selection, attack cadence, damage, death, revival time, RNG, rewards, collection, inventory, equipment effects, and persistence.
- Manual `respawn_hunter` input is rejected; revival remains server-timed.
- Reward and command ledger keys are scoped by player plus operation/command ID, and pending valuable operations are retained until persistence succeeds.
- Durable reconnect includes active encounter state, positions, HP, deadlines, RNG state, drops, inventory, and equipment.
- The browser verifies release-manifest and asset hashes, but client integrity is presentation defense only and is not trusted as anti-cheat authority.

## Known Gaps And Production Blockers

### Identity And Horizontal Scale

Slice 1 uses one fixed local demo identity and has no production login, token issuance, authorization, rotation, or account recovery. Concurrent ownership is guarded only by an in-process active-player set, so the server is safe only as a single replica. A distributed Redis lease or equivalent fencing mechanism, durable ownership revision, and takeover/recovery tests are required before horizontal scaling.

### Protocol And Performance

The current protocol sends bounded JSON debug envelopes and periodic full snapshots at the simulation cadence. It also checkpoints full durable state rather than an optimized event/delta stream. Binary codecs, interest-managed deltas, backpressure policy, compatibility windows, bandwidth profiles, database write profiles, load tests, and soak tests remain production gates. See ADR-0004.

### Legacy Compatibility

The village scene hierarchy, render ordering, collision/navigation, and spawn coordinates have not been reconstructed. Goldblin combat, starter hunter composition, ordinary attack audio/effects, item bindings, balance tables, content IDs, formulas, and rates still require serialized or observed evidence. Candidate BladeDance frames and four audio clips remain deliberately unbound to avoid false legacy claims.

### Release Evidence

The component suites cover deterministic simulation, idempotency, persistence, reconnect state, animation selection, client ordering, manifest integrity, and tamper rejection. Clean-volume Docker migration, the real WebSocket reward/equip/reconnect path, and functional desktop/mobile browser checks passed on 2026-07-23. Automated visual regression, accessibility, telemetry dashboards, failure injection, backup/restore, and performance budgets are not yet accepted.

## Next Acceptance Work

1. Run the isolated Docker/WebSocket smoke journey in CI from a fresh project volume and retain its logs as release evidence.
2. Publish deterministic desktop/mobile image baselines and add automated visual-diff thresholds.
3. Add binary protocol generation, compatibility fixtures, deltas/backpressure, and measured network budgets.
4. Replace the demo identity and single-process ownership guard with authenticated sessions and fenced distributed zone ownership.
5. Recover the map, content tables, actor bindings, formulas, and presentation events before increasing legacy-compatibility coverage.

## Evidence Index

- `docs/migration/slice-1-legacy-dossier.md`
- `reverse-engineering/evidence/slice1-asset-candidates.json`
- `game-assets/asset-index.json`
- `game-assets/manifests/slice-001.selection.json`
- `game-assets/manifests/releases/slice-001-combat-v1.json`
- `packages/protocol/world-v1.schema.json`
- `infra/db/migrations/0002_world_state_and_ledgers.sql`
- `tools/smoke-world.mjs` (historical protocol-v2 fixture only)
