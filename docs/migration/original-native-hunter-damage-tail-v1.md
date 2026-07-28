# Original Native Hunter Damage Tail v1

## Scope

This pass follows the complete API35 `HunterCtrl.Damaged` body from its damage
accumulator through armor subtraction into `HitDamageProcess`, then maps all
captured Evil callers of `HunterCtrl.Damaged`. It uses code and runtime schemas
only; no live Hunter or account values are read.

Normalized evidence lives at
`reverse-engineering/evidence/original-native-hunter-damage-tail-v1.json`.

## Final common damage tail

`HunterCtrl.Damaged(Int64,String)` first initializes an `ObscuredLong`
accumulator at offset `1888` with:

```text
accumulator = truncTowardZero(incomingDamage * GameManager.RandDamage())
```

Many conditional branches mutate this accumulator. Their skill, gear, trait
and mode meanings are not all resolved. After those branches converge, native
code performs the armor subtraction recovered in the previous pass:

```text
postArmor = accumulator - armorScratch
```

`armorScratch` is `HunterCtrl.OLCMCGJLADG` at offset `992`, already recovered
as the two-stage `CalcArmor` intermediate.

The common tail then splits:

```text
if postArmor <= 0:
    forwardedDamage = 1
else:
    forwardedDamage = truncTowardZero(postArmor * selectedFinalFactor)

HitDamageProcess(forwardedDamage)
```

The positive branch does not contain another minimum-one clamp before
`HitDamageProcess`. The runtime factor identity is unresolved, so this tail is
not yet safe to port as a complete formula.

## HP mutation

The default `HitDamageProcess` branch reads `HunterData.nowHp`, whose API35
runtime field is the `ObscuredLong` at offset `360`, and executes:

```text
nowHp = nowHp - forwardedDamage
if nowHp < 0:
    nowHp = 0
```

An earlier collection-backed branch can split or redirect damage through an
auxiliary pool. Its collection identity remains unresolved; only the default
HP path is normalized.

## Dodge and accuracy audit

The four integer RNG calls inside `HunterCtrl.Damaged` are now mechanically
rejected as direct `CalcDodge` or accuracy rolls:

- two read `StatusData.GearSetProperty` and `GearSetPropertyValue`;
- one reads `StatusData.GearProperty`;
- one is a separate Hunter effect proc.

None reads `StatusData.CalcDodge` at offset `192`.

Complete-capture caller scanning finds three direct native calls to
`HunterCtrl.Damaged`: two in `EvilCtrl.OFEIPNBMNML` and one in
`EvilCtrl.EBNOJHOGEMM`. `OFEIPNBMNML` contains no `Random.Range` call.
`EBNOJHOGEMM` has one pre-damage gate:

```text
if EvilCtrl.OCLFGGEJKMI >= 1:
    abortAttack = Random.Range(0, 100) < OCLFGGEJKMI
```

`BuffSetting(effectType=54, value, bool)` writes `value` directly to this Evil
field at offset `488`, and `BuffEndSetting` clears it. The proc branch exits via
effect presentation before `HunterCtrl.Damaged` is called.

This proves an attacker-owned effect-54 attack-abort percentage gate. It does
not prove the public gameplay name of effect 54. Calling it accuracy, blind,
miss, dodge or evasion would still be a guess. The original `CalcDodge`
consumer remains unresolved.

## Implementation boundary

Exact and testable:

- randomized incoming-damage accumulator initialization;
- late subtraction of the recovered armor intermediate;
- non-positive `postArmor` replacement with one;
- positive-branch final-factor multiplication and truncation;
- default `nowHp = max(nowHp - forwardedDamage, 0)` mutation;
- effect-54 attack-abort threshold and control-flow position.

Still blocked:

- full pre-armor modifier semantics and order;
- final-factor identity and zero-output boundary;
- auxiliary damage-pool behavior;
- semantic name of effect 54;
- `StatusData.CalcDodge` consumer and formula.
