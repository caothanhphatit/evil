# Hunter information data audit v1

## Scope and references

This audit covers the six original screenshots supplied in the Hunter sheet:

1. town Hunter roster;
2. common Hunter header/equipment surface plus the stats tab;
3. skills tab;
4. Secret Points tab;
5. riding-pet tab;
6. carried-materials tab.

It compares the packaged v1.411 data with migrations `0013`-`0015`, the Rust
durable aggregate, PostgreSQL loading/saving, and the world projection. It does
not treat fixture labels or guessed asset-name mappings as original behavior.

New exact evidence from this pass is in
`reverse-engineering/evidence/hunter-info-tables-v1.json`. The extractor
consumes every selected Unity MonoBehaviour exactly and leaves zero trailing
bytes.

## Exact packaged definitions now available

| Surface | QuickSheet object | Rows | Result |
| --- | --- | ---: | --- |
| Basic skills | `skill`, path 12636 | 10 | Exact values and 14 locales |
| Class-change skills | `subJobSkill`, path 12637 | 40 | Exact gates, values and 14 locales |
| Secret Points | `growupProperty`, path 12594 | 15 | Exact labels and per-rank values |
| Experience | `exp`, path 12579 | 100 | Exact six difficulty columns; six dummy strings remain intentionally unresolved |
| Job Traits | `jobTrait`, path 12602 | 69 | Exact job gates, tree positions, costs, levels and values |
| Riding pets | `ridingPet`, path 12627 | 21 | Exact grade/background definitions and titles |
| Riding-pet skills | `ridingPetSkill`, path 12625 | 3 | Exact values and 14 locales |
| Riding-pet traits | `ridingPetTrait`, path 12626 | 6 | Exact levels, values and 14 locales |

Existing exact evidence also supplies:

- 33 Characteristic/personality definitions;
- 5 class rows with HP, damage, armor, dodge, critical and attack-speed data;
- 10 consumables, 671 gear definitions, 369 materials, 61 runes and their recipes;
- 172 material icons currently packaged by the rebuild through the
  `shop_product_<index>` filename convention;
- 320 portraits and the modular Hunter Spine skins.

Definition completeness must not be confused with player-state completeness.
The tables above describe content, but they do not say which rows a particular
Hunter owns or how many points that Hunter allocated.

## Screenshot-by-screenshot coverage

### 1. Town Hunter roster

Confirmed and currently representable:

- stable Hunter ID, mutable display name, class, rarity, level and portrait;
- active/waiting roster state, active capacity eight and FIFO waiting order;
- generic action state and server-owned service gauges.

Missing or incorrectly represented:

- the title prefix is a Characteristic, not a Job Trait. The current
  `hunter_trait_definition` fixture conflates these two separate domains;
- per-instance composed appearance is defined by draft migration `0015` but is
  not loaded, saved or projected by the server;
- roster action labels such as Fun, Heal, Dead and Sell Material are not backed
  by the original state machine yet;
- rename and lock commands are absent even though `display_name` can be stored.

### 2. Common header, equipment and stats

Confirmed and currently representable:

- name, class, rarity, level, XP total, gold;
- current/maximum HP, satiety, mood and stamina;
- ATK and DEF fixture columns;
- complete reusable definitions for all equipment categories.

Confirmed in original per-instance metadata but not represented end to end:

- job/sub-job/fourth-job, grade rank, body index and Characteristic;
- critical, dodge/evasion, attack-speed/rank fields;
- gear, item, consumable and skill collections;
- costume, hat, weapon costume, wing, seal, fairy and riding-pet state.

Missing from the current database/protocol:

- reincarnation count/stars and awakening progress;
- rolled base stat values and class-change stat tier;
- critical rate, attack speed and evasion;
- the original DPS calculation;
- normalized gear instances and the equipped-slot assignment shown around the
  actor;
- gear grade, enhancement and rolled option state;
- lock state and the rename intent;
- visual component projection.

The class table provides generation bounds, not a Hunter's rolled values. It
cannot be used as a fallback for the stats tab.

### 3. Skills tab

Definition data is now complete for the pictured surface: 10 basic skills and
40 class-change skills, with names, descriptions, level caps, cooldowns,
upgrade values and study costs.

The current runtime is not complete:

- migration `0014` seeds six visual-only fixture skills rather than the 50
  original definitions;
