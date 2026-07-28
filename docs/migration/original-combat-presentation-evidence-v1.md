# Original Combat Presentation Evidence v1

## Scope

This pass recovers the field-combat presentation contract from the supplied
Evil Hunter Tycoon `1.411` package. It does not infer hit, dodge, critical, or
damage outcomes from an HP delta. The server emits a typed presentation only
after its authoritative combat path has selected that outcome.

Primary evidence:

- `reverse-engineering/evidence/original-native-combat-presentation-schema-api35-pass15.json`
- `reverse-engineering/evidence/original-native-combat-presentation-methods-api35-pass15.json`
- `reverse-engineering/evidence/original-native-hit-miss-pass12.json`
- `game-assets/extracted/joined_unity_files/sharedassets1.assets`

## Recovered `DamageCtrl` contract

`DamageCtrl.Show(Int32, Int64, Vector3, Int32, Boolean)` stores the display
type, amount, position, selector and position flag. The recovered type jump
table and initialized localization strings prove these combat-facing rows:

| Type | Recovered presentation |
| --- | --- |
| `0` | Plain damage number in `#DE3232` |
| `1` | Baseline normal damage; the default selector formats the number in `#AF70E0` |
| `2` | Critical damage; prepends localized `status_6` (`CRIT`) in `#FFD228`, size `20`, then a newline and the damage number |
| `3` | Localized `damagectrl_0` (`Evade`) in `#81F7F3` |
| `13` | Localized lifesteal presentation |
| `15` | Localized invulnerable presentation |
| `16` | Localized `damagectrl_5` (`Miss`) in `#D43D3D` |
| `17` | Green recovery presentation containing a white signed percentage |

Types `4`, `10`, `12`, and `14` are EXP, ELE, Penalty, and SOUL
presentations. Types `5..9` are purchase/item-name variants. They are not
modeled as hit results.

Pass 12 independently proves that the outgoing Hunter `getDamage` branch uses
result discriminator `2` when the exact critical roll succeeds; the baseline
path retains discriminator `1`. The effect-54 abort path calls
`DamageManager.Show(16, 0, ..., 0, false)`, which now closes its presentation
as `Miss`. This still does not prove a generic `StatusData.CalcDodge` formula.

The final Boolean position flag adds exactly `+60` to the display Y coordinate.
`HunterCtrl.Damaged` independently proves that ordinary positive incoming
damage calls the display with type `0`; non-positive damage is forwarded as
`1` through the same red path. Type `15` is its separate Invulnerable branch.

## Serialized presentation assets

The packaged `Damage` prefab is `50 x 20`, uses `DefaultFont2` at outer font
size `32`, enables rich text, centers the text, and contains a `DamageCtrl` plus
an Outline component. Its serialized default text is the Korean critical label
at size `20`, followed by a newline and `1000`, matching the native type-2
formatting contract.

The separate `Dodge` sprite animation lasts exactly
`1.0166666507720947` seconds and uses the frame order:

```text
0007_0 -> 0007_1 -> 0007_2 -> 0007_3 -> 0007_2 -> 0007_1 -> 0007_0
```

`DodgeMent` has a separate localized label sprite. This animation belongs to
the actor-side Dodge presentation and must not be substituted for
`DamageCtrl` type `3`, whose recovered text is `Evade`.

## Rebuild binding

Protocol v24 adds monotonic, server-owned combat presentation events with a
source entity, target entity, typed result, and nullable amount. The Pixi world
deduplicates by event sequence and renders the recovered font sizes, labels,
and colors above the authoritative target actor.

The current temporary combat loop emits only outcomes it actually resolves.
Incoming and outgoing normal damage are live with their distinct red and purple
presentations. Critical, Evade, and Miss renderer paths are bound and
tested, but must not be emitted by invented FE checks. Critical emission waits
for the original formula core to be safely connected; Evade waits for the
global `CalcDodge` consumer; Miss may be emitted only by the proven effect-54
gate or another source-confirmed miss path.

Pass 15 closes the generated `DamageManager` coroutine envelope. The text rises
through offsets `0 -> 5 -> 15 -> 20 -> 35` at `20`, `120`, `80`, then `20`
units/second. Its continuous ideal duration is `1.1458333333s`; the original is
`WaitForFixedUpdate`-quantized. While active, x/y scale decreases by
`deltaTime / 3`. The web renderer continuously interpolates this exact phase
envelope because browser render cadence differs from Unity FixedUpdate, and it
keeps the captured hit position rather than following the target actor.
