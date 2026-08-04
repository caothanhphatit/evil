# Weapon Affix Pool (v1)

This is a rebuild-designed weapon-generation contract. Package IDs, localized
descriptions, and original positive ranges remain package evidence; assigning a
property to prefix/suffix, setting its weight, splitting its range into tiers,
and defining duplicate groups are new product decisions.

## Generation order

1. Clamp item level by the selected base weapon's level cap.
2. Read prefix/suffix slot counts from rarity.
3. Select weighted active affixes for the weapon class; weight is independent
   of the base weapon and its level cap.
4. Reject candidates whose `exclusiveGroup` already exists on the item.
5. From the selected pool row, choose an eligible modifier tier by item level.
6. Roll one inclusive integer value with a server-owned RNG stream.
7. Roll special explicit, transformation, and Virtue/set layers separately.

Normal has `0P/0S`, Blue `1P/1S`, Purple `2P/2S`, and Gold `3P/3S`.

## Prefix pool

| Modifier | Source | Group | Weight |
| --- | --- | --- | ---: |
| Flat attack damage | rebuild | `attack_base` | 100 |
| ATK percent | property 5 | `attack_percent` | 100 |
| Additional damage | property 36 | `additional_damage` | 80 |
| Critical-hit damage | property 43 | `critical_damage` | 80 |
| Primate damage | property 11 | `race_damage` | 50 |
| Demon damage | property 12 | `race_damage` | 50 |
| Undead damage | property 13 | `race_damage` | 50 |
| Boss damage | property 41 | `race_damage` | 50 |
| Animal damage | property 42 | `race_damage` | 50 |
| Attack speed | property 6 | `attack_speed` | 100 |
| Critical-hit chance | property 7 | `critical_chance` | 100 |
| Lifesteal | property 34 | `sustain` | 70 |

Only one race-damage prefix can appear on an item.

## Suffix pool

| Modifier | Source | Group | Weight |
| --- | --- | --- | ---: |
| Movement speed | property 8 | `movement_speed` | 80 |
| Stun proc | property 21 | `control_proc` | 70 |
| Double-gold chance | property 9 | `economy_bonus` | 60 |
| Extra-material chance | property 10 | `economy_bonus` | 60 |
| EXP gain | property 40 | `economy_bonus` | 60 |
| Mood recovery proc | property 45 | `need_recovery` | 40 |
| Stamina recovery proc | property 46 | `need_recovery` | 40 |
| Satiety recovery proc | property 47 | `need_recovery` | 40 |

Only one economy bonus and one need-recovery proc can appear on an item.

## Tier construction

Every active affix currently has eight modifier tiers (`T1`-`T8`). These are
item-level tiers, not difficulty tiers. The current level cap of 800 means
`T1`-`T8` are the only supported tiers; the schema can add `T9` when a future
content release raises the cap to 900. Package-backed values partition the
exact recovered positive range into the current tier bands:

```text
low(d)  = roundHalfEven(sourceMin + (sourceMax-sourceMin) * (d-1) / 9)
high(d) = roundHalfEven(sourceMin + (sourceMax-sourceMin) * (d+1) / 9)
```

`high` is clamped to `sourceMax`. The resulting ranges are rebuild-designed
item-level bands, not claims about the original game's tier table. A base
weapon with a higher `baseLevelCap` only makes higher tiers eligible; it does
not alter pool weights or modifier identity.

Flat attack has no package `gearProperty` row, so it is explicitly
rebuild-designed from the base-power curve:

| Tier | Required item level | Flat attack roll |
| ---: | ---: | ---: |
| 1 | 0-100 | 8-19 |
| 2 | 100-200 | 12-31 |
| 3 | 200-300 | 19-49 |
| 4 | 300-400 | 30-79 |
| 5 | 400-500 | 48-126 |
| 6 | 500-600 | 76-201 |
| 7 | 600-700 | 121-322 |
| 8 | 700-800 | 194-515 |

The lower bound is 12% of the tier's starting base power, rounded up; the
upper bound is 20% of its ending base power, rounded half-even.

## Excluded layers

Archangel, Demon Lord, class-skill properties, always-on unique properties,
Virtue support, and collection-set effects are not ordinary affixes. They stay
visible in the source catalog but cannot consume prefix/suffix slots or enter
this weighted pool until their own acquisition contracts are accepted.
