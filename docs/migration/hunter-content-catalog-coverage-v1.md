# Hunter Content Catalog Coverage v1

Date: 2026-07-26
Package: Evil Hunter Tycoon `1.411`

## Exact packaged catalogs

All rows below were decoded from the local Unity QuickSheet objects with raw
and decoded hashes recorded in `reverse-engineering/evidence/hunter-info-tables-v1.json`.

| Catalog | Rows | What is known | What it does not prove |
|---|---:|---|---|
| Basic skills | 10 | Names, localization, levels, cooldowns, effects and study costs | Per-Hunter learned state, execution order or icon binding |
| Sub-job skills | 40 | Job/sub-job/third-job/fourth-job keys and study/effect fields | Exact Hero-tree node mapping or runtime handler semantics |
| Growth properties | 15 | Definition rows, levels/cost/value fields and localization | Learned node IDs, prerequisites and final stat recomputation |
| Job traits | 69 | Tree positions/gates, costs, levels and values | Per-Hunter unlock state and modifier stacking |
| Riding pets | 21 | Pet definitions and localization | Ownership, rating roll, gear and Hunter assignment |
| Riding-pet skills | 3 | Definition rows and effect fields | Pet skill selection and live execution |
| Riding-pet traits | 6 | Definition rows and effect fields | Pet trait roll/level and live execution |
| Experience | 100 | Level/experience progression rows | Any account/Hunter current level or boosts |
| Gear | 671 | Definition, ratings, prices, crafting materials and modifier arrays | Instance roll, enhancement RNG and equipped ownership |
| Materials | 369 | Material definitions and prices | Per-Hunter quantities, drops and carrying rules |
| Consumables | 5 | Values, prices, cooldowns and crafting materials | Consumption transaction and need-state formula |
| Runes | 61 | Rune definitions and coded modifiers | Rune roll, socket state and authoritative creation result |
| Rune craft | 10 | Craft definition rows and requirements | Craft RNG and duplicate/quality rules |

## Runtime boundaries

Reflection evidence confirms per-instance state containers for `HunterData`,
`SkillData`, `ItemData`, `GearData` and `RidingPetData`, plus methods for job
skill lookup, gear stat access, random damage and rune creation. These establish
server-domain boundaries only; protected method bodies do not reveal formulas or
RNG distributions.

## Mining priority

1. Capture one authorized Hunter before/after skill study and skill upgrade.
2. Capture gear equip and one enhancement, including currency/material diff.
3. Capture one growth/trait unlock and one riding-pet assignment.
4. Capture one farm run to resolve loot ownership, sell pricing and need drains.
5. Correlate runtime skill/icon references only after a before/after state diff.

Each capture must record package version, device ABI, Frida versions, UTC time,
exact user action, and both state snapshots. A visual match alone is not a
semantic binding.

## Source policy

- Package and runtime evidence are authoritative for `1.411` structure.
- Official Zendesk content is a separately versioned public reference; see
  `docs/migration/zendesk-hunter-skill-catalog-v1.md`.
- Third-party internet material is corroboration only and must retain URL,
  access date and confidence label.
