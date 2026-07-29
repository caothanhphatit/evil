# Hunter autonomous service evidence v1

## Implemented rebuild boundary

- Idle Hunters automatically sell only material quantities still requested by
  the Trading Post. The authoritative transaction debits town gold, credits the
  selling Hunter, moves stock to town inventory and decrements the remaining
  order quantity.
- Inn (`build_9`), Infirmary (`build_12`), Restaurant (`build_13`) and Tavern
  (`build_19`) service recipes are loaded from the normalized package tables.
- Crafting a service product consumes authoritative town resources and adds
  durable building stock.
- Starting a service consumes one stock unit and debits Hunter gold. Completion
  restores the bound gauge and credits the payment to town gold.

These service transactions currently require a player/client command. The
server does not yet decay stamina, satiety or mood, choose a service product for
an exhausted Hunter, or route that Hunter to the service building.

## Accepted rebuild healing policy

The original autonomous HP threshold is still not recovered. The rebuild uses
the user-specified deterministic rule until a native-body/live-value capture
supersedes it:

1. When `current_hp * 100 < maximum_hp * 10`, the authoritative server checks
   the Hunter's owned consumables.
2. If a Healing Potion is available, the server consumes one before changing
   the Hunter's hunting assignment. The highest owned potion tier is selected
   deterministically, healing is capped at maximum HP, and the recovered
   20-second potion cooldown is persisted with the Hunter.
3. If no Healing Potion is owned, the server clears the farming-region
   assignment and returns the Hunter to a commandable idle state in town. The
   profile may present `returning_for_infirmary` while the world actor walks
   back, but no durable service task is claimed or left stuck.

The Healing Potion identity and restoration values are package-confirmed:
`consumable:0`, levels `0..7`, with `keepValue` values `4000`, `12000`,
`32400`, `77800`, `163300`, `294000`, `1562500`, and `9375000`. The 10%
decision threshold and highest-tier-first inventory policy are rebuild product
rules, not claims about the original game's recovered AI.

Automatic Infirmary product selection, stock reservation, payment and service
start remain unresolved. They are not synthesized by this policy.

## Static original-game evidence

`StatusData` exposes the exact runtime fields:

| Field | Type | Offset |
| --- | --- | ---: |
| `MinusHungry` | `ObscuredFloat` | `248` |
| `MinusFeel` | `ObscuredFloat` | `268` |
| `MinusTire` | `ObscuredFloat` | `288` |

Its static tuning arrays are `MinusHungryValues`, `MinusFeelValues` and
`MinusTireValues`. Their live values and selection rules are not captured.

`HunterCtrl` exposes the following autonomy/service candidates in package
version `1.411`:

| Method | Token | Return |
| --- | ---: | --- |
| `HpSpeakClick` | `100686707` | `Void` |
| `TireSpeakClick` | `100686743` | `Void` |
| `FeelSpeakClick` | `100686747` | `Void` |
| `TireComeBack` | `100686796` | `Void` |
| `HungrySpeakClick` | `100686804` | `Void` |
| `FeelChange` | `100686813` | `Boolean` |
| `FoodEat` | `100686946` | `Boolean` |

Signatures prove method identity only. They do not yet prove thresholds,
priority between simultaneous needs, recipe choice, affordability behavior,
queue retry, gauge decay cadence or navigation destinations.

## Required runtime capture

On the next authorized stable ARM64 session, capture exact native bodies for
the seven methods above together with before/after values for `nowHp`,
`nowHungry`, `nowTire`, `nowFeel`, Hunter money, selected building/product and
building stock. Capture separate cases for one exhausted gauge, multiple
exhausted gauges, empty stock, insufficient Hunter gold and a full service
queue.

No Android target was connected during this audit, so no new native-body or
live-value claim is made here.
