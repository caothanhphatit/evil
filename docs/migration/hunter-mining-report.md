# Hunter Mining Report

## Scope and result

This report inventories the Hunter surface recoverable from the local Evil Hunter Tycoon 1.411 artifacts and compares it with the current rebuild. It covers actor assets, UI/HUD, equipment content, legacy class boundaries, scene objects, protocol bindings, and the Hunter lifecycle through town, field, combat, return, trade, services, and equipment.

Confidence labels used below:

- **Confirmed**: directly present in a serialized object, exported asset, decoded embedded table, generated protocol, or executable rebuild code.
- **Inferred**: a relationship strongly suggested by multiple names or neighboring systems, but without recovered field values, method bodies, or an observed runtime trace.
- **Unresolved**: the artifact needed to reproduce the original behavior has not been decoded or bound.

The decisive result is that the visual Hunter spine, portrait set, a large equipment catalog, and the broad subsystem boundaries are recoverable. The original Hunter runtime state machine, base/growth formulas, content-ID-to-skin binding, field routing, loot ownership, buying behavior, and per-Hunter equipment instances are not yet recovered. The current web/server combat Hunter is therefore a migration fixture and must not be treated as an implementation of the original Hunter loop. The rebuild now has an evidence-neutral roster foundation with eight active town slots, a FIFO waiting queue, banishment/promotion, and normalized persistence; these are infrastructure rules, not a claim that the remaining legacy behavior has been decoded.

## Backend migration increment

The first durable Hunter slice is implemented independently from the unresolved combat and economy behavior:

- Active town capacity is exactly 8; later arrivals enter a durable FIFO waiting queue.
- Banishment removes only an active Hunter and atomically promotes the oldest waiting Hunter.
- Banish commands are idempotent by command UUID and reject reuse for another Hunter.
- Durable schema version 11 safely upgrades legacy over-capacity rosters without dropping Hunters.
- Protocol version 14 projects capacity, active Hunters, waiting Hunters, live world action state, and accepts `banish_hunter`.
- Every active roster Hunter now has a stable `village-hunter-{hunter_id}` live entity. Waiting Hunters do not spawn; banishment removes its actor and FIFO promotion adds the queued actor in the same authoritative snapshot.
- Village presentation uses deterministic non-overlapping active-slot lanes and only the confirmed `hunter_stay`/`hunter_walk` clips. The lanes are rebuild presentation rules, not recovered legacy navigation.
- PostgreSQL migration `0013_normalized_hunter_roster.sql` stores roster state, ordering, vitals, wallet values, and banish command results. Legacy JSONB remains a load fallback and normalizes on the next authoritative save.

This slice deliberately does not invent Hunter jobs, names, stats, equipment, AI, loot ownership, service priorities, or field behavior.

## Evidence baseline

| Evidence | What it proves | Limit |
| --- | --- | --- |
| `game-assets/extracted/exported/metadata/inventory.json` | Unity source file, path ID, type, and object name | Does not reconstruct all serialized references |
| `game-assets/extracted/joined_unity_files/level1` and `reverse-engineering/evidence/level1-scene-hierarchy.json` | Scene object existence, transform/component records where decoded | Repeated prefab/UI names are not semantic bindings by themselves |
| `reverse-engineering/evidence/monoscripts.csv` | 4,435 script records and legacy class names/assemblies | No fields or method bodies |
| `game-assets/source/unity-assets/bin/Data/Managed/Metadata/global-metadata.dat` | IL2CPP metadata source, version 39 | Protected layout is not currently accepted by LibCpp2IL/Mfuscator repair |
| `reverse-engineering/native-libs/arm64-v8a/libil2cpp.so` | AOT gameplay binary | Stripped and cannot currently be paired with repaired metadata for reliable C# recovery |
| `reverse-engineering/evidence/core-economy-tables-v1.json` | Exact decoded equipment/material/rune rows from embedded bytes | Does not contain `AdminHunterData`, skill, job, trait, XP, field, monster, or drop tables |
| `packages/protocol/world-v1.schema.json` and `apps/server/src/simulation/` | Current rebuild authority and bindings | Mostly a deliberate fixture, not evidence of legacy formulas |

## Asset inventory

### Primary runtime actor bundle — confirmed

The three files below are one atomic Spine bundle and are sufficient to render the modular Hunter actor. They must stay linked; the atlas references the texture page by its original name.

| Unity object | Exported path | SHA-256 | Detail |
| --- | --- | --- | --- |
| `sharedassets1.assets:166`, `Texture2D hunter` | `game-assets/extracted/exported/textures/hunter__166.png` | `e4894324b686a69b8f57f304ab6c426e846c0d12733bf3fa1cd91ba79414e432` | 4096 x 1024 RGBA atlas page |
| `sharedassets1.assets:245`, `TextAsset hunter.json` | `game-assets/extracted/exported/text/hunter.json__245.bin` | `ca33c98ded10d455d6e460b5c3a843906976c00e7de4f062778a4130262e3689` | Spine 4.2.43, 28 bones, 56 slots, 1,937 skins, 70 animations |
| `sharedassets1.assets:258`, `TextAsset hunter.atlas` | `game-assets/extracted/exported/text/hunter.atlas__258.bin` | `112b655cfefe6465c2f296f2e9043212a9e91d7f50be9a9327dae377aa57d9d8` | Region/page metadata |

Published rebuild copies already exist at:

- `apps/web/public/content/releases/visible-world-v1/actors/hunter/`
- `apps/web/public/content/releases/slice-001-combat-v1/actors/hunter/`
- `apps/web/public/content/releases/original-flow-v1/textures/hunter__166.png`
- `apps/web/public/content/releases/original-flow-v1/text/hunter.json__245.bin`
- `apps/web/public/content/releases/original-flow-v1/text/hunter.atlas__258.bin`

The source skeleton has no top-level Spine event definitions. Gameplay hit timing, sound, damage, loot, and service events cannot be reconstructed from animation event keys in this bundle.

### Animation catalog — confirmed existence

