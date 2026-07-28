# Hunter Skills: Official Reference

This document is the implementation-facing index of the Vietnamese Evil
Hunter Help Center skill references captured on 2026-07-26. It is labelled
`Official-reference`: the live Help Center can describe a release newer than
the supplied APK `1.411`, so it does not replace package-confirmed QuickSheet
data.

## Learning and execution

The article [Kỹ Năng Thợ Săn](https://evilhunter.zendesk.com/hc/vi/articles/360039886791)
states that a Hunter learns skills from the Mayor at the Academy, can be told
to learn through the conversation flow (`Học > Kỹ Năng`), must have enough
gold, and automatically uses learned skills at the battle field. This is a
workflow reference, not proof of the rebuild's current command or cost model.

## Hero trees

The five official class articles expose two branches per job. Every branch has
16 published entries: one branch root, shared modifiers/passives, and two
branch-specific final nodes. The complete Vietnamese descriptions, numeric
ranges, cooldowns, trigger conditions, PVP clauses, positions, and attachment
URLs are preserved in
[`zendesk-hunter-skills-v1.json`](../../reverse-engineering/evidence/zendesk-hunter-skills-v1.json).

| Job | Branch roots | Published branch heading caveat |
|---|---|---|
| Berserker | `Blade Finish` / `Commanding Shout` | None observed |
| Paladin | `Token of Punishment` / `Light's Grace` | First heading is published as `Xạ Thủ Sao` |
| Ranger | `Vengeance` / `Falling Star` | None observed |
| Sorcerer | `Mana Charge` / `Elemental Force` | Second heading is published as `Người đối lập` |
| Dark Knight | `Hyper Blitz` / `Mistilteinn` | Article uses `Bảng kỹ năng` wording |

The two heading anomalies are retained verbatim in the evidence and resolved
only through the root node and job path. They must not be silently corrected in
source evidence.

## Package comparison boundary

The APK QuickSheet catalog contains 10 basic and 40 class-change skill rows.
The ten Hero roots match package `subJobSkills` rows `124, 125, 126, 127, 128,
129, 130, 131, 138, 139` by job/fourth-job semantics and localized naming.
This is a definition match, not a runtime learned-state or icon binding.

For the other 40 rows, the official prose sometimes mentions the English skill
name (for example `Multishot`, `Death Coil`, `Blessing`, `Battle Shout`,
`Aura Blade`, `Hands of God`, `Song of Peace`, `Summon Familiar`, `Polymorph`,
`Summon Phoenix`, `Meteor`, `Mystic Arrow`, and `Nightmare`). These are
dependency/name references only. Vietnamese translations and legacy naming
are inconsistent, so no remaining Hero-node-to-package-row or icon mapping is
promoted without a controlled runtime/UI capture.

## Server implementation boundary

- Seed official trees as versioned content separate from the APK `1.411`
  catalog.
- Validate Hunter job path, prerequisites, study level, cost, and learned state
  on the authoritative server once those fields are supported by evidence.
- Treat every numeric range as a documented range, not a recovered formula;
  preserve PVP reductions and trigger text as content until combat semantics are
  independently resolved.
- Keep icon binding, runtime unlock state, and exact effect ordering explicitly
  unresolved where the evidence does not prove them.

## Sources

- [Berserker](https://evilhunter.zendesk.com/hc/vi/articles/40578495995545-C%C3%A2y-K%E1%BB%B9-n%C4%83ng-l%E1%BB%9Bp-Anh-h%C3%B9ng-Berserker)
- [Paladin](https://evilhunter.zendesk.com/hc/vi/articles/40579021862425-C%C3%A2y-K%E1%BB%B9-n%C4%83ng-l%E1%BB%9Bp-Anh-h%C3%B9ng-Paladin)
- [Ranger](https://evilhunter.zendesk.com/hc/vi/articles/40578790147353)
- [Sorcerer](https://evilhunter.zendesk.com/hc/vi/articles/40579196882841)
- [Dark Knight](https://evilhunter.zendesk.com/hc/vi/articles/54044060240793)
