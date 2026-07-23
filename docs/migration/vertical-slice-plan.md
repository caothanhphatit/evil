# Vertical Migration Plan

## Strategy

Migrate a whole playable path at a time: evidence, assets, content, server rules, persistence, protocol, rendering, UI, audio, tests, telemetry, and Docker operation. Avoid migrating all assets, then all backend, then all frontend as disconnected horizontal phases.

The target is behavioral and presentation compatibility, while the implementation remains independently designed and server-authoritative.

## Release Gates

Every slice must pass:

- deterministic server tests and replayable golden traces;
- protocol compatibility and reconnect tests;
- transaction/idempotency tests for valuable state;
- automated asset manifest and reference validation;
- visual regression at supported viewport/device tiers;
- performance budget and soak test;
- logs, metrics, traces, dashboards, and actionable alerts;
- threat-model review for new commands and rewards;
- Docker clean-start verification and migration documentation.

## Slice 0: Foundation And Evidence

Deliver:

- repository/module boundaries, Docker development environment, CI quality gates;
- versioned protocol schema and generated Rust/TypeScript contracts;
- content import framework, asset manifest, checksum/provenance records;
- account/session skeleton, PostgreSQL migrations, Redis integration;
- PixiJS renderer shell, asset loader, camera, input, diagnostics overlay;
- evidence catalog and feature gap register.

Exit: a browser boots through Docker, authenticates to a local server, loads one versioned content/asset manifest, opens a WebSocket, and displays a diagnostic scene.

## Slice 1: Village Combat Loop

Scope one village area, one hunter archetype, one monster, and one equipment/drop path.

Deliver:

- map layers, actor sprites, animation clips, effects, audio, and fonts;
- server-owned movement, targeting, AI, attack cadence, damage, death, revival, RNG, and drop;
- browser interpolation, animation state projection, camera, selection, and combat HUD;
- transactional inventory and reward ledger;
- reconnect, resync, and deterministic replay.

Exit: `enter village -> hunter finds monster -> combat -> loot -> equip -> persisted improvement` matches documented references end to end.

## Slice 2: Village Management And Progression

Deliver buildings, production timers, upgrades, hunter roster, stats, equipment management, town currency, offline progress, and core menus. Introduce event-driven inactive simulation.

Exit: a new account can progress through the documented early-game milestone without admin intervention.

## Slice 3: Tutorial, Quest, And Content Breadth

Deliver tutorial state machine, quests, achievements, unlock gates, localization coverage, additional jobs/monsters/items/maps, mail, notifications, and content release tooling.

Exit: the early-game journey has no blocker, missing reference, untranslated key, or placeholder presentation.

## Slice 4: Instanced Combat

Deliver dungeons, bosses, raid/rift variants, encounter lifecycle, party rules, timers, failure/reward states, and instance ownership/recovery.

Exit: representative easy, boundary, failure, reconnect, and completion traces pass deterministically.

## Slice 5: Social And Competitive

Deliver guild membership/roles, guild content, rankings, PvP-compatible simulation, seasons, and moderation/audit flows. Never trust client-submitted scores or battle outcomes.

Exit: concurrency, season rollover, abuse cases, and ranking reconciliation pass load and integrity tests.

## Slice 6: Monetization-Compatible Services

Deliver product catalog, entitlement model, receipt-verification abstraction, rewarded-ad callback abstraction, coupon, inbox grants, refunds/revocation, and immutable transaction audit. Development uses fake providers only.

Exit: every grant is idempotent, traceable, revocable where required, and cannot be issued by a browser claim alone.

## Slice 7: Completeness And Launch Hardening

Deliver remaining approved content/assets, compatibility gap closure, device tuning, accessibility, disaster recovery, capacity planning, security review, backups, operations runbooks, and staged rollout.

Exit: the asset/content manifest reaches 100% approved-source coverage; all required gameplay scenarios pass; launch SLOs and rollback drills are demonstrated.

## Work Package Template

Each migrated feature is tracked with:

```text
Feature:
Evidence and confidence:
Source asset/content IDs:
Authoritative states and commands:
Persistence/transaction boundary:
Protocol messages:
Client rendering and UI:
Golden traces:
Visual/audio references:
Performance budget:
Security/abuse cases:
Telemetry and alerts:
Known gaps:
Owner and review status:
```

## Parallel Work Without Integration Debt

- Analysis publishes evidence and dossiers before implementation assumptions harden.
- Asset/content work publishes immutable manifests consumed by both client and server.
- Server and client work against generated contracts and golden traces.
- Quality work builds deterministic, visual, load, and security harnesses with each slice.
- One slice integration owner controls scope and accepts the exit criteria.

## Progress Measurement

Do not report one vague completion percentage. Track separately:

- feature behavior coverage;
- authoritative rule/formula coverage;
- approved asset object and byte coverage;
- animation state/clip coverage;
- localization key/locale coverage;
- golden scenario pass rate;
- supported-device visual and performance pass rate;
- unresolved high-risk gaps.

The project reaches “95%” only when the agreed weighted rubric is published and all remaining gaps are explicitly accepted. File extraction alone is never counted as migrated functionality.