| State family | Exact clips |
| --- | --- |
| Common idle/move | `hunter_stay`, `hunter_stay_back`, `hunter_walk`, `hunter_walk_back`, `hunter_stay_song`, `hunter_warry` |
| Common damage/death | `hunter_damage`, `hunter_damage_back`, `hunter_dying`, `hunter_die` |
| Mounted common states | `hunter_stay_vehicle`, `hunter_stay_back_vehicle`, `hunter_stay_song_vehicle`, `hunter_walk_vehicle`, `hunter_walk_back_vehicle`, `hunter_damage_vehicle`, `hunter_damage_back_vehicle`, `hunter_dying_vehicle`, `hunter_die_vehicle`, `vehicle_stay`, `vehicle_walk`, `vehicle_walk_back` |
| H1 family | `h1_hit`, `h1_hit_back`, `h1_hit_vehicle`, `h1_hit_back_vehicle`, `h1_a_hit`, `h1_a_hit_back`, `h1_a_hit_vehicle`, `h1_a_hit_back_vehicle`, `h1_hit_whirlwind`, `h1_hit_whirlwind_vehicle` |
| H2 family | `h2_hit`, `h2_hit_back`, `h2_hit_vehicle`, `h2_hit_back_vehicle`, `h2_hit_executor`, `h2_hit_executor_back` |
| H3 family | `h3_hit`, `h3_hit_back`, `h3_hit_vehicle`, `h3_hit_back_vehicle`, `h3_hit_arcane`, `h3_hit_arcane_back`, `h3_hit_arcane_vehicle`, `h3_hit_arcane_back_vehicle` |
| H4 family | `h4_hit`, `h4_hit_back`, `h4_hit_vehicle`, `h4_hit_back_vehicle`, `h4_hit_darkload`, `h4_hit_darkload_back`, `h4_hit_darkload_vehicle`, `h4_hit_darkload_back_vehicle` |
| H5 family | `h5_hit`, `h5_hit_back`, `h5_hit_vehicle`, `h5_hit_back_vehicle`, `h5_a_hit`, `h5_a_hit_back`, `h5_a_hit_vehicle`, `h5_a_hit_back_vehicle`, `h5_hit_roundslash`, `h5_hit_roundslash_back`, `h5_hit_roundslash_vehicle`, `h5_hit_roundslash_back_vehicle`, `h5_hit_shadejavelin`, `h5_hit_shadejavelin_vehicle`, `h5_hit_dragonbreath_vehicle`, `h5_hit_dragonbreath_back_vehicle` |

The clips confirm visual capabilities, facing variants, five broad combat families, advanced variants, and mounts. They do not by themselves identify job names, content IDs, attack cadence, hit frames, skill costs, targeting, or stat formulas.

### Modular skins — confirmed existence, unresolved data binding

The 1,937 skins are not 1,937 complete characters. They are composable visual fragments. Name-family counts from the skeleton are:

| Skin family | Count | Observed examples / meaning |
| --- | ---: | --- |
| `weapon*` | 1,059 | `weapon_h1a_a_01`; weapons and variants by broad H-family |
| `costum*` | 370 | `costum_h1_01`, gendered/advanced variants; source spelling retained |
| `hunter*` | 326 | body/face/hair-style Hunter components |
| `z*` | 64 | vehicle entries such as `z_vehicle_01` through later variants |
| `hat*` | 47 | headwear visual components |
| `All*` | 33 | aggregate composition helpers such as `All_h1`, `All_h2_executor`, `All_h3_mistic`, `All_h4_darkload`, `All_h5_concentrate` |
| `emotion*` | 18 | facial/emotion components |
| `atk*` | 8 | attack-effect composition helpers |
| Empty/reset helpers | 3+ | `hair_empty`, `hair_b_empty`, `hair_b_deco_empty` |

`All_h1` plus `weapon_h1a_a_01` is a confirmed renderable composition and is used by the current fixture. It is not confirmed as the original starter Hunter. No decoded row binds a Hunter job, gear content index, portrait index, gender/body components, costume, hat, weapon skin, or vehicle skin into one original runtime appearance.

### Portrait and roster assets — confirmed

- 160 female miniature portraits: `game-assets/extracted/exported/sprites/hunter_f_01__*.png` through `hunter_f_160__*.png`.
- 160 male miniature portraits: `game-assets/extracted/exported/sprites/hunter_m_01__*.png` through `hunter_m_160__*.png`.
- Typical portrait source dimensions are about 21–23 x 39 pixels, so they require nearest-neighbor/pixel-safe scaling and must not be stretched as large card art.
- Roster/selection decoration includes `hunter_area_bg`, `hunter_check_1`, `hunter_check_2`, `hunter_check_3`, `hunter_shadow`, `assign_hunter_info_box`, `assign_hunter_photo_box`, `assign_hunter_photo_frame`, `hunter_area_bg`, `rp_hunter_box`, `rp_hunter_box_corver`, `rp_hunter_empty`, `apvp_myhunter_frame`, `apvp_otherhunter_frame`, and `fp_hunter_frame`.
- Hunter equipment UI exposes 18 numbered `ic_hunter_gear_0` through `ic_hunter_gear_17` sprites, plus `ic_hunter_gear_search` and `ic_hunter_lock`.

The numbering is confirmed only as an asset family. Slot semantics and whether every icon is active in version 1.411 remain unresolved.

### Character detail, job, trait, equipment, HUD, and revive assets — confirmed existence

Relevant exact families in `game-assets/extracted/exported/sprites/` include:

| Area | Assets recovered |
| --- | --- |
| Character detail | `character_info`, `character_bar_dummy`, `character_graph0`–`character_graph5`, `character_star_off/on`, class boxes/name boxes/lines, class color patterns, toggle background/off |
| Job progression | `hero_job_*`, `backimg_herojob_*`, `herojob_hunter_bg`, `herojob_move_pop_img`, `erjob_hunter_bg`, `erjob_skill_box`, `erjob_skill_frame`, `img_herojob`, `job_move_pop_img` |
| Job traits | 69 `job_trait_*` sprites: four `job_trait_all_*` and H1–H5, S1–S4 branches with numbered nodes |
| Equipment slots | `equip_dummy_0`, `equip_dummy_01`–`equip_dummy_08` and corresponding `_on` states; `equip_bg_9`, `equip_gold_bg_9`, `equip_sel_bg_9`, `equip_sel_ic_9` |
| Gear management | `box_gear_9`, `box_gear_up_01`–`04`, gear growth, lock, preset, refresh, search, dismantle, storage, conversion, succession, limit break and option UI families |
| Basic HP HUD | `hp_bg`, `hp_in`, `hp_flag`, `hp_lv_bg_9`; scene objects named `HpBar` and `StatusGroup` |
| Revival/death | `ic_revive_off`, `ic_revive_on`, `die`, `h1_hero_deatharmor_0`–`3`, adventure grave sprites |
| Speech | `Adventure_Speechballoon_01/02`, `touch_bubble`, and Hunter speech/name data classes |

These assets show the original UI has substantially more Hunter detail than the current rebuild. Their exact panel composition, anchors, visibility rules, text labels, and click flow are unresolved because prefab component references and runtime values have not been bound into a Hunter UI contract.

