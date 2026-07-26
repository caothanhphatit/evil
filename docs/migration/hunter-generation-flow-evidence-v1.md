# Hunter generation and arrival evidence v1

## Scope

This pass separates packaged facts from behavior still hidden in the protected
IL2CPP method bodies. It does not define a replacement generator and does not
modify production runtime code.

Primary evidence:

- `reverse-engineering/evidence/hunter-generation-tables-v1.json`
- `reverse-engineering/evidence/hunter-name-pools-v1.json`
- `reverse-engineering/evidence/il2cpp-hunter-generation-v1.json`
- `reverse-engineering/evidence/hunter-modular-asset-audit-v1.md`
- `game-assets/extracted/joined_unity_files/sharedassets1.assets`
- `game-assets/source/unity-assets/bin/Data/Managed/Metadata/global-metadata.dat`
- `reverse-engineering/native-libs/arm64-v8a/libil2cpp.so`

## Exact packaged generation inputs

Four QuickSheet objects were decoded exactly. Every decoder consumes its Unity
object with zero trailing bytes and records raw/decoded SHA-256 hashes.

| Pool/table | Unity path ID | Rows | Result |
| --- | ---: | ---: | --- |
| Male Hunter names (`hunterNameM`) | 12596 | 70 | exact, 14 locales |
| Female Hunter names (`hunterNameW`) | 12597 | 70 | exact, 14 locales |
| Hunter class definitions (`hunter`) | 12599 | 5 | exact |
| Characteristics (`personality`) | 12613 | 33 | exact |

This resolves the original name-pool question for version 1.411: names are not
only hardcoded strings in native code and are not supplied by localization
Addressables. The two complete sex-specific pools are serialized QuickSheet
rows in `sharedassets1.assets`.

The serialized tables also retain their build-time Google Sheet identifier.
The currently public `hunterNameM` and `hunterNameW` sheets match all packaged
`70 x 15` cells, but no Addressables entry, Google Sheets endpoint, or runtime
fetch route was found. The packaged rows remain the versioned authority; a
runtime refresh from Google Sheets or the game server is not evidenced.

The five class-definition rows have indices `0..4`. Each row contains nine job
names and packaged numeric ranges for HP, damage, armor, dodge, and critical,
plus `Second` and `Third` variants, attack speed, revive percentages, and three
percentage fields. The English base job names are:

| Index | Base job | Other names stored in the same row |
| ---: | --- | --- |
| 0 | Berserker | Duelist, Slayer, Warrior, Barbarian, SwordSaint, Destroyer, SwordEmperor, BattleCommander |
| 1 | Paladin | Crusader, Templar, DarkPaladin, Guardian, Inquisitor, Executor, HolyKnight, HighPriest |
| 2 | Ranger | HawkEye, Sniper, Summonarcher, Minstrel, Scout, Arcanearcher, Deadeye, StarShooter |
| 3 | Sorcerer | ArchMage, DarkMage, Ignis, Conjuror, DarkLord, Illusionist, ManaLord, Oppositer |
| 4 | DarkKnight | Doomrider, ShadowLancer, LanceMaster, AbyssDefender, EvilKnight, DragonKnight, Overlord, Deathbringer |

The packaged base ranges are `HP 6000..6200 / damage 80..90 / armor
90..100 / dodge 1..2 / critical 1..2` for rows 0, 1, and 4. Rows 2 and 3 use
`HP 5600..5800 / damage 90..100 / armor 80..90 / dodge 4..5 / critical
4..5`. These are table bounds. The inclusive/exclusive RNG rule and roll
distribution are not yet recovered.

The 33-row `personality` table is the packaged content surface that matches the
player-facing Characteristic concept. It contains an index, localized name,
Korean description template, and `keepValue`. Presence of this table does not
prove that all 33 rows have equal generation weight.

## Recovered runtime boundaries

Protected metadata prevents exact decompilation, but enclosing record shapes
and readable method fragments establish these boundaries:

### HunterManager candidate

Metadata type index 138 is the strongest `HunterManager` candidate. It has five
fields and 74 methods and contains readable fragments for:

- `AddWaitHu...`
- `WaitUnitSav...`
- `FixRan...`
- `GetHunterDef...`
- `UseHunterI...`
- `AddHunte...`
- `WaitHunte...`

The `FixRan...` fragment is consistent with the previously recovered
`FixRandomHunterBodyIndex` identifier. Because the identifier record is
poisoned, this pass does not claim its implementation, range, or timing.

### Initialization and actor materialization

