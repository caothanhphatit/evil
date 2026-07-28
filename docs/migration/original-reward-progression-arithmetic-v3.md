# Original Reward And Progression Arithmetic V3

## Result

Pass 3 normalizes every ARM64 arithmetic instruction, in native order, for
`HunterCtrl.PlusExp`, `Reward`, `CalVillTax`, and `PlusGold`. The normalized
record contains 32, 99, 10, and three operations respectively. `PlusGold` ends
with the exact sequence `scvtf -> fmul -> fcvtzs`; EXP also terminates through
`fcvtzs`, so both conversions truncate toward zero.

This is a complete instruction trace, not a complete semantic formula. The
known EXP inputs remain the building EXP base, EXP scroll, EXP box, EXP gem
pack, costume, collection, revive-wisdom, and gear-property contributions.
Several singleton/static operands still lack mechanical names. Gold arithmetic
is ordered across `Reward -> CalVillTax -> PlusGold`, but not every operand can
yet be assigned to fairy, pet, building, event, or tax semantics.

## Unique gear trace

Register/object-flow checkpoints now bind exact `AdminEvilData` row accesses
inside `RewardMetrial` for:

- `metIdx` at offsets including `0x2B0`, `0x300`, `0x48C`, `0xC70`, `0xE64`
- `metPercent` at `0x4EC`
- `metCount` at `0xD90` and `0xDEC`
- monster `type` at `0xFD8`, `0x12C0`, `0x23AC`, and `0x4D68`

The same mechanically verified row-flow does not yet bind a read of
`AdminEvilData.uniqueLevel`. No object register is proven to hold an
`AdminDropUniqueGearData` row. Therefore the following remain deliberately
unset in evidence:

- `uniqueLevel -> pool index`
- `dropCut` evaluation order
- `gearPercent` denominator
- gear type/index RNG selection order

The package schemas and 61 pools remain exact catalog data, but using array
order as a replacement for missing native linkage would be an invented rule.

## Outputs

- `reverse-engineering/evidence/original-reward-progression-arithmetic-v3.json`
- `tools/analyze-original-reward-progression-pass3.py`
- `tools/tests/test_original_reward_progression_pass3.py`

```sh
python3 tools/analyze-original-reward-progression-pass3.py
python3 -m unittest tools.tests.test_original_reward_progression_pass3
```