### Equipment/content assets and data — confirmed

`reverse-engineering/evidence/core-economy-tables-v1.json` exactly decodes these Hunter-facing rows from embedded serialized bytes:

| Table | Rows | Confirmed fields useful to Hunter migration |
| --- | ---: | --- |
| Weapons | 315 | job, unique type, group, item/growth levels, rating values, base/secondary values, modifier arrays, crafting materials by rating, Hunter buy price by rating, localization |
| Armor | 43 | same general gear schema; job `999` appears as a broad-use marker but its semantic label is not decoded |
| Helmets | 107 | same general gear schema |
| Gloves | 43 | same general gear schema |
| Boots | 43 | same general gear schema |
| Rings | 43 | accessory schema |
| Necklaces | 43 | accessory schema |
| Belts | 34 | accessory schema |
| Materials | 369 | material index, price, conversion/composition fields, parent, rating, level, magic marker, localized names |
| Runes | 61 | rune type, job-use value, property index, grade percentages and min/max arrays, localization |
| Rune craft | 10 | decoded rune crafting rows |
| Consumables | 5 | decoded consumable rows |

The current published web release contains 284 source-bound gear icons under `apps/web/public/content/releases/evil-hunter-1.411/gear-icons/`. This is not complete visual coverage for all 671 decoded gear rows, particularly helmets and many weapon indices. Equipment data completeness and equipment icon completeness are separate metrics.

The decoded schema contains enough data to build a typed gear definition catalog and recipes. It does not provide owned gear instance IDs, rolled properties, equipped-slot ownership, durability, Hunter purchase transaction state, or the mapping from a gear definition to a Spine skin.

### Audio/effects — candidate only

The export contains named attack/skill audio and effects such as `spearAttack`, `roundSlash`, `frostwave`, `shadowStrike`, `true_death`, `uniquedrop`, and numerous H-family/advanced-skill sprite/controller pairs. Names establish broad intent only. No recovered event binding maps these files to a specific Hunter job, animation frame, skill row, ordinary attack, or loot event. They must remain disabled or explicitly marked fixture until a serialized reference or runtime capture confirms the mapping.

## Legacy functional surface

### Class inventory — confirmed boundaries

There are 55 `hunter`-named `Assembly-CSharp` MonoScripts. The most important groups are:

| Concern | Confirmed classes |
| --- | --- |
| Core actor/coordination | `HunterCtrl`, `HunterManager`, `HunterPatternCtrl`, `HunterClickCtrl`, `HunterSelectCtrl`, `HunterData`, `hunterData`, `hunter` |
| Content/admin | `AdminHunterData`, `BuildHunterData`, `AdminHunterNameMData`, `AdminHunterNameWData`, `AdminHunterSpeachData` and matching runtime row classes |
| Roster/detail | `HunterInfoList`, `HunterDetailPop`, `HunterDetailGupList`, `HunterThumbUIFormFactor`, `HunterSortPop`, `SortHunterThumbList`, `BatchHunterThumbList`, `HunterSelectPop`, `HunterWaitSelectPop`, `ExHunterList` |
| Growth/jobs/skills | `HunterGrowUpPop`, `HunterGrowUpPropertyPop`, `HunterSkillPop`, `HeroicJobPop`, `JobChangeList`, `AdminJobTraitData`, `jobTrait`, `AdminSubJobSkillData`, `subJobSkill` |
| Gear | `HunterGearDetailPop`, `GearData`, `AdminGearData`, gear slot/growth/option/preset/storage/cube/succession/limit-break classes |
| Death/revival | `HunterRevivePropertyPop`, `ReviveHunterInfoList`, `ReviveHunterInfoRowList`, `ReviveBuildingCtrl`, `AdminReviveBuildingData`, revive property/study classes |
| Mode variants | `AdventureHunterCtrl/List`, `RaidHunterCtrl/Data/ThumbList/WaitHunterList`, `WorldBossHunterCtrl/List`, `PvPHunterCtrl`, `GuildBattleHunterCtrl/List`, `FallenPastureHunterCtrl`, `DamageTestHunterCtrl` |
| Loot/sorting | `HunterRaidDropUtility`, `HunterSortDropUtility`, `HuntingSelectDropUtility`, `DropGearData`, `DropGearTouchCtrl`, `AdminDropUniqueGearData` |

This supports a clean separation between definitions, owned Hunter state, controller/state machine, presentation, mode adapters, equipment instances, and reward/drop systems. It does not reveal original inheritance, fields, method signatures, enums, or transition conditions.

### Scene bindings — confirmed object existence, unresolved route ownership

`level1` contains active objects named `HunterManager`, many `HunterGroup`, `HunterBorder`, `Hunter`, `HpBar`, `StatusGroup`, `MyHunterGroup`, `EnemyHunterGroup`, and world-boss Hunter variants. For example, scene `GameObject pathId 160` is active `HunterManager` with Transform `23334` and MonoBehaviour `103591`.

The large number of repeated names shows that roster, field, PvP, raid, and popup prefabs coexist in the serialized scene. A name match must not be used as proof that an object belongs to the town route. Exact parent-chain, canvas, controller, prefab instance, sorting layer, and route activation bindings still need a dedicated Hunter scene/UI extractor.

## Original Hunter lifecycle model

The following state model is the safest migration target. Solid facts and missing conditions are separated in the table after it.

```text
                 town roaming / waiting
                           |
                  field route selected
                           v
                    travel / acquire target
                           v
              approach <-> attack / skill / damage
                   |               |
                   |          HP reaches zero
                   |               v
                   |          dying -> dead -> revive
                   |
             target defeated -> loot/drop pickup
                           |
              needs / inventory / route condition
                           v
                       return to town
                           |
       +-------------------+--------------------+
       |                   |                    |
 sell requested loot   buy/equip gear      consume services
 at Trading Post       at sale shops       infirmary/inn/food/tavern
       |                   |                    |
       +-------------------+--------------------+
                           |
                    return to field loop
```