- `player_hunter_skill` can store ownership and level, but definitions lack
  basic/sub-job category and class-change gates;
- learned/locked state, study currency and class-change availability are not
  projected;
- packaged skill icons exist, but a row-to-icon binding must be supported by a
  serialized reference or verified UI ordering before publication.

### 4. Secret Points tab

The full 15-row definition table is decoded. It exactly covers the screenshot's
15 cells, including HP, mood, satiety, stamina, ATK, DEF, critical, speed,
evasion and skill-related properties.

The per-Hunter state is absent:

- total Secret Points;
- allocated rank for each of the 15 properties (the screenshot permits up to
  100 per cell);
- spend/refund rules and command idempotency;
- calculated stat contribution.

No rank values should be synthesized from level or rarity.

### 5. Riding-pet tab

Definitions are available for 21 riding-pet rows, 3 pet skills and 6 pet
traits. Metadata also confirms riding state belongs to `HunterData`.

Missing:

- player-owned pet instances;
- Hunter-to-pet mount assignment;
- ranch membership and the Move to Ranch command;
- pet level/grade/trait/skill instance state;
- the exact row-to-actor/icon asset binding.

The empty state shown in the reference can be implemented without fabricating a
pet, but the populated state cannot yet be authoritative.

### 6. Carried-materials tab

All 369 material definitions are available. The rebuild currently exposes 172
matching icon files selected by the `shop_product_<index>` filename convention;
that convention is not a serialized row reference and does not cover the other
197 rows. The original `HunterData` has per-Hunter item/consumable/gear
collections.

The current `BuildingSystemSnapshot.material_stocks` is town/trading stock, not
the selected Hunter's carried inventory. `hunter_materials` is an aggregate
fixture counter. Neither can supply this tab.

Missing:

- normalized `(player, hunter, material, quantity)` ownership;
- sale/reservation state while the Hunter is talking to the Trading Post;
- per-Hunter gear and consumable inventory projections.

## Current schema and BE gaps

There are also two persistence defects that block deploying draft migration
`0015`:

- `save_hunter_roster_in` deletes every `player_hunter` row and reinserts it.
  The database therefore generates a new `hunter_instance_id` at each save,
  so the supposed stable instance UUID is not stable;
- that delete cascades through `player_hunter_visual_component`, while
  `insert_hunter_row` does not reinsert visual components. A successfully
  seeded appearance would disappear on the first authoritative save.

The two portrait corrections in draft `0015` also reference filenames not
present in the current packaged catalog. The existing files are
`hunter_f_111__5928.png` and `hunter_m_117__3163.png`, not the draft suffixes.

Content fixtures also require replacement rather than promotion to confirmed
content: migration `0014` labels H5 as Lancer, whereas the exact packaged base
job at source index 4 is DarkKnight. Its six seeded skills are not the decoded
10 basic plus 40 class-change definitions.

The current model needs separate domains rather than extending the fixture
trait JSON shape:

- `hunter_characteristic_definition` and one Characteristic assignment per
  Hunter;
- full basic/sub-job skill definitions plus learned skill state;
- `hunter_growth_property_definition` and per-Hunter allocated ranks;
- `hunter_job_trait_definition` and per-Hunter tree ranks;
- riding-pet definitions, owned instances and mount assignment;
- player gear instances, equipped slots and rolled options;
- per-Hunter material/item/consumable stacks;
- reincarnation, awakening and rolled combat-stat state;
- normalized visual components loaded and projected in composition order.

`player_hunter_trait` must not continue serving both Characteristic and Job
Trait. The former is one generated personality-like property; the latter is a
69-row class progression tree.

The protocol currently projects only ATK, DEF, four gauges, fixture traits and
fixture skills. It has no detail snapshot for equipment, Secret Points, mounts
or carried inventory. A dedicated Hunter detail projection is preferable to
inflating every town roster card with all nested state.

## Safe implementation boundary

Enough original content exists to implement:

- the complete definitions for Stats labels, Skills, Secret Points and riding
  pets;
- the riding-pet empty state;
- the material catalog and the 172 currently source-bound icon candidates;
- the static/common layout and tab structure.

End-to-end authoritative implementation still requires explicit player-state
schema for every tab. The following must remain unavailable rather than receive
fallback data: DPS, reincarnation/awakening, Secret Point allocation, equipped
gear instances, mounted pet instance, carried material quantities, and
unverified row-to-icon mappings.
