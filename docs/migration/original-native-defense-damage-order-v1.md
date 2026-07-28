# Original Native Defense and Damage Order v1

## Scope

This pass analyzes complete exact-boundary ARM64 bodies from the authorized
Android API35 guest tutorial runtime for Evil Hunter Tycoon `1.411`. It reads
native code and runtime type metadata only. It does not read Hunter values,
private save data, account data, or service credentials.

The normalized record is
`reverse-engineering/evidence/original-native-defense-damage-order-v1.json`.

## Hunter armor intermediate

`HunterCtrl.Damaged(Int64,String)` has a complete 18,964-byte body. Native code
calls `getStatusData()`, copies the 32-byte `ObscuredLong` at `StatusData+72`,
and decodes it. The API35 schema identifies this field as `CalcArmor`.

The recovered arithmetic is:

```text
scratch = truncTowardZero(CalcArmor * selectedArmorFactor)
scratch = truncTowardZero(scratch * (1 - decode(field_1952)))
```

The first result is stored in the `HunterCtrl` `ObscuredLong` at offset `992`
(`OLCMCGJLADG`). The second factor is the `ObscuredFloat` at offset `1952`
(`DJDEHDEKGIO`). Both conversions use ARM64 `FCVTZS`, so finite in-range values
truncate toward zero.

This is an exact armor intermediate, not yet a complete damage-intake formula.
Earlier branches choose `selectedArmorFactor` from a runtime float table whose
selector semantics are unresolved. Later code has not yet been reduced to a
proven subtraction, minimum-damage clamp and final HP mutation order. The two
obfuscated Hunter fields therefore retain their runtime names.

## Monster incoming-damage prefix

`EvilCtrl.Damaged(String,Int64,Int32,Boolean,Int32,Boolean)` receives the damage
argument in ARM64 register `x2`. Its first recovered stage is:

```text
damage = truncTowardZero(
  incomingDamage * selectedRuntimeFactor * GameManager.RandDamage()
)
```

It then sums the floats at `EvilCtrl+476` and `EvilCtrl+480`. The native compare
constant is not `1.0`: it is float32 bits `0x00000001`, the smallest positive
subnormal. On the greater-than branch:

```text
damage = truncTowardZero(damage * (1 + fieldSum))
```

This corrects the earlier provisional wording that described the branch as
`fieldSum > 1`. The selected factor and later reduction/subtraction/clamp chain
remain unresolved.

## Dodge and accuracy boundary

The captured `StatusData` schema contains `CalcDodge` at offset `192`, but no
field whose runtime name contains `Acc`, `Accuracy`, or `Hit`. The separate
`GameManager.GetGearAcc` helper is evidence for a gear statistic only; it does
not identify the combat hit-roll source.

The full `HunterCtrl.Damaged` body calls integer `Random.Range` four times. The
observed thresholds come from nested `StatusData` arrays, and no
`getStatusData()` result is directly followed by a read/decode of offset `192`.
Those branches may be traits, gear properties or other procs. They are not
labeled dodge, evasion, accuracy or miss without an exact array-index or caller
binding.

## Port boundary

Safe as evidence:

- `CalcArmor` is the source of the recovered armor intermediate.
- Both armor-intermediate stages truncate toward zero.
- The post-armor shape is multiplication by `1 - decoded field_1952`.
- The Evil prefix uses `RandDamage`, then conditionally multiplies by
  `1 + field_476 + field_480` after the exact subnormal comparison.

Still blocked from Rust integration:

- armor factor table selection and semantics;
- the final armor subtraction, minimum damage and HP mutation order;
- exact dodge/evasion/accuracy RNG and threshold;
- complete monster reduction, rounding and clamps.
