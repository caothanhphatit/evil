# Original Formula Migration Matrix v1

## Status rule

`Original` means the `1.411` package/runtime supplies the inputs, arithmetic,
branch order and rounding boundary. `Catalog only` means original table values
are loaded but runtime math is still incomplete. `Fixture` means the current
web rebuild deliberately uses temporary behavior and must not be presented as
the original formula.

`apps/server/src/simulation/original_combat.rs` now contains tested, disconnected
reference implementations for the exact `1.75` critical base, three-slot
monster attack-reduction stack, Hunter animation-time branch and monster attack-
delay branch. It also replays the recovered Hunter common damage tail, default
HP mutation and effect-54 attack-abort gate. They are not wired to outcomes
while their input semantics and alternate branches remain unresolved.

`apps/server/src/simulation/original_progression.rs` likewise contains the
exact generated EXP lookup and strict carry loop as a tested reference. It is
not wired into the live Hunter loop even though the original stored cap is now
proven as `99` (displayed level `100`), because the complete EXP modifier chain
remains unresolved.

`apps/server/src/simulation/original_gear.rs` contains disconnected reference
implementations for the exact Armor/Accuracy neutral pipeline, quality
multipliers, ties-to-even rounding, the recovered Seal Attack selector,
`GetFirstPercent` step schedule, and the exact structural Gear Damage pipeline.
The caller level adjustment and broader stat aggregation still block live use.

## Combat

| Formula boundary | Current web runtime | Recovery status | Required before replacement |
| --- | --- | --- | --- |
| Hunter base attack | Fixture profile value is subtracted directly from monster HP | `CalcDamage`, the D8/D10 selector and JobTrait(5) branch, final float64 SSA, S12/S13/D14, critical, Slayer/Rift, GearSet stack/S8 and job-specific Collection/Relic factors now exist in a disconnected core; ordinary attack is proven to call `getDamage(false,false,false)` | Resolve opaque modifier writers, the complete hit/miss chain and golden caller vectors before live use |
| Hunter critical chance | Fixture value is projected but not rolled in combat | Exact core recovered: `min(100, CalcCritical + enabledBonus)`, `Random.Range(0,100)`, hit when `roll < threshold`; outer gates/bonus writer remain unresolved | Resolve the conditional bonus writer and connect server RNG only after a golden caller vector |
| Hunter critical damage | Not applied | Exact disconnected core now covers the `1.75` base, named collection/pet/trait adds, three opaque Hunter fields, GearProperty rows `43/59/14`, target-race gate, float32 `1.8` temporary cap and final add | Resolve product names/writers for the opaque Hunter fields and complete caller vectors before live use |
| Hunter attack speed | Fixed recovery ticks | Exact `StatusData` producer and `HunterCtrl.FixedUpdate` countdown recovered: `AttackSpeed = WeaponSpeed * (1 + 0.01 * (Personal + Option + Rank - Guild - GUP[7] - RidingPet))`; `CalcAttackSpeed = max(0.25, AttackSpeed / (Fury > 1 ? Quicken + Fury + SpeedPotion : Quicken + SpeedPotion + Personal))`; `mAttackDelay` subtracts `Time.deltaTime`; animation block remains `composite > 1 ? 0.333/composite : 0.7` | Resolve the non-managed/serialized source of `DANCPPLMKIK`, product meaning of type-zero/535.0 buff paths, and exact gear-to-Spine weapon selection |
| Hunter accuracy vs monster evasion | Accepted tech debt: not applied; no synthetic hit-chance fallback | `GetGearAcc` proves a gear accuracy stat exists, while Pass 12 proves the normal Hunter `getDamage` roll is critical selection, not accuracy; the captured monster schema has no evasion field and no generic accuracy-vs-evasion check is present in the direct normal path | Trace `GetGearAcc` consumers, ignore-evasion properties, accuracy-reduction skills, and indirect projectile/skill delivery gates before enabling accuracy or claiming a universal always-hit rule |
| Monster armor | Original catalog value loaded but ignored | Damage-intake body captured; reduction/rounding unresolved | Resolve armor formula and min/max damage rules |
| Monster outgoing damage | Original catalog value divided by temporary `250` | Original damage catalog is exact; `UnitAttack` delay is `0.08 * max(field_572, 1)` but field semantics and full damage consumer remain pending | Remove compatibility divisor/fixed cadence only after damage, defense and field-writer vector recovery |
| Monster attack-reduction effects | Not applied | Three multiplicative slots and percent writers recovered | Resolve human effect IDs and confirm all outgoing-damage consumers |
| Hunter armor/defense | Fixture value projected but ignored | Feel bands/factors, 32 ordered pre-armor mutations, two-stage armor scratch, exact constant final factor `0.75`, minimum branch, first-shield absorption/spillover and HP floor are recovered | Resolve product-facing gates for obfuscated modifier sources and multi-shield ordering before live use |
| Hunter dodge/evasion | Fixture value projected but not rolled | Pass 18 recovers normal `CalcDodge` production/consumption, signed exclusive rolls, Meze bypass, pet fallback, and the `HunterCtrl.Damaged` early exit; direct Evil effect-54 remains a separate gate | Model the proven producer inputs, Unity PRNG stream, and unresolved alternate-mode writers before live connection |
| Damage variance | Not applied | Exact wrapping 30-value `RandDamage` stream recovered | Resolve every caller and where variance sits relative to armor/crit/skill |
| Skill damage/buff/debuff | Validation/cooldown/presentation only | Ten base-skill definitions and native dispatch captured; fifteen exact caller bodies now cover Blizzard-modified, plain-percent, decoded-ObscuredFloat-percent, internal-ObscuredInt-percent and affine-percent coefficient families | Recover the remaining 34 caller coefficients plus public row bindings, target rules, duration and stacking contracts |