| Lifecycle area | Confirmed | Inferred | Unresolved |
| --- | --- | --- | --- |
| Town | Hunter actor bundle; `HunterManager`, `HunterGroup`; click/select/detail classes; service buildings refer to Hunters | Hunters roam/wait and can be selected for detail | Exact spawn, pathfinding, building avoidance, queue choice, idle speech, town scale/sorting, number of initial Hunters |
| Field/farm | Field/mode controllers exist; common walk/attack/damage/death clips; field scene objects/HUD exist | Autonomous target acquisition and repeated farming are central Hunter behavior | Original maps, route selection, navigation graph, target policy, leash/return rules, encounter stats and cadence |
| Combat | `DamageManager/Ctrl/EffectCtrl`; H1–H5 attack families; HP HUD; gear stats tables | Job/weapon family selects an attack/skill presentation | Damage/defense/crit/evasion/speed formulas, attack frame, aggro, AoE, skill AI, status effects, death penalty |
| Loot | Material table; drop/gear runtime classes; touch controller; raid/hunting drop utilities | Hunter owns or carries loot before town settlement | Drop tables/rates, capacity, pickup rules, per-Hunter carried inventory, material seller attribution |
| Return | Mode controllers and town/field concepts exist | Needs, death, inventory, orders, or route completion can cause return | Exact return trigger/priority, travel timing, retained target, entry point, state persistence |
| Sell material | Material prices/ratings; Trading Post request/cancel concept; directional economy names in rebuild content | Hunter sells only requested/eligible loot and receives personal gold | Original transaction algorithm, partial sale, seller wallet, stock capacity, visit animation/queue |
| Buy product/gear | Gear `buyMoneyByRating`; weapon/armor/potion/accessory sale building capabilities; shop classes | Hunter autonomously chooses useful affordable stock | Buyer-selection AI, desired upgrade comparison, personal gold debit, town credit, owned instance creation, old item handling |
| Services | Infirmary/Inn/Restaurant/Tavern product routes; HP/stamina/satiety/mood concepts in rebuild; legacy building/controller names | Hunters visit when a gauge requires restoration and pay personal gold | Original thresholds, priority, walking/queue behavior, service durations/effects/prices, interruptions |
| Equip/growth | Gear tables and management classes; composable weapon/costume skins | Equipped definitions alter derived stats and visual skins | Slot schema, instance rolls, job restrictions, auto-equip, visual skin binding, enhancement/growth formulas |
| Revive | Dying/dead animations, revive UI/classes/building | Death enters a village revival flow rather than a simple client timer | Cost, timer, building capacity, property restoration, death armor/grave use, post-revive needs |

## Current rebuild: implemented versus fixture

### Protocol/data bindings — confirmed current behavior

Protocol schema version 14 exposes these Hunter-relevant browser intents:

- `select_bottom_menu(character)` and `navigate_back`
- `enter_field` and `select_entity`
- `open_hunter_progression(hunter_id)`
- `equip_hunter_item(hunter_id, item_id)`
- `banish_hunter(hunter_id)` with the envelope command UUID as its idempotency key
- `start_building_service(instance_id, hunter_id, product_id)` and legacy alias `start_infirmary_treatment`
- `set_material_request`, `cancel_material_request`, `purchase_shop_item`, and `sell_shop_item`

The authoritative snapshot exposes a Hunter roster projection, product-service candidates/active visits, one fixture combat world, material stocks split into town/hunter quantities, crafted product stock, and a selected world entity.

Durable Hunter roster state currently contains only:

```text
hunter_id, active/waiting position, waiting arrival sequence,
gold, current_hp, max_hp,
stamina(current/max), satiety(current/max), mood(current/max)
```

It also stores resolved-state flags and prior banish command results. It still lacks identity/name/gender/portrait, job/class, level/XP, trait, skills, base and derived stats, equipment slots/instances, carried loot, target/route/state machine, position, speech, revival, and per-mode assignments.

### Explicitly blocked current paths

- `open_hunter_progression` is blocked by `hunter_catalog_binding`, `starter_stats_binding`, and `progression_rules_binding`.
- Field gameplay is blocked by `field_map_exact_binding`, `field_monster_gameplay_binding`, and `combat_rules_binding`.
- Gear sale is blocked by `hunter_roster_binding`, `hunter_wallet_binding`, `hunter_equipment_inventory_binding`, and `shop_visit_binding`.
- Returning-Hunter material settlement fails closed whenever requested Hunter stock needs a seller payment, because stock is pooled and no seller Hunter is attributable.
- `sell_shop_item` has capability routing but no Hunter-side executor.

This fail-closed behavior is correct: completing those transactions without a concrete Hunter wallet and owned item/material instances would create or destroy value.

### Migration fixture only

`apps/server/src/simulation/model.rs` is explicitly named `migration-fixture.slice1-combat-v1`. It has one Hunter, one monster, hard-coded positions/stats, deterministic movement, attack intervals, loot, respawn timers, and a single equipped item bonus. Its states are `Idle`, `Moving`, `Attacking`, `Dead`, and `Reviving`.

The web maps these fixture states to verified visual clips and currently applies `All_h1`, optionally adding `weapon_h1a_a_01`. That validates networking, determinism, Spine rendering, facing, reward idempotency, persistence, and equip projection. None of its HP, attack, speed, range, timers, drop IDs, rewards, or state transitions are legacy claims.

The village world projection is also visual-only roaming (`authority_scope: visual_roaming_only`); it is not the original Hunter town AI.

## End-to-end linkage audit

### Counting rule and result

For this audit, **linked** means a concrete source identity reaches a BE model/handler, a protocol field, and a live FE consumer without a missing hop. It does not mean the rule matches the legacy game. **Partial** means one or more hops exist but use a fixture, hard-coded value, pooled state, static screen, dead code, or an unresolved source mapping. **Missing** means no executable path crosses the stack.

Across the 33 capabilities below:

- **2 linked rebuild pipelines**: Hunter Spine actor loading/rendering and world-entity click/select transport.
- **18 partial pipelines**: they stop at a fixture, static projection, incomplete durable model, or disconnected FE view.
- **13 missing pipelines**.
- **0 legacy-behavior capabilities are fully linked**. The two linked pipelines establish asset/interaction plumbing, not original Hunter rules.
- Only **8 of 70** verified Hunter clips are named in the unused fixture renderer mapping; the live `VisibleWorld` path is currently driven by BE string projections and normally receives only `hunter_walk` for Hunter entities.

### Capability matrix