- Metadata type index 1515 is the large `GameManager` candidate and contains an
  `InitHunte...` method fragment.
- Metadata type index 1079 has 138 fields, 148 methods, and a clean `InitHunter`
  method. Its controller-sized record is consistent with a Hunter actor/control
  boundary, but exact class correlation is not yet proven.
- Scene evidence contains `WaitHunterView`; raw metadata contains four
  `EntryHunter` occurrences and four `WaitHunter` occurrences.
- `UserData` (type index 5) has two `EntryHunter...` backing-field fragments and
  matching accessor fragments, showing that entry/wait state crosses into the
  durable user snapshot rather than existing only in a transient popup.

These facts support separate generation, waiting/entry, and actor-init stages.
They do not establish an exact call graph such as `Create -> AddWait -> Entry ->
Init`; that order still requires method-body recovery or a runtime trace.

## Per-instance Hunter snapshot

Metadata type index 1587 is the strong `HunterData` match: 109 fields and 236
methods. Readable backing fields/accessors show that one Hunter instance stores
all of the following categories together:

- progression and identity-like state: job, sub-job, fourth job, money,
  position, area index, grade rank;
- live stats/state: damage, attack, dodge, revive state, hunting timestamps,
  building occupancy, ranking values;
- appearance: costume, costume hat/hide, fairy, weapon costume, wing costume,
  seal costume, hat-hide, riding state, body index;
- inventory/build: gear, item, consumable, skill, `JobTraitDic`;
- Characteristic/personality-like state through a `char...` backing field and a
  `set_char...` accessor.

Readable setters include `set_costum...`, `set_Hat...`, `set_costumeH...`,
`set_CosHat...`, `set_bodyInde...`, and `set_weaponC...`. This is direct
evidence that visual/loadout choices are stored per Hunter instance and must
not be derived from the renameable display name.

Metadata type index 1972 is the strong `HunterLookData` match. Its 11 fields
include clean fragments `acenum`, `acebody`, `acerevive`, `acesubjo...`, and
`acewing`, plus weapon/costume-like fragments. It confirms a compact look
projection exists in addition to the full `HunterData` state, but its exact
serialization consumer remains unresolved.

## What is resolved versus still blocked

### Resolved from packaged data

- Complete original v1.411 male name pool: 70 rows.
- Complete original v1.411 female name pool: 70 rows.
- Five base class-definition rows and their packaged stat bounds.
- All 33 packaged Characteristic/personality definitions.
- Appearance, class/progression, equipment, traits, and mutable display state
  belong to a per-instance Hunter snapshot.
- Waiting/entry and actor initialization are separate runtime surfaces.

### Not yet resolved; do not fabricate

- Sex probability and whether sex is rolled before or after class/grade.
- Name-row weighting, uniqueness checks, and reroll behavior.
- Rarity/grade probability, pity/tutorial overrides, and how grade chooses
  base/Second/Third stat arrays.
- Exact integer/float RNG boundaries for each stat.
- Characteristic weighting, exclusions, duplicate rules, and grade coupling.
- Body-index RNG range. Spine has 120 standard appearance skins plus 40
  `darkload` skins per sex, while portraits use numeric IDs `01..160`. Asset
  order strongly suggests numeric portraits `121..160` correspond to
  `darkload01..40`, but code/runtime evidence has not confirmed that mapping.
- The purpose and correction rule of `FixRandomHunterBodyIndex`.
- Default costume, hat, and weapon selection and their dependency on class,
  sub-job, starting gear, or body/sex.
- Exact arrival call order, waiting capacity, walking path, accept/banish
  conditions, and when the generated snapshot is committed.
- Whether any server response can override local generation inputs.

## Required next evidence pass

The fastest reliable route is a runtime trace on the original build, not a
static guess:

1. Hook or instrument the recovered HunterManager method indices around
   `AddWaitHu...`, `AddHunte...`, `WaitHunte...`, and `FixRan...`.
2. Capture `HunterData` immediately before waiting insertion, immediately before
   entry, and immediately after actor `InitHunter`.
3. Generate enough Hunters to derive observed grade, sex, body,
   Characteristic, and stat distributions; keep tutorial/paid summon routes
   separate.
4. Correlate body/costume/hat/weapon snapshot values to Spine skin composition.
5. Only after this trace, publish a deterministic rebuild generator and seed
   demo accounts through the same command path.

Static native recovery remains possible, but the current metadata v39
protection and unresolved Android relocations prevent trustworthy method-body
decompilation. Any generator implemented before either runtime tracing or that
repair would be a rebuild policy, not a recovered original rule.
