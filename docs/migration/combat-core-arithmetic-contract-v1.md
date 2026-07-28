# Combat Core Arithmetic Contract v1

## Scope

`apps/server/src/simulation/combat_core/` is a disconnected reference layer for
native-confirmed arithmetic. It is not the live combat ruleset and must not be
used to fill unresolved caller, skill, trait, target, or RNG semantics.

This audit is bounded by the recovered Android `1.411` evidence in:

- `original-native-status-data-calc-damage-producers-v1.md`;
- `original-native-hunter-outgoing-damage-chain-v1.md`;
- `original-native-hunter-outgoing-damage-ssa-v2.md`;
- `original-native-hunter-outgoing-helpers-v3.md`;
- `original-native-hunter-get-damage-producers-v4.md`;
- `original-native-hunter-damage-contract-v1.md`;
- `original-native-hunter-skill-coefficients-v1.md`;
- `original-native-hunter-skill-coefficients-v2.md`;
- `original-native-hunter-d8-d10-tree-v5.md`;
- `original-native-hit-miss-chain-pass12.md`;
- `original-native-hunter-constant-coefficients-v1.md`;
- `original-native-hunter-damage-tail-v3.md`;
- `original-native-monster-damage-analysis-v1.json`.

## Numeric contract

- Float32 and float64 stages remain separate. Integer-to-float conversion,
  multiplication/addition order, widening, and every `FCVTZS` boundary are not
  algebraically collapsed.
- `FCVTZS` is claimed only for finite values inside the signed 64-bit range,
  where the evidence establishes truncation toward zero. NaN, infinities, and
  out-of-range values fail closed in the Rust reference layer because their
  product-domain behavior was not captured.
- Native signed integer `add`/`sub` sites represented by the evidence use Rust
  wrapping arithmetic. A later clamp does not turn the preceding operation into
  saturating arithmetic.
- Hunter critical threshold arithmetic performs wrapping signed 32-bit addition,
  then applies only the recovered upper cap of `100`. The Unity roll comparison
  is exclusive. No lower clamp is synthesized.
- Hunter incoming damage subtracts armor with wrapping arithmetic, forwards one
  when the result is non-positive, otherwise converts the selected float32 final
  factor. The default HP branch performs wrapping subtraction and then floors at
  zero.
- Monster incoming damage truncates armor reduction before subtracting it,
  floors effective armor at zero, and enforces minimum damage one. Its recovered
  HP mutation is wrapping subtraction, not a zero clamp; overkill can therefore
  produce a negative reference value.
- The recovered Hunter feel selector divides `nowFeel / feel`. The native block
  has no isolated zero-denominator guard, while runtime data is expected to keep
  the denominator valid. The reference layer rejects zero and non-finite inputs
  instead of inventing behavior.

## Reusable boundaries

- `arithmetic.rs`: checked finite/in-range float-to-`i64` truncation.
- `critical.rs`: threshold wrapping/cap and exclusive roll comparison.
- `hit_resolution.rs`: the exact attacker-owned effect-54 pre-damage abort
  gate, intentionally without an unsupported accuracy/dodge/miss label.
- `status_damage.rs`: ordered float32/float64 CalcDamage producer and potion
  wrapping/clamp tail.
- `hunter_incoming.rs`: feel armor, armor scratch, minimum forwarded damage,
  first-shield routing, and Hunter HP mutation.
- `monster_incoming.rs`: feel scaling, variance, direct/pre-armor bonuses,
  effective armor, minimum damage, and wrapping monster HP mutation.
- `outgoing.rs`: exact critical-damage positive gates, percentage scaling,
  GearProperty temporary/cap chain, and the final `getDamage` float64 SSA and
  truncation boundary; it also contains the named Slayer race segment and the
  exact gated Rift-NPC integer scale. Recovered S12/S13/D14 arithmetic,
  GearSet stack/S8 factors, and job-specific Collection/Relic damage selection
  are reusable here. The recovered D8/D10 base selector, float32 attack-speed
  division, JobTrait(5) augmentation, reduction slot, early-percent additions,
  and optional later job multiplier are also represented without assigning
  product names to opaque operands.
- `skill.rs`: the exact Blizzard modifier segment plus the proven plain-percent,
  affine-percent and internal-ObscuredInt-percent coefficient families, all
  with their exact float32 ordering and final truncation. It also preserves the
  distinct Poison Aura and Curse Aura truncation chains, optional whole-damage
  integer scaling used by the Frozen Heart callers, the Sniping coefficient
  chain, and Thunder Dragon Fury's wrapping integer-power construction. Action
  routing, opaque gates and public skill bindings remain outside core.

## Explicit exclusions

The core does not resolve or enable the complete outgoing caller chain, skill
coefficients outside the fifteen proven caller bodies, product names for three
critical-damage Hunter fields, opaque Slayer additions, producer
semantics for every final SSA register, all 32 optional Hunter pre-armor gates,
the global `CalcDodge` consumer and any generic accuracy/evasion formula,
multi-shield dictionary order, or live combat integration. Tests cover
arithmetic boundaries, not original balance or valid player-stat ranges.

## Validation

```sh
cargo test --manifest-path apps/server/Cargo.toml combat_core
cargo test --manifest-path apps/server/Cargo.toml
```
