# Original Reward And Progression Gold/Tax Chain V6

## Result

The exact top-level mutation order is `Reward -> CalVillTax -> PlusGold`.
`CalVillTax` receives the gold grant by reference, updates village tax state,
and reduces the grant before `PlusGold` can add it to `HunterData.money`.

Mechanically bound Reward operands include:

- positive `UserData.mBuildingGoldUp` in the early gold construction;
- `StatusData.FairyGoldUp` as
  `trunc((1 + FairyGoldUp * 0.01) * priorGrant)`;
- `StatusData.RamblePetGoldUp` with the same sequential form;
- positive `StatusData.RelicCollectionGoldUp` as an additive
  `trunc(RelicCollectionGoldUp * priorGrant)` contribution.

No direct access to `StatusData.CollectionGoldUp` at offset `0x588` is claimed.
Several static/event/table branches between the named modifiers remain
unresolved.

## Village tax

The tax candidate uses float32 arithmetic over two still-unnamed
`ObscuredFloat` operands and the current grant. The method truncates the whole
part toward zero, subtracts it from the grant, adds it to `UserData.tax`, and
accumulates the fractional part in `UserData.taxRemainder`. Whole units carried
out of the remainder are added to tax without a second subtraction from the
current grant. The final tax value is clamped to an unnamed static cap.

Golden vectors cover this fully bound arithmetic segment while accepting the
already-computed candidate and cap as inputs. They do not claim the missing tax
rate identities.

## Hunter money

`PlusGold` normally preserves the post-tax grant. When
`HunterData.revive > UserData.mStageLevel` and stage level is at most `3`, it
uses `trunc(postTaxGrant * 0.3)`. The float32 value `0.3` is read from the
packaged ARM64 `libil2cpp.so` at file offset `0xD2B404` as bytes `9a99993e`.
Only resulting grants of at least one are added to `HunterData.money`.

The product meaning of this early-stage scaling branch remains unresolved.

## Boundary

The two tax-rate identities, tax cap, and remaining Reward event/static table
branches prevent a full caller vector. Live reward integration remains blocked.

## Outputs

- `reverse-engineering/evidence/original-plus-gold-scaling-literal-package-v1.json`
- `reverse-engineering/evidence/original-reward-progression-gold-tax-chain-v6.json`
- `tools/analyze-original-reward-progression-pass6.py`
- `tools/tests/test_original_reward_progression_pass6.py`