| # | Capability | Source ID / evidence | BE domain, storage, or handler now | Protocol field/intent | Live FE renderer/binding | Status | Concrete blocker |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 1 | Load/render modular Hunter | `sharedassets1:166/245/258`; Spine 4.2.43 bundle | `WorldEntityDescriptor.asset_bundle_id = hunter`; content manifest | `world.entities[].descriptor` | `visible-world.ts` loads bundle, applies `All_h1`, renders Spine | **Linked** | Composition is fixture; no Hunter-definition binding |
| 2 | Core Hunter prefab/controller | `sharedassets1:1581` `Hunter`; component `12751` -> MonoScript `207 HunterCtrl`; 2,736-byte undecoded payload | No `HunterController` domain; flat roster plus fixture simulation | None specific to controller state | No prefab/component contract; only Spine actor | **Partial** | IL2CPP type tree/field layout absent; payload references not decoded |
| 3 | Hunter manager / town ownership | `level1:160 HunterManager`; component `103591` -> MonoScript `2502 HunterManager`; zero payload bytes | `OriginalFlowSession` projects every active roster Hunter, capped at 8; waiting Hunters remain absent | Whole `world` snapshot | `VisibleWorld` renders one keyed actor per active Hunter | **Partial** | Roster ownership is authoritative, but original manager methods and spawn policy remain in unresolved AOT behavior |
| 4 | Town spawn and roaming | `HunterManager`, `HunterGroup`, `HunterPatternCtrl 4345`; confirmed stay/walk clips | Stable entity ID per Hunter; deterministic non-overlapping active-slot lanes; idle/walking action state | `world.mode`, `visual_tick`, x/y/facing/action_state/animation | Projects and y-sorts all active Hunters | **Partial** | Lane coordinates, path/AI/navigation/collision and obstacle policy are rebuild presentation rules, not recovered legacy behavior |
| 5 | Field visual spawn and roaming | Field `Hunter`, `HpBar`, `StatusGroup`; mode controllers | `world_entities()` hard-codes `field-hunter-01` and `hunter_walk` | Same world projection | Same `VisibleWorld` renderer | **Partial** | `visual_tick` advances only in village; field actor is disconnected from combat simulation |
| 6 | Click/select Hunter | `HunterClickCtrl 606`, `HunterSelectCtrl 1493`, selectable Hunter scene objects | `select_entity()` validates projected selectable ID | `select_entity { entity_id }`, `selected_entity_id` | Full Spine root has `pointertap`; sends ID | **Linked** | Selection has no Hunter detail payload or follow-up action |
| 7 | Character/roster navigation | `HunterInfoList 3509`, `HunterDetailPop 982`, `HunterBorder` scene nodes | Durable active roster (max 8), FIFO waiting queue, idempotent banish and promotion | `select_bottom_menu`, `screen=hunter_roster`, active/waiting projections, `banish_hunter`, `navigate_back` | Shows a static card and back button | **Partial** | FE does not bind the projected roster; no portrait, detail tabs, equipment, stats, or selected Hunter |
| 8 | Hunter identity/name/gender/portrait | `AdminHunterNameM 300`, `AdminHunterNameW 284`; 160 male + 160 female portraits | Only numeric `hunter_id` | Service snapshots expose `hunter_id` only | Displays `Hunter #id`; portrait assets unused | **Missing** | Name tables and portrait/composition indices not decoded/bound |
| 9 | Base job/class | `AdminHunterData 1607`; gear `job=0..4`; Spine H1–H5 families | No job field/catalog | None | Hard-coded `All_h1` | **Missing** | Numeric job -> legacy label -> H-family binding is not serialized/observed |
| 10 | Job trait tree | `AdminJobTraitData 1001`; 69 `job_trait_*` sprites | No trait definitions or owned traits | None | No trait UI | **Missing** | Trait row bytes, node effects, costs, prerequisites and H-family binding absent |
| 11 | Skills/sub-jobs | `HunterSkillPop 2939`, `AdminSubJobSkillData 379`; advanced clip names | No skill definitions, cooldowns, or AI | None | Advanced effects/clips unused | **Missing** | Skill tables/methods/event binding not decoded |
| 12 | Base stats and growth | `AdminHunterData 1607`, `HunterGrowUp*`, likely admin XP/growth classes | Fixture has hard-coded HP/attack; durable roster has HP only | Fixture entity HP; no stat block | Debug HUD shows HP only | **Missing** | Stat field semantics, starter rows, level/XP/growth formulas absent |
| 13 | Derived stats from equipment/trait | Gear tables contain coded values/modifiers; `GearData 4174` | Fixture adds fixed `+4` attack for any equipped item | Only fixture `equipped_item_id` | No stat sheet or derived-stat projection | **Missing** | Property code dictionary and recomputation rules absent |
| 14 | HP/vital state | HP HUD assets; `DamageManager 2270`; `HunterData 711` | Durable roster `current_hp/max_hp`; separate fixture HP | Service roster and fixture entity fields | Service Hunter row plus debug-only combat HUD; source HP art not used | **Partial** | Two unrelated HP stores; field combat does not update durable roster HP |
| 15 | Stamina/satiety/mood | Service building evidence and current product-service contract | `HunterServiceGauge` in durable roster; service handler restores gauges | `product_services[].hunters.current_value/maximum_value` | Hunter service tab renders gauge text/actions | **Partial** | Gauge source fields, original names, decay, thresholds and autonomous visit AI unresolved |
| 16 | Idle visual action | `hunter_stay`, `hunter_stay_back` | Village world emits `action_state=idle` with confirmed `hunter_stay`; fixture has separate `EntityState::Idle` | Live world action state and animation string | `VisibleWorld` plays the projected clip | **Partial** | Town idle presentation is linked, but original idle selection/length/speech behavior remains unresolved |
| 17 | Movement visual action | `hunter_walk`, `hunter_walk_back` | Village world emits deterministic authoritative positions and `action_state=walking` with confirmed `hunter_walk`; field fixture remains separate | Live world x/y/facing/action_state/animation | `VisibleWorld` moves all active roster actors | **Partial** | Village lanes are not legacy navigation; field combat still uses a separate projection |
| 18 | Basic attack action | `h1_hit`, `h1_hit_back`; visual duration 0.3333 s | Fixture `Attacking` with hard-coded 5-tick interval | Fixture state/events | Attack mapping exists in unused `world.ts`; live world never consumes it | **Partial** | No original job/cadence/hit frame; live renderer not wired to fixture state |
| 19 | Advanced attack/skill actions | H1 whirlwind, H2 executor, H3 arcane, H4 darkload, H5 roundslash/shadejavelin/dragonbreath | No states/commands/skill executor | None | Assets available but unreferenced | **Missing** | Skill definition -> AI -> authoritative event -> animation binding absent |
| 20 | Damage reaction | `hunter_damage*`, duration 0.6667 s | Fixture emits `Damage` but state remains attacking/dead rather than damage-react state | `combat.events.damage` | Debug text updates HP; no live damage animation | **Missing** | Protocol/entity state has no transient reaction/action sequence |
| 21 | Dying/death/revival | `hunter_dying` 0.1667 s, `hunter_die` 1.0 s; revive classes/building | Fixture states Dead/Reviving and 30-tick timer; durable roster has no death state | Fixture state/events | Debug HUD; dead/revive clip mapping only in unused renderer | **Partial** | Legacy revive cost/building/queue and live visual projection absent |
| 22 | Target acquisition and combat loop | `HunterPatternCtrl`, `Damage*`, field/mode controllers | Deterministic one-Hunter/one-monster fixture | Fixture world snapshot/events | Debug HUD only; visible actors follow separate roaming path | **Partial** | Fixture target/stats/range/cadence are not legacy and not connected to visible world |
| 23 | Loot creation/pickup | `HuntingSelectDropUtility 4176`, `DropGearData 2632`, `DropGearTouchCtrl 2671` | Fixture creates item `2001`, moves Hunter to it, records reward ledger | `ground_drops`, inventory, drop events | Debug HUD; unused arena renderer can draw drops | **Partial** | Original drop table/item binding and live field rendering absent |
| 24 | Return from field | Mode controllers and village/field routes | `navigate_back()` changes screen and calls settlement | `navigate_back`, screen transition | Back button returns to town | **Partial** | No autonomous return condition, travel, spawn point, per-Hunter assignment, or carried state |
| 25 | Per-Hunter carried materials | Material table and Hunter/drop utilities | `hunter_quantity` is pooled by material in town state | `material_stocks[].hunter_quantity` | Trading Post shows pooled number | **Missing** | No `hunter_id`, trip, source encounter, capacity, or provenance |
| 26 | Hunter sells requested loot | Material prices/ratings; Trading Post request/cancel evidence | `settle_returning_hunters()` deliberately refuses non-empty settlement | Request intents exist; no seller/settlement command/event | Request UI exists | **Missing** | Seller wallet attribution and atomic town-stock transfer absent |
| 27 | Hunter consumes building services | Infirmary/Inn/Restaurant/Tavern route evidence | Authoritative slot, stock, Hunter gold debit, timer and gauge restore handler | `start_building_service` plus product-service snapshots | Production/Hunters tabs can start service | **Partial** | Roster defaults unresolved; original autonomous behavior/formulas/animation/path absent |
| 28 | Gear definition/catalog | Exact 671 decoded gear rows; 284 published icons | Building content/recipes model gear products, not Hunter inventory | Shop recipe snapshots | Craft/shop panels list gear | **Partial** | Catalog is not connected to job definitions, owned instances or Hunter detail |
| 29 | Hunter buys gear/product | `GearBuyList 3874`, shop classes, `buyMoneyByRating` | `purchase_shop_item()` validates stock then returns `binding_blocked` | Intent lacks `hunter_id`; snapshot lacks buyer inventory | Shop control cannot complete purchase | **Missing** | Explicit buyer, wallet debit, town credit and owned item creation absent |
| 30 | Owned equipment instances/slots | `GearData`, 18 `ic_hunter_gear_*`, 9 dummy slot states | Fixture has one global `equipped_item_id`; no gear instances/slots | `equip_hunter_item(hunter_id,item_id)` uses numeric fixture item | No live roster equipment panel | **Missing** | Definition vs instance, ownership, slot, roll, job restriction and concurrency absent |
| 31 | Equip changes visual appearance | `weapon_h1a_a_01` confirmed renderable | Fixture equip checks inventory and stores one ID | Fixture `equipped_item_id`; equip intent | Unused `world.ts` composes weapon skin; live `VisibleWorld` always uses `All_h1` | **Partial** | Live renderer ignores equipped item and no gear -> Spine skin map exists |
| 32 | Gear growth/upgrade/options | Decoded growth/modifier fields; many gear management classes/assets | No Hunter gear growth domain | None | No Hunter equipment management UI | **Missing** | Property code meanings, costs, rolls, caps and transactional owned instances absent |
| 33 | Hunter durability/reconnect/multi-Hunter concurrency | `SaveData`, local/cloud classes; multiple Hunter mode lists | Normalized roster tables persist active/waiting order, gauges, wallets and banish idempotency; fixture state JSONB persists one combatant; revision/fence guards player aggregate | Snapshot/resync plus active/waiting/capacity projection | Restores screens/services/fixture; no per-Hunter world actor restoration | **Partial** | No per-Hunter revisions/assignments, checkpointed AI, gear/loot ownership, or economy ledgers |

