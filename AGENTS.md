# Repository Agent Rules

These rules apply to every contributor and automation agent in this repository.

## Knowledge Transfer

Read `docs/KT.md` completely before changing this repository. It records the
current implementation, verified migration evidence, unresolved Hunter data,
runtime capture workflow, and validation boundaries. Do not invent original-game
mappings, values, icons, mechanics, or fallback data for unresolved evidence.

## Mission

Rebuild the game as a web client backed by an authoritative server. Preserve observable gameplay and migrated assets while keeping the implementation clean-room, testable, secure, and independently maintainable.

## Source Of Truth

- Product and migration decisions: `docs/migration/vertical-slice-plan.md`
- System boundaries and runtime topology: `docs/architecture/software-architecture.md`
- Asset rules and completeness criteria: `docs/assets/asset-migration-spec.md`
- Engineering standards: `docs/engineering/`
- Accepted technical decisions: `docs/architecture/adrs/`
- Current migration handoff and evidence gaps: `docs/KT.md`

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
8. Prefer evidence under `reverse-engineering/evidence/` and `docs/migration/` over filename order or visual guesses.
9. Preserve unsupported behavior as an explicit unresolved state instead of adding a silent fallback.

## UI Consistency

- Follow `docs/engineering/project-rules.md#source-style-ui-consistency` for every new or changed building screen, popup, and modal.
- Screen dimensions and internal layout may vary with content, but the established frame language, title treatment, close control, action buttons, disabled/focus states, and keyboard/touch interaction behavior must remain consistent with existing game screens.
- Do not ship a technical placeholder or raw evidence diagnostics as the player-facing UI. Unresolved mechanics must remain visible as a compact fail-closed state inside the complete product frame.

## Definition Of Done

A change is done only when it is deterministic, tested, observable, documented where necessary, compatible with Docker development, and has no known critical security or licensing issue. Asset work also requires manifest coverage and visual/audio validation.

## Code Review Priorities

Review in this order: authority or economy exploits, data loss, nondeterminism, protocol compatibility, concurrency safety, performance regressions, missing telemetry, maintainability, then style.
