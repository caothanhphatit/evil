# Weapon Difficulty Progression (v1)

This is a rebuild design decision layered on top of the package catalog. It
does not claim that the original package used this exact level curve.

## Difficulty and level

- Difficulty is one-based.
- Each difficulty owns a base-weapon pool and a maximum of 100 weapon levels.
- Global weapon level cap is `difficulty * 100`; difficulty 8 therefore caps at
  level 800.
- A base weapon declares its own `base_level_cap`. Effective weapon level is
  clamped by the global difficulty cap and the base cap:

```text
effectiveLevel = min(rolledLevel, difficulty * 100, baseLevelCap)
```

- A weapon is unavailable until its base difficulty is unlocked.

## Base-power thresholds

The package starts the five recovered starter weapon families at
`firstValue = 60` and reaches approximately the same power order around its
level-320 rows that this rebuild targets at level 800. To reduce progression
pressure while preserving that broad range, rebuild v1 uses this deterministic
threshold curve:

```text
basePower(level) = roundHalfEven(60 * 1.6^(level / 100))
```

| Level | Base power |
| ---: | ---: |
| 0 | 60 |
| 100 | 96 |
| 200 | 154 |
| 300 | 246 |
| 400 | 393 |
| 500 | 629 |
| 600 | 1007 |
| 700 | 1611 |
| 800 | 2577 |

This curve is `rebuild-designed`, informed by the recovered package range; it
is not claimed as an original-game formula. The recovered starter-family
`secondValue` factors remain separately stored as package evidence:
Berserker `180`, Paladin `200`, Ranger `150`, Sorcerer `210`, and Dark Knight
`200`. Their final gameplay interpretation remains formula-bound rather than
being renamed as weapon speed or another unsupported semantic.

## Item layering

Weapon generation keeps these layers separate:

1. Base weapon identity, implicit/base stats, and its level cap.
2. Quality multiplier.
3. Explicit prefix/suffix affixes. Their pool weight is independent of the
   base weapon; the base level cap only controls which item-level modifier
   tiers can be selected.
4. Special explicit properties such as transformation or skill procs.
5. Set/Virtue contribution, which does not consume prefix/suffix slots.

Rarity slot budgets remain Normal `0/0`, Blue `1/1`, Purple `2/2`, and Gold
`3/3` for prefix/suffix.

The server must clamp the level and validate the difficulty/base pool before
persisting a generated weapon. Modifier eligibility is then evaluated from
the item's level against the modifier's tier requirements. Unknown package
generation pools remain fail closed; this document defines the rebuild
progression boundary only.
