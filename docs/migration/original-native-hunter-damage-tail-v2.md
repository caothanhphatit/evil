# Original Native Hunter Damage Tail v2

> **Superseded:** v2 used an incomplete direct-XOR ACTk decode and therefore
> misclassified the final factor as mutable runtime state. The corrected owner,
> writer, plaintext value and complete pre-armor chain are documented in
> `original-native-hunter-damage-tail-v3.md`.

## Scope

This pass resolves the armor-factor selector and the shield-routing branch left
open by v1. It combines complete API35 native bodies, runtime type schemas,
package constants and a rooted guest-tutorial capture of static `GameManager`
data. The static capture does not read Hunter objects, saves, account values or
service credentials.

The deterministic analyzer is:

```sh
python3 tools/analyze-hunter-damage-tail.py
```

Its output is
`reverse-engineering/evidence/original-native-hunter-damage-tail-v2.json`.

## Armor-factor selector

The selector reads `HunterData.feel` at offset `392` and
`HunterData.nowFeel` at offset `412`. Four package float constants define five
bands. The selected values come from the five-element static `GameManager`
array used directly by `HunterCtrl.Damaged`:

| Float32 ratio condition | Armor factor |
| --- | ---: |
| `nowFeel / feel >= 0.8` | `1.2` |
| `nowFeel / feel >= 0.6` | `1.1` |
| `nowFeel / feel >= 0.4` | `1.0` |
| `nowFeel / feel >= 0.2` | `0.9` |
| `nowFeel / feel < 0.2` | `0.8` |

The native equation is therefore:

```text
armorScratch = truncTowardZero(CalcArmor * feelBandArmorFactor)
```

The float32 comparisons are ordered from `0.8` down to `0.2`, and equality
remains in the higher band. The division must not be algebraically rewritten as
`nowFeel >= feel * threshold`, because float32 rounding changes equality vectors
such as `60 / 100`. The selected native block does not isolate a denominator-
zero branch; the normal `feel` invariant is still required before live use.

## Final factor boundary

The positive post-armor branch decodes the `ObscuredFloat` at static runtime
data offset `0x114` and executes:

```text
forwardedDamage = truncTowardZero(postArmor * selectedFinalFactor)
```

The rooted capture resolves the pointer source, raw ACTk bytes and value for the
captured process. Repeated reads in that process are stable. A previous process
produced a different value, so this field is runtime state rather than a proven
package constant. Its writer and gameplay identity remain unresolved.

The capture workflow is reproducible with:

```sh
python3 tools/runtime/capture-combat-static-factors.py \
  --output reverse-engineering/evidence/original-runtime-combat-static-factors-api35-v1.json \
  --action "<exact capture action>"
```

## Shield routing

The earlier `HitDamageProcess` branch is now schema-resolved.
`HunterData.mShieldDataDic` is a
`Dictionary<String,ShieldData>` at offset `1488`. `ShieldData` contains:

- `MaxShield`: `ObscuredLong` at offset `16`;
- `CurrentShield`: `ObscuredLong` at offset `48`.

When the dictionary contains at least one entry, the method enumerates it and
uses the first yielded `ShieldData`:

```text
if CurrentShield < forwardedDamage:
    forwardedDamage -= CurrentShield
    CurrentShield = 0
else:
    CurrentShield -= forwardedDamage
    forwardedDamage = 0

nowHp = max(nowHp - forwardedDamage, 0)
```

Shield routing therefore occurs before HP subtraction. Exact ownership and
ordering semantics for multiple simultaneous dictionary entries remain
unresolved.

## Implementation boundary

Now exact:

- feel-band selection and all five armor factors;
- armor factor boundary behavior and truncation;
- the static runtime source of the positive final factor;
- shield absorption, spillover and HP ordering;
- `ShieldData` field offsets and types.

Still blocked:

- writer and semantic name of the final factor;
- multi-entry shield dictionary ordering/ownership;
- complete semantic normalization of all pre-armor modifiers.
