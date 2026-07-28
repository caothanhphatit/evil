# Original Native Combat Cadence and Stat Chain v1

## Scope

This pass captures exact ARM64 method boundaries from the authorized API 35
runtime for Evil Hunter Tycoon `1.411`. It reads code and static constants only;
it does not capture player/account values. The normalized record is
`reverse-engineering/evidence/original-native-combat-cadence-stat-chain-v1.json`.

## Exact cadence increment

`EvilCtrl.UnitAttack()` is a complete 44-byte body. It sets the Boolean at
offset `132` and writes the float at offset `128` using:

```text
delay = 0.08 * max(field_572, 1.0)
```

The live float at `libil2cpp+0xd2b040` is `0.0799999982`. The fields are still
obfuscated (`JLHEBEFNCPL`, `EPFLFJHGLKH`, `EPIMNKLDPCC`), so `field_572` must
not yet be renamed to attack speed without its writer chain.

`HunterCtrl.HuntingAttackAction()` contains the following exact timing block:

```text
composite = DANCPPLMKIK * decode(BCEBGLKCDHN)
AttackAniTime = composite > 1.0 ? 0.333 / composite : 0.7
```

The destination is the confirmed `AttackAniTime` field at offset `428`.
`DANCPPLMKIK` is a float at `984`; `BCEBGLKCDHN` is an `ObscuredFloat` at
`1708`. The constants were read from `libil2cpp+0xd2b20c` and
`libil2cpp+0xd2b7f8`. Their arithmetic is exact, but their source semantics are
not yet resolved.

## Attack state transitions

- `EvilCtrl.UnitAttackEnd()` clears `mTargetUnit` (`160`) and the Boolean fields
  at `132` and `344`, then dispatches the next FSM action.
- `HunterCtrl.HuntingAttackEnd()` clears `mTargetEvil` (`888`) and
  `mAttackCheck` (`500`) before its next action dispatch.
- `HunterCtrl.HuntingAttackAction()` writes `AttackAniTime` and increments
  `TargetAttackCount` (`896`) once at the common completion tail.

These transitions support a queued attack FSM. They do not support a generic
fixed attack interval shared by Hunter and Evil.

## Damage/stat chain boundary

The exact bodies now pinned together are:

| Method | Token | Module offset | Bytes |
| --- | --- | --- | ---: |
| `HunterCtrl.getDamage` | `0x06005C37` | `0x33f51c4` | 9,496 |
| `HunterCtrl.getCriticalDamage` | `0x06005C4D` | `0x33f97a8` | 1,908 |
| `HunterCtrl.Damaged` | `0x06005CAA` | `0x346514c` | 18,964 |
| `EvilCtrl.GetReduceAttackValue` | `0x06002FAD` | `0x2f0f6d8` | 88 |
| `EvilCtrl.Damaged` | `0x06003014` | `0x2f2be20` | 4,736 |

`getCriticalDamage()` starts from exactly `1.75`, converts multiple runtime
percentage modifiers using `0.01`, and adds them to that base. The modifier
identities and branch gates remain unresolved, so only the base and additive
shape are accepted.

`GetReduceAttackValue()` remains the exact multiplicative stack:

```text
(1 - field_0x1E4) * (1 - field_0x1EC) * (1 - field_0x1F4)
```

The current native evidence does not yet isolate the accuracy/evasion roll,
armor reduction order, final rounding, or damage clamps. None of those are
ported to Rust by this pass.

## Status and critical threshold increment

The API35 runtime schema now gives semantic names to the `StatusData` offsets
used by native combat. The confirmed fields include `CalcDamage` (`40`),
`CalcArmor` (`72`), `CalcAttackSpeed` (`136`), `CalcCritical` (`176`),
`CalcDodge` (`192`), `Damage` (`312`), `Armor` (`344`), `AttackSpeed` (`408`),
`WeaponSpeed` (`428`), `Critical` (`468`) and `Dodge` (`488`). Option, personal
and rank critical/dodge fields are also checksum-pinned in the normalized
evidence.

`HunterCtrl.getDamage(bool,bool,bool)` reads `StatusData.CalcCritical`, may add
the decoded `HunterCtrl.JOFGKPCLDAI` value at offset `2316` behind a runtime
gate, caps the result at `100`, then calls `UnityEngine.Random.Range(0, 100)`.
The confirmed critical branch is:

```text
threshold = min(100, CalcCritical + enabledBonus)
critical = roll < threshold
```

The three Boolean parameter meanings, the bonus gate and target-specific gates
are not resolved. The equation is therefore a confirmed core threshold, not a
complete callable critical-chance contract.

## Incoming monster damage prefix

For `EvilCtrl.Damaged(string, long, int, bool, int, bool)`, the incoming damage
argument arrives in ARM64 register `x2`. The first exact stage is:

```text
damage = truncTowardZero(
  incomingDamage * selectedRuntimeFactor * GameManager.RandDamage()
)
```

The method later reads floats at offsets `476` and `480`. It compares their sum
against the float32 value with bits `0x00000001` (the smallest positive
subnormal), then multiplies the current damage by `1 + sum` on the greater-than
branch. The runtime factor identity and subsequent armor/reduction/rounding
order are unresolved, so this prefix is evidence only.

The full 18,964-byte `HunterCtrl.Damaged` body is now captured without
truncation. It contains integer RNG calls, but the observed branches pass
through nested status arrays and have not been proven to implement dodge or
accuracy. They remain unlabeled until caller or writer evidence closes that
chain.

## Golden vectors

The evidence includes deterministic vectors for both recovered cadence blocks.
They are validation vectors for the recovered arithmetic, not original balance
fixtures for unresolved stat inputs.

## Implementation boundary

- Safe to port later: the two cadence equations, once the input field writers
  give the obfuscated factors stable semantic names.
- Safe now as evidence only: critical base `1.75`, percent scale `0.01`, and the
  three-factor reduction multiplication order.
- Still blocked: accuracy/evasion/dodge, armor/reduction, complete critical
  branch gates and modifiers, final damage rounding and clamp order.