### Important runtime disconnects found

1. `apps/web/src/game/world.ts` contains the only state-to-clip mapping for idle/move/attack/dead/revive and the only fixture weapon-skin composition, but it has no live import/constructor call. Its tests validate dead code, not the production renderer.
2. The production `VisibleWorld` consumes `OriginalFlowSnapshot.world`, while combat behavior lives in `migration_fixture_combat.world`. These are separate projections with separate positions/states.
3. Field simulation ticks update `combat_snapshot`; field visible Hunter/monster entities are generated from `visual_tick`, which advances only in village. Therefore production field actors do not visually execute the fixture fight.
4. The roster screen does not consume `hunter_roster` entries; it is static explanatory markup.
5. The roster subset is normalized in PostgreSQL, but combat fixture state and richer Hunter ownership are not. There is still no gear/loot ownership model, assignment checkpoint, or Hunter economy ledger.

## Additional mapping mined from local artifacts

### Exact prefab-to-script bindings

- `sharedassets1.assets:1581 GameObject Hunter` has Transform `5138`, `CircleCollider2D 6001`, `Rigidbody2D 6000`, and MonoBehaviour component `12751` whose header points to `globalgamemanagers.assets:207 HunterCtrl`.
- `HunterCtrl` component `12751` is 2,768 bytes, including a 32-byte MonoBehaviour header and 2,736 bytes of opaque serialized payload. This is a promising future reference-mapping source, but without its IL2CPP field/type layout, scanning integer pairs would create false PPtr bindings.
- `level1:160 GameObject HunterManager` has Transform `23334` and MonoBehaviour `103591`, whose header points to `globalgamemanagers.assets:2502 HunterManager`. Its serialized payload ends at the 32-byte header, so its behavior/configuration must come from methods, static data, spawned objects, or runtime state rather than scene fields.

### Five-way job-family structure

The strongest new class mapping is structural, not yet a confirmed semantic ID mapping:

- Weapons contain exactly five numeric jobs `0..4`, each with exactly 63 rows.
- Each numeric job has the same gear group distribution: `1, 3, 3, 20, 15, 18, 3` rows across groups `0..6`.
- Runes contain seven job-specific rows for each job `0..4`, plus 26 rows marked `999`.
- Hunter animations, aggregate skins, costumes, weapons, and trait sprites independently form five H-families: H1 through H5.
- Trait art has exact topology: 4 universal nodes plus 13 nodes for each H-family (`S1..S3` have 3 nodes each; `S4` has 4), totaling `4 + 5*13 = 69`.

