# Original Reward And Progression PlusExp Chain V5

## Result

Pass 5 binds the exact known `HunterCtrl.PlusExp` accumulator order. It also
corrects the product-facing interpretation of the pass-4 cap: the captured
static value `99` caps the stored `HunterData.level`, while the level-up UI and
mission threshold use `HunterData.level + 1`. The maximum displayed value
observed on this path is therefore `100`.

The known accumulator order is:

1. Use positive `UserData.mBuildingExpUp`, otherwise start at `1.0`.
2. Add `1.0` when `TimeData.expScroll` is true.
3. Add `0.5` when `TimeData.BoxExp` is true.
4. Add `1.0` when `UserData.ExpGemPack_Active` is true.
5. When an unnamed singleton Boolean is true, add float32 `0.2` from module
   literal address `0xD2AAB8`.
6. For `HunterData.reviveWisdom` values `1..5`, add the selected unnamed table
   value in hundredths.
7. Apply the exact `StatusData.GearProperty[40]` delta branch in hundredths.
8. Add `StatusData.CostumeExpUp` only when
   `GameManager.IsCostumeExpUp(HunterData.costumeIndex)` succeeds.
9. Add positive `StatusData.CollectionExpUp`.
10. Apply an area-dependent unnamed static-table branch.
11. Clamp a negative accumulator to zero.

Three exact native sites then truncate the modified incoming grant toward zero.
Two use `accumulator * incomingGrant`; the third multiplies by float32 `0.2`.
That raw value is cross-verified against the same package literal already
captured by the damage-tail analyzer. The stage/revive/area comparisons are
mechanically preserved, but their remaining static IDs and branch product
meanings are unresolved. A complete original formula is therefore not
published or connected to live rewards.

## Level-up side effects

The mutation loop increments the stored obscured level, resets EXP to zero, and
continues carrying only positive overflow. After the loop it adds the remaining
grant to stored EXP. When at least one level was gained and the method's Boolean
parameter is true, the native body additionally:

- runs presentation and status-refresh calls;
- formats level text from stored level plus one;
- checks `UserData.currentMissionIdx` and displayed level `>= 100` before an
  unresolved mission call chain;
- evaluates the separate `75/100/125` secondary-progression branch;
- conditionally reads the four job fields, mutates `HunterData.DSoul`, and emits
  notification code `14`.

The exact helper/table meanings in those side effects remain unresolved and are
not renamed by inference.

## Outputs

- `reverse-engineering/evidence/original-reward-progression-plus-exp-chain-v5.json`
- `tools/analyze-original-reward-progression-pass5.py`
- `tools/tests/test_original_reward_progression_pass5.py`

```sh
python3 tools/analyze-original-reward-progression-pass5.py
python3 -m unittest tools.tests.test_original_reward_progression_pass5
```
