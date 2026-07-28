# Original Gear Generation Boundary V8

The exact `GetGearDamage`, `GetGearArmor`, and `GetGearAcc` bodies directly read
only `GearData.index`, `gearIndex`, `quality`, `level`, and `rating`. Their
confirmed formulas, rating clamp, quality multipliers, and ties-to-even rounding
remain preserved in the generated evidence.

The package schema confirms `AdminGearData.plusType/plusValue` and
`minusType/minusValue`, but those arrays and rune fields are not read by the
three formula bodies. The caller-supplied level adjustment, option enum meanings,
option roll order, enhancement writers, rune participation, and generation-time
quality/rating order remain unresolved. No enum name or writer order is inferred.

The next evidence required is the exact GearData creation, enhancement, and rune
writer bodies plus the caller that supplies the level adjustment to
`GetFirstPercent`. This pass remains disconnected from live combat.
