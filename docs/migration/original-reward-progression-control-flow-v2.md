# Original Reward And Progression Control Flow V2

## Confirmed call order

Exact ARM64 direct calls in `HunterCtrl.Reward/2` occur in this order:

1. `HunterCtrl.PlusExp/2` at method offset `0xFEC`
2. `HunterCtrl.CalVillTax/1` at `0x1478`
3. `HunterCtrl.PlusGold/1` at `0x16DC`

This proves mutation order, not the complete semantic formula constructed
between calls.

`HunterCtrl.RewardMetrial/5` contains 50 integer `Random.Range` calls, 17 calls
to `LDHAEMDJCFF/5`, and six calls to `GHPHHEFFNKN/2`. The ordinary material
loop reaches `LDHAEMDJCFF/5` at offset `0xEB4`, passing the recovered material
index/count and presentation arguments. Consequently this helper is a general
reward-emission boundary and must not be labeled unique-gear-only.

`LDHAEMDJCFF/5` itself calls `GHPHHEFFNKN/2` once and uses exact ranges
`Range(0,20)` and `Range(0,3)`. `GHPHHEFFNKN/2` has fifteen RNG sites: nine
mechanically constant `Range(0,100)` sites, one upper-bound-100 site whose
minimum is control-flow merged, two `Range(0,1000)` sites, and three
`Range(0,10000)` sites.

## RewardMetrial RNG families

The complete method contains both fixed and dynamic ranges. Confirmed fixed
families include the ordinary `Range(1,10001)`, four `Range(1,1001)` sites,
three `Range(1,100001)` sites, one `Range(1,1500001)` site, multiple small
selection ranges, and exact index windows such as `[348,351)`, `[316,319)`,
`[166,171)`, and `[171,176)`. Fourteen sites retain a dynamic or
control-flow-merged bound in the mechanical analysis.

These values establish RNG outcome counts and ordering sites only. They do not
establish that a particular range means unique gear, material rarity, event
reward, costume, or another product concept.

## Unique gear blocker

Runtime schema confirms:

- `AdminEvilData.uniqueLevel`, `metIdx`, `metCount`, `metPercent`, `exp`, `gold`
- `AdminDropUniqueGearData.index`, `dropRange`, `dropCut`, `gearType`,
  `gearIndex`, `gearPercent`

The current direct-call and instruction evidence does not yet mechanically
prove `uniqueLevel -> AdminDropUniqueGearData` pool lookup, `dropCut` evaluation
order, `gearPercent` denominator, or type/index selection order. Those fields
must remain disconnected from live Rust formulas until their object-flow
registers or controlled outcomes are captured.

## Outputs

- `reverse-engineering/evidence/original-reward-progression-callgraph-v2.json`
- `tools/analyze-original-reward-progression-pass2.py`
- `tools/tests/test_original_reward_progression_pass2.py`

```sh
python3 tools/analyze-original-reward-progression-pass2.py
python3 -m unittest tools.tests.test_original_reward_progression_pass2
```
