# Repository Agent Rules

These rules apply to every contributor and automation agent in this repository.

## Mission

Rebuild the game as a web client backed by an authoritative server. Preserve observable gameplay and migrated assets while keeping the implementation clean-room, testable, secure, and independently maintainable.

## Source Of Truth

- Product and migration decisions: `docs/migration/vertical-slice-plan.md`
- System boundaries and runtime topology: `docs/architecture/software-architecture.md`
- Asset rules and completeness criteria: `docs/assets/asset-migration-spec.md`
- Engineering standards: `docs/engineering/`
- Accepted technical decisions: `docs/architecture/adrs/`

If code and documentation disagree, stop and resolve the mismatch in the same change.

## Non-Negotiable Architecture Rules

- The Rust server is authoritative for simulation, RNG, time, progression, inventory, economy, rewards, and persistence.
- The browser sends player intent, never trusted outcomes such as damage, currency, loot, or completion.
- PixiJS owns world rendering. UI frameworks must not render per-frame game entities.
- PostgreSQL stores durable state; Redis stores ephemeral coordination, sessions, rate limits, and hot caches.
- Network contracts are versioned and generated from a single schema source.
- A vertical slice includes client, server, data, assets, tests, telemetry, and operational documentation.
- Do not connect the rebuild to the original production backend or reuse secrets, certificates, account data, or proprietary service credentials.

## Change Rules

1. Read the nearest documentation and existing tests before editing.
2. Keep changes scoped; do not reformat or rewrite unrelated files.
3. Add or update tests for every behavioral change.
4. Run the narrowest relevant checks first, then the repository quality gate.
5. Update architecture docs or add an ADR when changing a system boundary, data authority, protocol, persistence model, or major dependency.
6. Never commit generated build output, decrypted credentials, user data, APK signing material, or unreviewed extracted binaries.
7. Treat migrated assets as immutable source artifacts. Derivatives are reproducibly generated into build directories.

## Definition Of Done

A change is done only when it is deterministic, tested, observable, documented where necessary, compatible with Docker development, and has no known critical security or licensing issue. Asset work also requires manifest coverage and visual/audio validation.

## Code Review Priorities

Review in this order: authority or economy exploits, data loss, nondeterminism, protocol compatibility, concurrency safety, performance regressions, missing telemetry, maintainability, then style.
