# Weapon Core Data (v1)

This document records the implemented weapon-core data boundary for the web
rebuild. The generated release is
`evil-hunter-rebuild-v1.weapon-core-v1`.

## Generated artifacts

- Source generator: `tools/generate-rebuild-weapon-core.py`
- Canonical release JSON:
  `packages/content/releases/evil-hunter-rebuild-v1/weapon-core-catalog.json`
- PostgreSQL import: `infra/db/core_game/002_rebuild_weapon_core.sql`
- Import entrypoint: `infra/db/core_game/init.sql`

The generator reads the accepted rebuild weapon ladder and decoded QuickSheet
evidence. Generated files are deterministic and must not be hand-edited.

## Imported data

| Object | Count | Evidence boundary |
| --- | ---: | --- |
| Difficulty bands | 8 | rebuild-designed |
| Rarity slot budgets | 4 | rebuild-designed |
| Weapon bases | 40 | rebuild-designed, package-informed class factors |
| Weapon localizations | 80 | required English and Vietnamese rows |
| Visual bindings | 40 | rebuild-designed contract; assets remain planned |
| Modifiers | 126 | 125 package-confirmed rows plus one rebuild flat-attack affix |
| Affix tiers | 160 | 20 active affixes x 8 item-level tiers (`T1`-`T8`) |
| Active weapon-affix pool rows | 20 | 12 prefixes and 8 suffixes |
| Virtue effects | 5 | package-confirmed |
| Collection sets | 61 | package-confirmed rows, unresolved effect semantics |

## Fail-closed rules

- The ordinary weapon pool contains 12 prefixes and 8 suffixes. Every active
  affix has exactly one tier for each current item-level band; see
  `docs/game-design/weapon-affix-pool-v1.md`.
- Flat attack is a rebuild-designed prefix because the mined `gearProperty`
  table does not provide a flat-damage row. Its tier values are derived from
  12%-20% of the accepted base-power curve.
- Package-backed tier values never exceed their recovered positive range.
- Exclusive groups prevent duplicate race, economy, and need-recovery families
  on one item.
- Archangel (`48`) and Demon Lord (`49`) remain visible special-explicit rows,
  but their acquisition pool is disabled because normal gear rows do not prove
  the Ancient/Primal binding.
- Collection-set `optionType` and `optionValue` are preserved as raw values and
  are not interpreted as active gameplay effects.
- Virtues do not consume prefix or suffix slots.

## Admin projection

The Basic Auth admin exposes paginated, searchable read-only pages for Weapon
Bases, Modifiers, Modifier Tiers, Weapon Pools, Virtue Effects, and Collection
Sets. Full row payloads remain inspectable, including evidence state, source
IDs, weights, duplicate groups, expected visual paths, and unresolved status.
The dedicated Weapon Wiki groups each active modifier with its English and
Vietnamese names, slot, weight, exclusive group, source boundary, and complete
eight-tier level/roll table for faster design review.

## Runtime weapon slice

`evil-hunter-rebuild-v1.weapon-core-v1` is now used by the authoritative
blacksmith craft path for weapon recipes with a resolved package icon binding.
Each requested quantity becomes an individually identified instance. The
server rolls the inclusive `attackDamageMin..attackDamageMax` range, persists
the result through the existing normalized gear-instance authority, and
projects the item into Hunter Inventory after purchase. A compatible weapon
bought for the preselected Hunter is equipped during the same authoritative
purchase settlement. The selected UUID is encoded in the persisted weapon-slot
catalog reference, and the rolled Attack Damage contributes to the Hunter's
projected ATK, normal attack calculation, and skill fallback damage. An
incompatible weapon remains owned in inventory and cannot replace the current
weapon slot; it can be reconsidered later through the same class-validated
`equip_hunter_weapon` command. This first slice still has no random affix
instances.

The display shop projects the next individually rolled weapon instance instead
of presenting only its recipe aggregate. The detail popup shows the authoritative
Attack Damage roll, quality, rating and price, and requires an idle Hunter with
enough gold before enabling Purchase. Settlement still revalidates every value
on the server and transfers the same first displayed instance.