This makes ordinal `job 0..4 <-> H1..H5` a **strong candidate mapping**, but not a confirmed one. No serialized pointer, decoded Hunter/job table, or runtime trace joins either side. It must remain outside authoritative content until such a join is recovered.

Advanced family names such as `executor`, `templer`, `mistic`, `darkload`, `duallist`, `concentrate`, `whirlwind`, `arcane`, `roundslash`, `shadejavelin`, and `dragonbreath` are confirmed source spellings. They are visual labels only; they do not establish localized class names, unlock requirements, trait IDs, or skill semantics.

### Action timing available from Spine

The Spine timelines provide visual durations, not authoritative action timing:

- Ordinary H1–H5 attack variants: 0.3333 s.
- Whirlwind, roundslash, and shade-javelin visuals: 0.5 s.
- Mounted dragon-breath visual: 2.0 s.
- Damage: 0.6667 s; dying: 0.1667 s; die: 1.0 s.
- Idle: 1.0 s; walk: 1.3333 s.

There are no top-level Spine events. Attachment changes at timeline offsets can reproduce visuals, but cannot safely define hit frames, damage cadence, cooldowns, or sound triggers.

### What IL2CPP/metadata still does not yield

- MonoScript records recover class names and exact prefab/component script IDs.
- The protected metadata layout still prevents reliable fields, methods, enums, and method-body recovery.
- Plain `strings` over the stripped `libil2cpp.so` does not expose useful first-party Hunter identifiers; visible field-related strings are generic IL2CPP runtime exports.
- Scene extraction supplies headers and selected built-in component values, but custom Hunter payloads lack matching external type trees.
- Consequently, no new original formula, state transition, job label, trait effect, or content-to-skin ID can be promoted to confirmed from the current binary set.

## Required Hunter schema and API additions

### Content schemas

```text
HunterDefinition
  id, source_row_id, base_job_id, growth_curve_id, default_visual_id,
  base_stat_profile_id, source_confidence, release_id

HunterJobDefinition
  id, source_numeric_id, localized_name, h_family_binding?,
  allowed_weapon_groups, trait_tree_id, skill_tree_id, evidence

HunterTraitDefinition / HunterSkillDefinition
  id, job_id, branch, tier, prerequisites, costs,
  effect_operations[], presentation_binding, evidence

HunterVisualDefinition
  id, portrait_asset_id, base_skin_names[], job_skin_names[],
  equipment_skin_rules[], vehicle_skin_names[], evidence
```

The `h_family_binding` and every presentation binding must be nullable until confirmed. A candidate ordinal must not be encoded as a required foreign key.

### Durable ownership/storage schemas

Add normalized PostgreSQL tables rather than extending one JSON document indefinitely:

```text
player_hunter
  hunter_id UUID PK, player_token, definition_id, name, gender_code,
  portrait_id, visual_id, level, xp, gold, lifecycle_state,
  assignment_id, revision, created_at, updated_at

hunter_vital
  hunter_id PK/FK, hp, max_hp, stamina, max_stamina,
  satiety, max_satiety, mood, max_mood, updated_at

player_gear_instance
  gear_instance_id UUID PK, player_token, definition_id, rating,
  quality, growth_level, rolled_properties JSONB, locked, revision

hunter_equipment
  hunter_id, slot_code, gear_instance_id UNIQUE, revision,
  PRIMARY KEY (hunter_id, slot_code)

hunter_trait / hunter_skill
  hunter_id, definition_id, level, state, PRIMARY KEY (...)

hunter_assignment
  assignment_id UUID PK, hunter_id, mode, route_id, state,
  started_at, checkpoint JSONB, revision

hunter_carried_item
  hunter_id, trip_id, item_id, quantity, provenance_id,
  PRIMARY KEY (hunter_id, trip_id, item_id, provenance_id)

hunter_economy_ledger
  operation_id UUID, player_token, hunter_id, operation_type,
  town_gold_delta, hunter_gold_delta, item_id, quantity,
  gear_instance_id, source_command_id, created_at
```

Equip, purchase, service, return-sale, loot grant, and revival must update ownership plus ledger rows in one transaction with player lease fencing and idempotency keys.

### Protocol additions

Replace overloaded fixture fields with versioned Hunter projections:

```text
HunterSnapshot
  hunter_id, revision, definition_id, name, portrait_asset_id,
  job_id, level, xp, wallet_gold, lifecycle_state, assignment,
  vitals, base_stats, derived_stats, traits, skills,
  equipment[], carried_items[], visual_composition

HunterWorldProjection
  hunter_id, action_sequence, action_state, action_started_at,
  x, y, facing, target_id, animation_binding, hp/status projection
```

Required intents/events:

- `open_hunter_detail { hunter_id }` returning the selected Hunter projection, not only changing screen.
- `assign_hunter { hunter_id, route_id, expected_revision }` and `request_hunter_return`.
- `equip_hunter_gear { hunter_id, slot_code, gear_instance_id, expected_revision }`.
- `purchase_for_hunter { hunter_id, shop_instance_id, product_stock_id, expected_hunter_revision }`.
- Existing `start_building_service` should add the expected Hunter revision and return the updated Hunter plus service assignment.
- Server events/projections for `hunter_action_changed`, `hunter_vitals_changed`, `hunter_loot_changed`, `hunter_equipment_changed`, and economy settlement IDs. These may be folded into revisioned world snapshots, but the fields must remain explicit and authoritative.

The client should send assignment/equip/purchase/service intent only. Target choice, movement, damage, drop quantities, sale totals, stat derivation, and service completion remain server-owned.

### FE binding additions

- Replace the static roster card with a keyed Hunter list using `portrait_asset_id` and `hunter_id`.
- Introduce one `HunterVisualResolver` that composes source skins from `visual_composition`; remove hard-coded `All_h1` from renderers.
- Make the live `VisibleWorld` consume `HunterWorldProjection` from the same simulation that owns combat, services, and assignments.
- Represent action state as a sequence/timestamp so non-looping attack, damage, dying, death, service, and equip transitions are not lost between snapshots.
- Render source HP/status art through a reusable Hunter HUD component; keep debug fixture HUD separate and disabled in compatibility captures.
- Bind detail tabs to stats, equipment instances, traits, skills, carried loot, wallet, and assignment data; never infer values from sprite names.

