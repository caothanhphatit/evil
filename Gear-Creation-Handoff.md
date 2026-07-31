# Gear Creation Handoff

Updated: 2026-07-31

## Current Result

- ARM64 API 35 evidence confirms `GearData` stores `quality`, `level`,
  `rating`, four plus/minus option arrays (including additional arrays), and
  `buyGold`.
- Writer identities are confirmed: `CreateGear`, `SetRandOption`,
  `SetRandOptionValue`, `SetAddedRandOption`, and `SetAddedRandOptionValue`.
- The quality pool, option pool, default-option branch, roll order, option enum
  meanings, and mod-dependent `buyGold` formula are not proven.
- The original package detects in-process debug hooks. Do not use a live
  Frida hook for this task; use external native method captures only.

## Server Rule

- Synthetic `%5/%8` stat generation was removed.
- Gear crafting rejects with `gear_creation_evidence_unresolved` before any
  material or wallet mutation.
- Rows from the removed synthetic generator cannot be purchased as if their
  price were mod-dependent; the server rejects them with
  `gear_price_evidence_unresolved`.
- Enhancement remains fail-closed until its own cost/material/probability
  evidence is resolved.

## Evidence Files

- `reverse-engineering/evidence/original-gear-creation-writer-boundary-v1.json`
- `docs/migration/original-gear-creation-writer-boundary-v1.md`
- `docs/game-design/gear-enhancement-flow.md`

## Verification

- `cargo test --lib -- --nocapture`: 294 passed.
- `git diff --check`: passed.
- Emulator was shut down after capture.

## Next Mining Step

Capture external native ranges for the writer callees and correlate controlled
before/after `GearData` values without an in-process hook. Do not implement a
pool or mod-dependent price until those values are bound to exact consumers.
