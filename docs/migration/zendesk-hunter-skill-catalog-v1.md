# Zendesk Hunter Skill Catalog v1

Date: 2026-07-26
Authority: Official Evil Hunter Tycoon Help Center, Vietnamese locale
Package baseline used for comparison: `1.411`

## Scope

The official Help Center currently exposes five Hero-class skill-tree
articles:

| Job | Article ID | Hero branches | Parsed entries |
|---|---:|---:|---:|
| Berserker | 40578495995545 | 2 | 32 |
| Paladin | 40579021862425 | 2 | 32 |
| Ranger | 40578790147353 | 2 | 32 |
| Sorcerer | 40579196882841 | 2 | 32 |
| Dark Knight | 54044060240793 | 2 | 32 |

The captured artifact contains 10 branches and 160 published tree entries,
including root skills, passive nodes, numeric ranges, cooldowns, duration,
PVP modifiers, trigger conditions and official Help Center attachment URLs.
The machine-readable source is
`reverse-engineering/evidence/zendesk-hunter-skills-v1.json`.

## Package comparison

The supplied package's QuickSheet catalogs contain 10 basic skills and 40
sub-job skill definitions. The official Hero trees are a newer/public content
surface and are not a drop-in replacement for the package catalog.

All ten Hero branch-root names can be matched to package `subJobSkills` rows by
job/branch semantics and localized names:

`124, 125, 126, 127, 128, 129, 130, 131, 138, 139`.

The remaining package rows are mentioned in official node descriptions but do
not have a safe direct node-to-row binding from prose alone. Do not bind icons
by array order. The package's only exact basic icon bindings remain Fury to
`skill_h1_01` and War Cry to `skill_h1_02`.

## Data fields worth preserving

Each published node should retain:

- source article ID and URL;
- job and branch identity;
- row/column tree position;
- Vietnamese display name;
- full description text;
- parsed numeric expressions as raw strings;
- official attachment URL;
- package binding status and evidence basis.

Numeric expressions such as `3500~8000%`, `20 giây`, `6~15%`, PVP reductions,
trigger conditions and stack limits are documentation evidence. They are not
yet authoritative Rust formulas for the `1.411` rebuild until their release
compatibility and execution semantics are confirmed.

## Published anomalies

- A Paladin branch heading is published as “Xạ Thủ Sao”, while its root node
  resolves to the semantic `Token of Punishment` branch.
- A Sorcerer branch heading is published as “Người đối lập”, while its root
  node resolves to `Elemental Force`.

Use the root node and package semantic match for internal identity, while
retaining the original heading as source text.

## Implementation policy

- Store official trees as versioned content, separate from package `1.411`
  definitions.
- Let the server validate job-path eligibility, prerequisite nodes, study
  level, cost, current skill state and content release.
- Let the web client render descriptions and server-projected availability; it
  must not calculate damage, cooldown outcomes or PVP modifiers.
- Keep unmatched icons and skill handlers explicitly unresolved.
- Treat third-party guides as corroboration only; they cannot override package
  evidence or official Help Center text without a reviewed release decision.

## Sources

- [Evil Hunter Help Center](https://evilhunter.zendesk.com/hc/vi)
- [Berserker](https://evilhunter.zendesk.com/hc/vi/articles/40578495995545-C%C3%A2y-K%E1%BB%B9-n%C4%83ng-l%E1%BB%9Bp-Anh-h%C3%B9ng-Berserker)
- [Paladin](https://evilhunter.zendesk.com/hc/vi/articles/40579021862425-C%C3%A2y-K%E1%BB%B9-n%C4%83ng-l%E1%BB%9Bp-Anh-h%C3%B9ng-Paladin)
- [Ranger](https://evilhunter.zendesk.com/hc/vi/articles/40578790147353-C%C3%A2y-k%E1%BB%B9-n%C4%83ng-L%E1%BB%9Bp-Anh-h%C3%B9ng-X%E1%BA%A1-th%E1%BB%A7)
- [Sorcerer](https://evilhunter.zendesk.com/hc/vi/articles/40579196882841-C%C3%A2y-k%E1%BB%B9-n%C4%83ng-L%E1%BB%9Bp-Anh-h%C3%B9ng-Ph%C3%A1p-s%C6%B0)
- [Dark Knight](https://evilhunter.zendesk.com/hc/vi/articles/54044060240793-B%E1%BA%A3ng-k%E1%BB%B9-n%C4%83ng-ngh%E1%BB%81-anh-h%C3%B9ng-Hi%E1%BB%87p-s%C4%A9-B%C3%B3ng-%C4%91%C3%AAm)
- [Hunter skills (learning workflow)](https://evilhunter.zendesk.com/hc/vi/articles/360039886791)
