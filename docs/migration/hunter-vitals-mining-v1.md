# Hunter vitals mining v1

## Evidence boundary

The packaged/runtime schema confirms four independent Hunter vital pairs. They
are not a single percentage field:

| Vital | Maximum field | Current field | HunterData offset (maximum/current) |
| --- | --- | --- | ---: |
| HP | `hp` (`ObscuredLong`) | `nowHp` (`ObscuredLong`) | 328 / 360 |
| Mood/feel | `feel` (`ObscuredFloat`) | `nowFeel` (`ObscuredFloat`) | 392 / 412 |
| Satiety/hunger | `hungry` (`ObscuredFloat`) | `nowHungry` (`ObscuredFloat`) | 432 / 452 |
| Stamina/tire | `tire` (`ObscuredFloat`) | `nowTire` (`ObscuredFloat`) | 472 / 492 |

Source: `hunter-info-runtime-schema-android-api35-v1.json`, class
`HunterData`. The values are stored as ACTk obscured numeric types; no live
value capture was available in this pass.

## Maximums and randomization

The exact packaged base-job table provides rolled HP bounds, not a fixed 100:

- Berserker, Paladin and DarkKnight: base HP `6000..6200`;
- Ranger and Sorcerer: base HP `5600..5800`;
- second/third job rows raise those intervals to `+200`/`+400`.

The table decoder does not contain `feel`, `hungry` or `tire` generation
bounds. Their maxima are nevertheless first-class per-Hunter fields and are
modified by the growth system, so treating all four maxima as an unconditional
`100` is not evidence-backed. The exact constructor/generator roll and any
initial current-value rule remain unresolved until a stable ARM64 value capture.

## Growth and consumption evidence

The exact 15-row `growupProperty` table binds the relevant properties:

| Row | Property | `upValue` |
| ---: | --- | ---: |
| 0 | HP increase | `0.15` |
| 1 | Mood increase | `0.30` |
| 2 | Satiety increase | `0.30` |
| 3 | Stamina increase | `0.30` |
| 12 | Mood consumption decrease | `0.30` |
| 13 | Satiety consumption decrease | `0.30` |
| 14 | Stamina consumption decrease | `0.30` |

These are percentage-style growth inputs, not a proof that the gauge itself is
displayed as a percentage. The native `HunterCtrl.HuntingAttackAction`
boundary is confirmed to mutate `HunterData.nowHungry`, proving hunger is
consumed during attacks. The decrement amount, cadence, and the corresponding
feel/tire decay writers are not recovered.

`StatusData` also exposes `MinusHungry`, `MinusFeel`, and `MinusTire` plus the
static arrays `MinusHungryValues`, `MinusFeelValues`, and `MinusTireValues`.
Their live contents and selection logic remain unresolved.

## Recovery/services

The serialized service products are complete for the four known buildings:

- `build_9` Inn -> stamina;
- `build_12` Infirmary -> HP;
- `build_13` Restaurant -> satiety;
- `build_19` Tavern -> mood.

Each has product rows at required levels `0..6`, service time `10,000 ms`,
and recovered effect/payment values. For example, level-zero rows restore
`140` (Inn/Restaurant/Tavern) or `9,216` (Infirmary) and cost `90` Hunter gold;
level-six rows restore `150,323` or `33,750,000` and cost `99,000`. These are
product effects, not proof that a Hunter's maximum is that value.

The rebuild currently consumes town stock, charges Hunter gold, restores the
bound current value with a maximum clamp, and credits town gold after a
player-issued service command. Autonomous decay, service selection, walking
to a building, queue retry and affordability behavior are intentionally not
implemented because their native bodies/live values remain unresolved. See
`hunter-autonomous-service-evidence-v1.md` for the required capture matrix.

## Non-claims

- No exact RNG distribution for HP or any vital maximum is claimed.
- No exact per-action decay formula or elapsed-time cadence is claimed.
- No threshold/priority rule for simultaneous exhausted vitals is claimed.
- No original-game mapping is inferred from the current rebuild fixture values.
