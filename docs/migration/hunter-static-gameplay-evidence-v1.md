# Hunter static gameplay evidence v1

Date: 2026-07-26

The original catalog pass mined packaged QuickSheet data and Android API 30
reflection. A follow-up API 35 capture now also recovers selected decrypted
native bodies and one controlled runtime value table. The machine-readable
native record is `reverse-engineering/evidence/original-native-combat-runtime-v1.json`.

## What is exact

The packaged data has complete definitions for 10 basic skills, 40 sub-job
skills, 15 growth properties, 69 job traits, 21 riding pets, 3 pet skills, 6
pet traits, 100 experience rows, 671 gear definitions, 369 materials, 5
consumable definitions, 61 runes, and 10 rune-craft definitions. Gear rows contain
rating-specific values, crafting material IDs/quantities, buy prices, and
modifier arrays. Consumable rows contain level-specific values, prices,
cooldowns, and crafting materials. These are content definitions, not a
particular Hunter's owned state.

## Runtime boundaries recovered

Reflection confirms `HunterData` (109 fields), `GearData` (29), `SkillData`
(4), `ItemData` (5), `RidingPetData` (11), and `HunterManager` (5). The method
surface includes `AddHunter`, `AddWaitHunter`, `FixRangeOverBodyIndex`,
`GetHunterDefaultMoney`, `GetGearDamage`, `GetGearArmor`, `GetGearAcc`,
`RandDamage`, `GetHunterSkillIcon`, `GetJobSkillDataIndex`, `GearAddData`,
`CreateRunes`, and `GetNewRunes`. Their existence and signatures are evidence
of boundaries. `RandDamage` is no longer unresolved: the decrypted method uses
a fixed 30-entry multiplier stream and advances a wrapping index. The exact
stream is `0.91, 1.00, 1.10, 0.92, 0.91, 1.10, 1.03, 1.06, 1.13, 0.95,
1.00, 0.92, 1.06, 0.98, 1.13, 0.95, 1.10, 1.10, 0.92, 1.05, 1.03, 0.99,
1.10, 1.10, 0.90, 0.90, 1.10, 0.90, 1.00, 1.10`. `GetNeedExp` is also
confirmed as a job-column lookup into the packaged 100-row EXP table. The base
combat formula, armor reduction, crit/dodge, and `GetGearDamage` dependencies
remain unresolved.

## BE handoff

The server can safely persist the definition catalogs and typed owned
collections, then expose calculation/mutation evidence states as unavailable.
The server contains the recovered multiplier stream as deterministic integer
hundredths, but it is intentionally not wired into fixture combat until the
surrounding damage formula is recovered. Do not implement a guessed DPS formula, roll probabilities, upgrade costs,
equip command, or skill-icon filename ordering. The complete machine-readable
record is `reverse-engineering/evidence/hunter-static-gameplay-v1.json`.

## Remaining controlled-capture targets

Capture one authorized ARM64 Hunter before/after exactly one skill study, gear
equip, gear enhancement, and craft action. Record the command intent, the
authoritative state diff, currency/material/timer changes, and before/after
icon references. A separate generation trace is required to recover sex,
grade, body, characteristic, and stat RNG distributions.
