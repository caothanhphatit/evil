# Original Native Gear Formula Recovery v2

Pass 3 resolves the remaining helper at module offset `0x26bdf7c` as
`GameManager.GetFirstPercent`, token `0x06006B04`. Its exact 1,108-byte body has
SHA-256 `93763390fa4008b56c5c4a3cb16d085b3005d778fc2a9cb8246490a553cea3b`.

Runtime schema reflection names the formula-facing `AdminGearData` fields:

| Field | Offset |
| --- | ---: |
| `ratingValue` | 112 |
| `firstValue` | 128 |
| `firstPercent` | 152 |
| `secondValue` | 160 |

`GetFirstPercent` sums `firstPercent[0]` for steps 1-5, index 1 for 6-10,
index 2 for 11-15, index 3 for 16-20, and indices 4-8 for steps 21-25.
Step 0 and steps above 25 add zero; a negative limit returns zero.

The exact structural damage expression is:

```text
roundToEven(
  firstValue
  * ratingValue[min(rating, last)] / 100
  * (1 + GetFirstPercent(index, gearIndex, level + adjustment) / 100)
  * qualityMultiplier
  * secondValue / 100
)
```

Quality multipliers remain `0 -> 0.8`, `1 -> 0.9`, `3 -> 1.1`, `4 -> 1.2`,
otherwise `1.0`. Midpoint rounding is ties-to-even.

This resolves native structure, not design semantics. The caller-provided level
`adjustment`, option enum meanings, and any plus/minus/rune effects applied by
other methods remain unresolved. The inspected `GetGearDamage`,
`GetGearArmor`, and `GetGearAcc` bodies directly use only `GearData.index`,
`gearIndex`, `quality`, `level`, and `rating`; they do not directly read the
plus/minus/rune arrays.

No runtime or UI code is changed by this recovery pass.
