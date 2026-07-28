# Original Native Hunter D8/D10 Tree v5

Pass 14 isolates the early base tree in the complete `getDamage` body.

Argument 1 mechanically selects the initial base:

- false: decoded raw `StatusData.CalcDamage`;
- true: `CalcDamage / CalcAttackSpeed`, with a gated `CheckJobTrait(5)`
  augmentation using opaque AdminData values.

The `CheckJobTrait(5)` branch requires `HMKFKBCNPDH@0x6C0 > 1` and the trait
check to decode true. It evaluates two opaque AdminData floats and a decoded
skill-dictionary integer in Float32, converts the sum to Float64, multiplies by
the exact Float64 literal `0.01` (`0xD282B0`, raw `7b14ae47e17a843f`), then
applies `D8 += D8 * percent`. No integer rounding occurs in this branch.

Both paths then apply `D8 *= 1 - HunterCtrl.DJDEHDEKGIO` when that field is
nonzero. The early percent accumulator starts from
`StatusData.gearPropertyNeedMoveSpeed`, can add
`DragonProtectionFairyAtkValue` behind a GearProperty gate, and can add
`RidingPetGearProperty[11]*0.01` while not in meze state. The common boundary is
`D10 = D8 * (1 + earlyPercent)`.

The following tree decodes `HunterData.job/subJob`. Explicit special pairs in
the body are `(1,2)`, `(1,3)`, `(4,1)`, and `(4,3)`. These paths use
`GearProperty`, `RunesProperty`, AdminData rows and static index pairs before a
shared `D10 *= 1 + percentExpression` instruction. Fixed pre-D10 GearProperty
gates include indices 79 and 99; several other row/index choices remain dynamic
and are not assigned product labels.

The exact instruction order is normalized in the evidence JSON. AdminData type
labels, dynamic GameManager indices, the static skill lookup key, and the
caller-facing name of argument 1 remain fail-closed. No live integration is
made.
