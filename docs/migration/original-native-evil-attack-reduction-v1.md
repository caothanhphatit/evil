# Original Monster Attack-Reduction Recovery v1

## Result

The API 35 exact-boundary capture now covers every `EvilCtrl` method in the
authorized `1.411` runtime. The normalized evidence is
`reverse-engineering/evidence/original-native-evil-attack-reduction-analysis-v1.json`.

`EvilCtrl.BuffSetting(effectType, value, ...)` writes three outgoing-attack
reduction slots. Native code confirms these assignments:

- effect type `8` writes offset `0x1E4` as `value * 0.01`;
- effect type `55` writes offset `0x1EC` as `value * 0.01`;
- a GameManager-owned dynamic effect identifier writes offset `0x1F4` as
  `value * 0.01`.

The matching `BuffEndSetting` branches clear each slot individually and
`BuffAllEndSetting` clears all three. `GetReduceAttackValue` combines them in
this exact order:

```text
(1 - slot_0x1E4) * (1 - slot_0x1EC) * (1 - slot_0x1F4)
```

The float constants were read from the live module's read-only data:
`0xd2ac8c` is float32 `0.01`, and `0xd2b6d0` is float32 `0.0001`. The latter is
used by other effect types and is recorded so percent and basis-point effects
are not conflated.

## Rounding boundary

The complete obfuscated consumer `EvilCtrl.OFEIPNBMNML` multiplies integer
factors, applies the three-slot float multiplier, then converts the result with
ARM64 `fcvtzs` (truncate toward zero). This proves one complete consumer path,
but it is not yet safe to claim that every monster basic attack uses the same
path or factor set.

## Implementation boundary

The arithmetic is exact, but the third effect identifier and the
human-readable names of the slots remain unresolved. The Rust server must not
silently bind them to guessed skill names. Port the formula after the effect
identifier map and every relevant damage caller have deterministic golden
vectors.
