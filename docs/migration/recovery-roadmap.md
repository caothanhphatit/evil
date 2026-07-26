# Recovery Roadmap

## Purpose

This roadmap replaces percentage-based claims with a reproducible path from the
current visual prototype to a server-authoritative game rebuild. It does not
claim that unknown legacy rules can be recovered from filenames or client-side
behavior alone.

## Research Scope

This repository is an educational security and clean-room compatibility
exercise. The work may analyze checksummed metadata, serialized tables,
locally supplied assets, and observable client behavior to document and
reimplement compatible rules. It must not connect to the original production
backend, recover credentials or player data, bypass executable protection, or
silently present an inferred rule as native-confirmed behavior.

## Current Baseline

The repository currently has three partially disconnected products:

1. an evidence and asset archive recovered from Evil Hunter Tycoon 1.411;
2. an original-flow visual prototype for boot, village, roster, and field;
3. an isolated deterministic combat fixture that is not connected to the main
   WebSocket runtime.

The recovery effort must integrate these through generated contracts and
evidence-backed content releases. A green unit test suite is not sufficient if
the Docker player journey does not exercise the same runtime.

## Non-Negotiable Decisions

- The Rust server owns time, RNG, simulation, progression, rewards, inventory,
  economy, and durable state.
- The browser renders authoritative projections and sends bounded intent only.
- Original evidence, normalized content, and runtime derivatives remain
  separate and checksum-addressed.
- Unknown bindings remain explicitly blocked. A migration fixture must be
  labelled as a fixture and cannot silently become a legacy compatibility
  claim.
- PostgreSQL and Redis are required runtime dependencies. In-memory adapters
  are test fixtures, not a production fallback.
- Each released slice is exercised through the real HTTP, WebSocket, database,
  asset, and browser path.

## Recovery Stages

### R0: Reproducible Evidence Baseline

- Rebuild the 415-file Unity source tree from the checked-in XAPK.
- Verify joined Unity files, exported assets, and manifests from portable tools.
- Pin extractor dependencies and reject both missing and unindexed outputs.
- Publish deterministic scene/content evidence with diagnostics for unsupported
  components.

Exit: a clean checkout can reproduce and validate the evidence baseline without
paths or files from the original analyst's workstation.

### R1: Runtime Foundation

- Harden bootstrap, reconnect, sequence handling, shutdown, dependency timeout,
  session recovery, and lease fencing.
- Make protocol schemas describe complete messages and generate both Rust and
  TypeScript contracts.
- Add Docker-backed PostgreSQL, Redis, WebSocket, and browser integration tests.

Exit: boot and reconnect remain deterministic under delay, duplicate messages,
packet gaps, dependency restart, and server shutdown.

### R2: Source-Faithful Village

- Compile scene transforms, sprite references, sorting, colliders, camera data,
  animation references, UI anchors, fonts, and interaction evidence.
- Replace hard-coded village bindings with a versioned generated scene release.
- Implement one depth model shared by tiles, foreground objects, and actors.

Exit: desktop and mobile golden views use only declared evidence or visibly
labelled migration fixtures, with no hidden fallback bindings.

### R3: Authoritative Core Loop

- Connect fixed-step simulation to the primary session runtime.
- Persist a versioned player aggregate and deterministic world checkpoint.
- Implement movement, target selection, combat, death, revival, drops, pickup,
  equipment, and reconnect through transactional idempotent commands.

Exit: the real Docker journey completes combat, reward, equip, restart, and
reconnect without using the historical fixture transport.

### R4: Progression And Village Management

- Recover and model hunter roster, stats, inventory, buildings, timers, quests,
  shops, mail, localization, offline progress, and content gates.
- Keep all valuable mutations ledger-backed and provider-independent.

Exit: a new durable account reaches an evidence-defined early-game milestone
without admin mutation or client-trusted outcomes.

### R5: Content Breadth

- Add verified maps, monsters, hunters, items, skills, effects, audio, modes,
  bosses, dungeons, raids, and social surfaces as vertical releases.
- Track behavior, asset, localization, animation, and device coverage separately.

Exit: every promoted feature has evidence, deterministic traces, browser goldens,
security cases, telemetry, and rollback instructions.

### R6: Production Hardening

- Add real identity recovery, deployment secrets, backups, restore drills,
  compatibility windows, binary/delta transport, load/soak testing, accessibility,
  moderation, purchase verification abstractions, and incident runbooks.

Exit: launch SLOs, integrity gates, recovery objectives, and legal distribution
approval are demonstrated rather than inferred.

## Immediate Work Packages

The first integration milestone consists of:

1. portable XAPK and asset verification;
2. frontend bootstrap and protocol-fault recovery;
3. production dependency fail-closed behavior;
4. generated scene depth and camera correction;
5. primary-runtime simulation and durable identity design;
6. a fresh-volume Docker browser journey used as the release gate.

Work packages may run in parallel only when they own separate files or consume a
stable generated contract. Integration happens in the order above so later work
does not build on an unverifiable asset or session baseline.