## Required clean domain model

Do not expand the current flat `DurableHunterState` until every concern is separated. The target model should use stable definitions and owned instances:

| Aggregate/table | Required responsibility |
| --- | --- |
| `hunter_definitions` | Evidence-tagged job/body/growth archetype definitions; never player ownership |
| `player_hunters` | Stable Hunter instance, player, name/gender/portrait/composition IDs, level/XP, wallet, lifecycle state, current route |
| `hunter_vitals` | HP and service gauges with authoritative timestamps/revisions |
| `hunter_base_stats` / derived projection | Base/growth inputs separate from recomputed gear/trait/skill modifiers |
| `gear_definitions` | Existing decoded gear rows and localization |
| `player_gear_instances` | Owner Hunter/player, definition, rating/quality, rolled options, growth, lock, durability/state |
| `hunter_equipment_slots` | Hunter + slot -> gear instance, with uniqueness and job validation |
| `hunter_skills` / `hunter_traits` | Learned/unlocked instance state referencing evidence-tagged definitions |
| `hunter_assignments` | Field/raid/PvP/service/building assignment, start/revision/status |
| `hunter_carried_loot` | Per-Hunter material/item quantities and provenance before settlement |
| `hunter_targets` | Ephemeral target/route/position/cooldowns; hot state may live in memory/Redis but checkpoints remain server authoritative |
| `hunter_economy_ledger` | Material sales, shop purchases, services, rewards, and equip transfers with idempotency keys |
| `hunter_visual_compositions` | Data binding from Hunter/job/gear/costume/vehicle definitions to Spine skin names |

PostgreSQL should retain durable ownership and ledger state. Active movement/target/cooldown simulation can be held in the authoritative process and checkpointed in batches; Redis may coordinate leases or hot recovery but must not become the only copy of player ownership/economy.

## Migration sequence

1. **Publish Hunter evidence artifacts.** Build a deterministic inventory of the 320 portraits, Spine skin/animation names, Hunter UI sprite families, scene nodes, and 55 Hunter class records with source path IDs and hashes.
2. **Decode Hunter definition tables.** Extend the serialized-table extractor to `AdminHunterData`, names, speech, job traits, sub-job skills, XP/growth, revive, field/monster/drop, and shop-selection data. Preserve dummy/unknown fields without guessed labels.
3. **Recover prefab/controller bindings.** Traverse `level1` parent chains and MonoBehaviour references for the roster/detail popup, town Hunter prefab, field Hunter prefab, HP/status HUD, gear panel, and service queues. Produce route-specific contracts instead of global name matching.
4. **Bind visual composition.** Obtain authorized runtime traces for at least one Hunter per base job showing portrait, body/face/hair, costume, weapon, front/back movement, attack, damage, death, return, service, and equip change. Require a serialized reference or two independent observations before publishing a mapping.
5. **Extend normalized Hunter persistence.** The roster, wallets, vitals, waiting order, and banish idempotency are now normalized. Add identity definitions, carried loot, owned gear instances, equipment slots, assignments, and ledger transactions without inventing missing starter data.
6. **Implement town state machine first.** Spawn/wait/roam/select, depth ordering, building avoidance, return arrival, service eligibility/queue, Trading Post seller attribution, shop buyer attribution, and visual equipment projection.
7. **Implement field state machine behind evidence flags.** Route/travel, target acquisition, approach, attack/skill, damage/death, loot pickup, return conditions, and checkpoint/reconnect. Keep fixture formulas isolated until decoded values replace them.
8. **Complete economy atomically.** Hunter material sale debits town gold and credits that Hunter while moving carried loot to town stock; shop purchase debits the same Hunter, credits town, decrements stock, creates an owned instance, and equips/transfers it transactionally.
9. **Add progression and revival.** Only after Hunter/job/trait/skill/growth/revive tables and conditions are decoded. Do not derive formulas from UI labels or animation names.
10. **Validate compatibility.** Golden server traces, reconnect/checkpoint tests, idempotent economy tests, multi-Hunter concurrency, visual captures for every state/facing/job family, and explicit verified/inferred/fixture coverage metrics.

## Immediate migration backlog

| Priority | Deliverable | Exit criterion |
| ---: | --- | --- |
| P0 | Hunter evidence generator/manifest | Exact objects/files/hashes are reproducible and reviewed |
| P0 | `AdminHunterData` + name/job/trait/skill table extraction | Row counts and byte-exact decoder validation pass |
| P0 | Complete Hunter ownership schema | Roster/wallet/vitals are durable; carried loot, gear instances and assignments still need normalized storage |
| P0 | Trading Post seller settlement | No pooled anonymous Hunter stock; double settlement is impossible |
| P0 | Shop buyer/equipment transaction | Buyer is explicit; gold, stock, owned item and equip change commit atomically |
| P1 | Town Hunter prefab/UI contract | Roster, detail, HP/status, click targets and depth/scale bind to exact assets |
| P1 | Town behavior executor | Multiple Hunters roam, queue, service, buy/sell and reconnect deterministically |
| P1 | Gear-to-Spine composition | Definition/instance changes produce verified skin composition |
| P1 | Field/monster/drop table extraction | Map, target, combat and loot rules no longer use fixture content |
| P2 | Growth/job/skill/revive systems | Evidence-backed progression and death/revival loop pass golden traces |
| P2 | Mode adapters | Adventure/raid/PvP/world-boss controllers reuse the core Hunter aggregate without duplicating ownership |

## Unresolved evidence checklist

- Original starter Hunter count, stats, names, jobs, portraits, skins, traits, skills, wallets, and equipment.
- Meaning of H1–H5 and advanced suffixes as data IDs/job labels.
- Full equipment slot semantics and gear-definition-to-Spine-region mapping.
- Original Hunter stat and progression formulas, XP curves, AI priorities, combat cadence, status effects, and skill triggers.
- Exact town pathfinding/navigation, collision, depth anchors, spawn/return points, and building queues.
- Field map routing, monster bindings, drop tables/rates, carried capacity, return rules, and death behavior.
- Hunter material seller attribution and personal-wallet settlement.
- Hunter product/gear purchasing AI, comparison rules, stock reservations, and old-equipment disposition.
- Service need thresholds, selection priority, duration/effect values, and original gauge names/types.
- Revival cost/timer/building capacity and post-revival state.
- Audio/effect event bindings.
- Complete original API/save schema for Hunter cloud synchronization and mode-specific assignments.

Until these items are resolved, UI can use recovered visual components and the server can use clearly labelled fixtures, but compatibility reporting must not count inferred Hunter behavior as migrated.
