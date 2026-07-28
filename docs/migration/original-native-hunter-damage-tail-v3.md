# Original Native Hunter Damage Tail v3

## Scope

This pass corrects the v2 ACTk interpretation, resolves the exact writer of the
positive post-armor factor, and normalizes every accumulator mutation before
armor subtraction in `HunterCtrl.Damaged`. It is evidence-only: no protocol,
browser, database or live Rust combat behavior changes in this pass.

The deterministic analyzer is:

```sh
python3 tools/analyze-hunter-damage-tail.py
```

It generates
`reverse-engineering/evidence/original-native-hunter-damage-tail-v3.json`.

## Corrected final factor

The global at `libil2cpp+0x601c6e0` owns `ConstantData`, not `GameManager`.
Runtime reflection resolves the exact static init-only field:

```text
ConstantData.DEFALUT_DAMAGE_DECREASE_VALUE
offset 0x114
type CodeStage.AntiCheat.ObscuredTypes.ObscuredFloat
```

ACTk `ObscuredFloat` does not decode through a direct integer XOR. Its
`hiddenValue` bytes are first passed through `ACTkByte4.UnShuffle`, which swaps
the middle two bytes, and only then XORed with `currentCryptoKey`. Both captured
session values decode exactly to float32 `0.75`:

```text
fba7067e fb46a741 --UnShuffle--> fba74641 -> 0x3f400000 -> 0.75
01c30e4a 014ec375 --UnShuffle--> 01c34e75 -> 0x3f400000 -> 0.75
```

The differing raw crypto bytes observed by v2 are therefore expected randomized
ACTk storage, not evidence of a mutable gameplay factor.

`ConstantData..cctor` constructs the value with
`ObscuredFloat.op_Implicit(0.75f)` and stores it at runtime data offset `0x114`.
The positive damage branch is exact:

```text
forwardedDamage = truncTowardZero(postArmor * 0.75f)
```

## Pre-armor accumulator

`HunterCtrl.LIEGAADKDHD` is an `ObscuredLong` at offset `0x760`. Its initial
value is:

```text
A0 = truncTowardZero(float32(incomingDamage) * GameManager.RandDamage())
```

The complete native body contains exactly 32 optional accumulator writes before
armor subtraction. The analyzer pins every instruction window by SHA-256 and
keeps their execution order. Their normalized arithmetic belongs to these exact
families:

- proportional add or subtract;
- summed proportional subtract;
- negative percent-point or basis-point add;
- fixed scale;
- one-minus-percent scale;
- direct-product or percent-product subtract.

After checkpoint 32, the next operation is:

```text
A33 = A32 - armorScratch
```

The machine-readable evidence records the source field or native value boundary
for every checkpoint. Named examples include `StatusData.GearArmorUpgrade`,
`StatusData.CostumeImmuneUp`, `StatusData.RidingPetImmuneUp`,
`ConstantData.IMMUNE_VALUE`, `EXECUTOR_DAMAGE_DECREASE_VALUE`,
`SOUL_ABSORPTION_DECREASE_VALUE`, `FRENZY_ARMOR_DECREASE_VALUE`, and
`FROZEN_HEART_DAMAGE_DECREASE_VALUE`.

## Implementation boundary

Now exact:

- `ConstantData` ownership and `DEFALUT_DAMAGE_DECREASE_VALUE` field offset;
- ACTk field layout, UnShuffle decode, exact plaintext `0.75` and cctor writer;
- accumulator initialization and all 32 pre-armor mutation checkpoints;
- arithmetic family, native order and instruction-window hash for each stage;
- armor subtraction as operation 33;
- previously recovered feel-band armor selection and shield-before-HP routing.

Still unresolved:

- product-facing names for obfuscated Hunter/effect gates in several stages;
- whether each optional stage belongs to ordinary village combat, PvP, bosses,
  traits or late-game modes;
- multi-entry `mShieldDataDic` ownership and enumeration semantics.

Those unknown gates must remain explicit. The recovered arithmetic must not be
enabled in live authoritative combat until its reachability and operand meaning
are proven for that gameplay mode.
