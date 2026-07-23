# ADR-0005: Vertical Slice Migration

- Status: Accepted
- Date: 2026-07-22

## Context

The source package contains many interdependent assets, content records, animation references, and partially recoverable behavior. Migrating horizontal layers in bulk would postpone integration and make completeness claims unreliable.

## Decision

Migrate complete player journeys vertically. Each slice includes evidence, canonical content, all required assets, authoritative server rules, persistence, protocol, web rendering/UI/audio, tests, telemetry, security review, and Docker operation.

The first functional slice is village combat through persisted loot/equipment. Subsequent slices expand management, progression, instances, social/competitive, external grants, and completeness.

## Consequences

- Playable evidence arrives early and validates architecture against real requirements.
- Some pipeline/platform work is repeated or generalized incrementally, which is accepted.
- Product scope per slice must be controlled tightly.
- Completeness is measured across behavior, assets, animation, localization, scenarios, and supported devices, not by copied file count alone.

## Rejected Alternatives

- Copy every asset before runtime integration: rejected because reference and playback failures would surface too late.
- Implement the whole backend before client work: rejected because protocol and presentation assumptions would remain unvalidated.
- Rebuild feature-by-feature only in the frontend: rejected because it violates server authority and creates throwaway logic.
