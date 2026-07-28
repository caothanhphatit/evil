# Original Reward Unique-Gear Boundary V7

The complete 30,732-byte `RewardMetrial` body still does not mechanically bind
`AdminEvilData.uniqueLevel` or an `AdminDropUniqueGearData` row. The caller does
bind `metIdx`, `metCount`, `metPercent`, and monster `type`.

The exact blocking boundary is now explicit: all 17 `LDHAEMDJCFF` calls pass
only two integers, a `Vector3`, and two Booleans. `GHPHHEFFNKN` receives only an
`ObscuredInt` and `ObscuredBool`. Neither helper receives a typed monster row or
unique-gear row, so array ordering cannot restore the missing object identity.

Consequently `uniqueLevel -> pool`, `dropRange`, `dropCut`, the `gearPercent`
denominator, and gear type/index RNG order remain unset. The next required
capture is the singleton/static lookup returning `AdminDropUniqueGearData`,
including its key, returned row pointer/type, and immediately following RNG
comparisons. No runtime integration or catalog-order fallback is allowed.
