# Testing Strategy

## Test Pyramid

- Unit/property tests: formulas, state machines, codecs, validators, interpolation, asset transforms.
- Domain integration tests: PostgreSQL/Redis repositories, transactions, idempotency, migrations, reconnect flows.
- Contract tests: generated Rust/TypeScript messages, compatibility fixtures, invalid payloads, limits.
- End-to-end tests: browser through Docker against real server/database for vertical journeys.
- Visual/audio tests: golden screenshots, animation contact sheets/playback, map layers, nine-slice UI, loop timing.
- Performance/reliability tests: simulation benchmarks, renderer profiles, load, soak, failover, and recovery.

## Deterministic Simulation

Every authoritative feature supports deterministic tests with injected clock, RNG stream, content release, and ordered commands. Golden traces record initial state, content version, commands/ticks, emitted events, snapshots, and final checksum. Replay equality is checked across repeated runs and supported server platforms.

Use property tests for invariants such as non-negative balances, conservation of item ownership, bounded cooldowns, valid state transitions, and duplicate-command idempotency.

## Economy Integrity

Test concurrent spend, duplicate grant, retry after timeout, transaction rollback, full inventory, content version mismatch, refund/revoke, and out-of-order delivery. Acknowledged mutations must survive restart. Ledger and materialized state must reconcile.

## Asset And Content Testing

CI validates manifest coverage, checksums, references, schemas, IDs, ranges, localization glyphs, media decoding, and deterministic generation. Browser smoke tests load every release bundle. Representative visual/audio review is required for changed groups.

## Compatibility Testing

Maintain protocol fixtures for every supported compatibility version. Test old client/new server within the declared window, unknown fields, unsupported versions, reconnect from snapshots, and content-release pinning.

## Performance Budgets

Track client startup bytes/time, peak memory, frame time p50/p95/p99, draw calls, texture memory, entity count, and garbage-collection pauses. Track server tick duration, queue depth, active zones, snapshot bytes, command latency, DB query latency, and transactions/second.

Performance regression thresholds block merge unless the change includes an approved budget update with evidence.

## Required Journeys

At minimum automate: new session bootstrap, enter village, combat and drop, equip and persist, disconnect/reconnect, offline progress, duplicate reward request, content upgrade, and corrupt/invalid command rejection. Add one full journey with each vertical slice.

## CI Stages

1. formatting, static analysis, schema generation drift;
2. unit/property tests;
3. integration tests with disposable PostgreSQL/Redis;
4. migrations from supported baseline and representative data;
5. asset/content validation;
6. Docker end-to-end and browser smoke;
7. selected security and performance gates.

Nightly/release pipelines run full browser matrices, large asset checks, soak/load, backup restore, and deterministic replay suites.

## Flake And Failure Policy

Do not mask flakes with unrestricted retries. Capture seed, content version, trace ID, screenshots, logs, and replay artifact. Quarantine only with an owner, issue, expiry, and no risk to value integrity or release-critical journeys.