## Rewards and progression

| Formula boundary | Current web runtime | Recovery status | Required before replacement |
| --- | --- | --- | --- |
| Ordinary material drop | Original slots and base thresholds are loaded | Independent `Range(1,10001)` slot rolls and `rawPercent * 10` threshold confirmed | Recover complete global/difficulty/pet/building modifier chain |
| Unique gear drop | Not complete | 61 catalog pools recovered | Resolve pool selection, quality/rating roll, modifiers and order |
| Monster EXP | Original catalog amount granted directly | `GetNeedExp(revive, level)` and strict carry loop recovered; full gain modifier incomplete | Resolve the global max-level source and ordered EXP multiplier/event branches |
| Monster gold | Original catalog amount granted directly | Base catalog exact | Resolve gold gain modifier chain and rounding |
| Hunter gold/tax settlement | Fixture flow does not reproduce original modifier/tax chain | Reward order, building/fairy/ramble-pet/relic segments, tax fractional carry, post-tax `PlusGold`, early-stage float32 `0.3` scaling and money sink are recovered as disconnected references | Resolve two tax-rate operands, tax cap and remaining event/static branches before live use |
| Level-up | Live loop preserves the exact strict `remaining > 0` carry and cap behavior, but still advances a temporary fixture threshold because per-Hunter `revive` is not bound | Exact stored cap `99` / displayed cap `100`, row `level+1`, revive column, strict carry, multi-level carry and cap discard are recovered | Bind the authoritative per-Hunter `revive` value and remaining EXP operands before replacing the temporary threshold with `GetNeedExp` |
| Post-cap secondary progression | Not implemented as original behavior | Exact `75/100/125` selector is recovered for revive `5`, level `99`, stage level and downstream `mBuildingSoulUp` access | Resolve the downstream formula, persistence target and product-facing meaning before live use |
| Gear stat generation | Dummy gear/profile values | Armor/Accuracy and Gear Damage structural pipelines, AdminGearData formula fields, `GetFirstPercent`, quality multipliers and ties-to-even rounding recovered | Resolve the caller level adjustment, option enum meanings, enhancement, runes and traits before live aggregation |

## Authority target

Every completed formula is implemented once in the Rust simulation. The Pixi
client receives authoritative outcomes and only interpolates movement and
plays recovered presentation. Browser-provided damage, crit, loot, EXP, gold,
cooldown completion or RNG results are never trusted.
