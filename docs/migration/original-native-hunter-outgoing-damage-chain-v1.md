# Original Native Hunter Outgoing Damage Chain v1

## Scope

This evidence pass covers Evil Hunter Tycoon `1.411` on Android API 35. It does
not change UI or connect an incomplete formula to the authoritative server.

```sh
python3 tools/analyze-original-native-hunter-outgoing-damage-pass6.py
python3 -m unittest tools.tests.test_analyze_original_native_hunter_outgoing_damage_pass6
```

## Proven base boundary

`HunterCtrl.getDamage(bool,bool,bool)` starts by calling
`StatusData.LCENGICKKGP()`. Its exact 136-byte body decodes
`CalcDamage@0x28` and `CalcAttackSpeed@0x88`, then returns:

```text
ObscuredDouble(float(CalcDamage) / CalcAttackSpeed)
```

No integer rounding occurs in that helper. `CalcDamage` is already aggregated;
this is not evidence that `GameManager.GetGearDamage` alone is final attack.

## Critical path

The threshold block decodes `StatusData.CalcCritical@0xB0`. When the second
Boolean argument does not bypass the roll, it can add
`HunterCtrl.JOFGKPCLDAI@0x90C` behind the adjacent gate at `0x924`, clamps the
sum to 100, and tests `Random.Range(0,100) < threshold`.

The third Boolean is a separate downstream gate over target-specific critical
damage and Slayer/Rift helper branches; it does not bypass the roll itself.

`getCriticalDamage()` starts at exactly `1.75`. Named `StatusData` contributors
read by its body include `VillagePetCriDemUp`, `CollectionCriDem`,
`RelicCollectionCriDem`, `HeroicJobTraitCriDemUp`, `RidingPetCriDemUp`, and
`SylphBlessCriDemUp`. Other `HunterCtrl` values and array/tag gates remain
obfuscated, so a complete critical formula is not yet safe to port.

## Variance and final rounding

`HunterCtrl.getDamage` has no direct call to
`GameManager.RandDamage@0x2706384`. The recovered variance stream is consumed
downstream in `EvilCtrl.Damaged`, not while constructing the outgoing value.

At its final boundary, `getDamage` performs chained double arithmetic and then
executes `FCVTZS x9,d0`: truncation toward zero occurs before the obscured return
value is built.

## Fail-closed gaps

- Full `StatusData.CalcDamage` producer and gear caller adjustment.
- Semantics/order of every opaque modifier and target/tag gate.
- Caller-provided skill coefficient boundary.
- Monster armor and minimum-damage consumer. Hunter incoming armor evidence is
  not interchangeable with this path.
- Complete caller vectors for normal, critical, skill and monster-type cases.

Rust integration remains disconnected until these inputs and vectors close.
