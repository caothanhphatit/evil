# Original Reward And Progression Level Domain V4

## Result

A targeted Android API 35 capture resolves the live `1.411` stored-level cap
operand consumed by `HunterCtrl.PlusExp`. The method loads an `ObscuredInt`
from the referenced class static-fields block at offset `0xE8`. Its captured
key `0x446E6E2B` and hidden value `0x446E6E48` decode by XOR to `99`.

The native cap branch compares `HunterData.level` with that decoded value and
skips the EXP mutation when the stored `level >= 99`. The level-up presentation
uses `HunterData.level + 1`, so this corresponds to a maximum displayed value of
`100`, not a displayed level of `99`. Consequently, the normal
`GetNeedExp` current-level domain is `0..98`. Since that method reads packaged
row `currentLevel + 1`, this path consumes rows `1..99`; row `0` is not used by
this path. The semantic product name of the static holder remains unresolved.

## Separate 75/100/125 branch

The constants `75`, `100`, and `125` in `PlusExp` do not define alternative
level caps. Runtime schema offsets and the ordered native comparisons recover
the branch as:

```text
if revive == 5 and hunterLevel == 99 and stageLevel >= 6:
    value = 100 if stageLevel == 6 else 125
else:
    value = 75
```

The result continues into code reading `UserData.mBuildingSoulUp`, so it is a
separate secondary-progression path. A more specific product-facing meaning is
not assigned without an exact writer/consumer binding.

Confirmed field offsets are:

- `HunterData.level`: `0x88`
- `HunterData.revive`: `0xC4`
- `UserData.mStageLevel`: `0x5D8`
- `UserData.mBuildingSoulUp`: `0x9B0`

## Evidence boundary

This pass resolves the global cap value and level-domain branch only. Remaining
unnamed singleton/static EXP additions, fairy/pet/event gold operands, and
their complete semantic order remain unresolved. No live Hunter or account
values were captured and no rebuild runtime behavior changed.

## Outputs

- `reverse-engineering/evidence/original-plus-exp-max-level-static-api35-v1.json`
- `reverse-engineering/evidence/original-reward-progression-level-domain-v4.json`
- `tools/analyze-original-reward-progression-pass4.py`
- `tools/tests/test_original_reward_progression_pass4.py`

```sh
python3 tools/analyze-original-reward-progression-pass4.py
python3 -m unittest tools.tests.test_original_reward_progression_pass4
```
