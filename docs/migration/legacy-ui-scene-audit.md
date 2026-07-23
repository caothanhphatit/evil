# Legacy UI and Scene Audit (Phase A/B)

**Scope.** This document is an evidence-only audit of the original Evil Hunter Tycoon 1.411 presentation surface. It is a migration input, not a claim that the current web `Training Ground` screen matches the original. No runtime code was changed for this audit.

## Evidence and confidence

| Evidence source | What it can prove | Limits | Confidence |
| --- | --- | --- | --- |
| `/Users/trana/Downloads/Evil_Hunter_Tycoon_1.411_assets/exported/metadata/inventory.json` | Unity object names, object types, source asset file, `path_id` | Does not preserve full hierarchy, transforms, serialized references, or runtime visibility | High for existence/identity; low for placement |
| `/Users/trana/Downloads/Evil_Hunter_Tycoon_1.411_assets/exported/` | Exported PNG/audio/text and their stable source-derived names | Sprite names do not prove which screen uses a sprite | High for asset existence |
| `reverse-engineering/evidence/level1-scene-hierarchy.json` | UnityPy extraction of 23,286 GameObjects, 23,286 transforms/RectTransforms, parent topology, active flags, layers, component identities, transform geometry, and 16 Canvas records | Does not decode Image-to-Sprite references, text/localization values, layout component settings, click/event bindings, or runtime-mutated state | High for static hierarchy and serialized transform state |
| `/Users/trana/Downloads/Evil_Hunter_Tycoon_1.411_assets/joined_unity_files/level1` | Immutable source scene consumed by `tools/extract-level1-scene.py` | Still required when extending extraction to component payloads | High |
| `reverse-engineering/evidence/assembly_csharp_classes.txt` | First-party class/type names and UI/domain surface | IL2CPP metadata is incomplete/protected; no method bodies or field values | High for feature-name inventory; medium for flow interpretation |
| `game-assets/asset-index.json` | 9,359 exported files with byte size and SHA-256 | No screen binding | High for asset provenance |

## Confirmed scene composition signals

The recovered `level1` scene graph contains 23,286 GameObjects with an equal number of decoded `Transform`/`RectTransform` records. This is direct hierarchy evidence, not a name-only inference. The following names are incompatible with the existing mock training screen:

- **World/village shell:** `WorldCanvas`, `MainCanvas`, `TopView`, `BottomView`, `Background`, `MapManager`, `GameManager`, `BuildGroup`, `BuildButton`, `CharacterButton`, `ArchiveButton`, `StoreButton`, `RaidButton`, `Post`, `Guild`, `Achieve`, `Cash`, `Money`, `Elemental`.
- **Hunter/roster presentation:** `HunterGroup`, `HunterBorder`, `HunterManager`, `CharacterView`, `CharacterContent`, `HunterCreateTimer`, `HunterBorder`, `MyHunterBtnBorder`, `HunterGroup`, `HunterInfoList`, `ReviveHunterInfoList`, `AdventureHunterList`, `WorldBossHunterList`, `GuildBattleHunterList`.
- **Combat/field presentation:** `EnemyGroup`, `EvilGroup`, `EnemyDamageBorder`, `MyDamageBorder`, `HpBar`, `StatusGroup`, `ElementalGroup`, `Timer`, `NextStage`, `RaidStatusGroup`, `RaidDamageGroup`, `UIAdventureDamageCanvas`, `UIRaidCanvas`, `PvPView`, `PvPAction`, `WorldCanvas`.
- **Overlays and navigation:** `UIPopupCanvas`, `FocusCanvas`, `Focus Camera`, `Main Camera`, `SettingTileGroup`, `StorageView`, `ChattingView`, `BuildSelect`, `BuildInfoList`, `Building`, `TreasureBox`, `Revive`, `WaitUpButton`, `StartButton`, `ConfirmButton`, `CancleButton`.
- **Results/state feedback:** `Win`, `Lose`, `LogText`, `RewardTitle`, `TreasureBox`, `Tip`, `Overlay`, `Revive`, `LastStage`, `ForceLimitedView1`, `RaidPowerCheck`, `FallenPastureBorder`.

The 16 decoded Canvas records are:

| Canvas GameObject | GameObject ID | Render mode | Sorting order | Serialized enabled |
| --- | ---: | ---: | ---: | --- |
| `BuildSelect` | 35 | 2 | 0 | true |
| `UICanvas` | 83 | 1 | 0 | true |
| `MainCanvas` | 119 | 1 | 0 | true |
| `UIAdventureCanvas` | 121 | 1 | 10 | true |
| `UIRaidCanvas` | 139 | 1 | -1 | true |
| `UIAdventureDamageCanvas` | 147 | 1 | -1 | true |
| `FocusCanvas` | 194 | 1 | 0 | true |
| `WorldCanvas` | 212 | 2 | 0 | true |
| `UIPopupCanvas` | 224 | 1 | 0 | true |
| `UIAdventureHunterCanvas` | 228 | 1 | 0 | true |
| `UIPvPCanvas` | 229 | 1 | -1 | true |
| `UIAdventureMapCanvas` | 303 | 1 | -1 | true |
| `Background` | 355 | 2 | 5 | true |
| `Background` | 350 | 2 | 5 | true |
| `CaptureUICanvas` | 17671 | 1 | 0 | true |
| `UIWorldBossCanvas` | 22039 | 1 | -2 | true |

This confirms a village/main UI shell, a world-space layer, and separate Adventure, Raid, PvP, World Boss, popup, focus, and capture surfaces. Sorting orders are now known for the Canvas components. They do not by themselves establish the final draw order of every child renderer or the runtime visibility sequence.

### Recovered top and bottom navigation topology

`TopView` (GameObject 30) is active, is a `RectTransform`, has 19 direct children, and is parented to active `TopUIBorder` (GameObject 10) under `UICanvas`:

| Direct child | ID | Active | Confirmed immediate contents |
| --- | ---: | --- | --- |
| `Guild` | 20 | true | none |
| `Achieve` | 21 | true | inactive `New` badge |
| `Cash` | 24 | true | `Icon`; `Background > Value > Button` |
| `Money` | 26 | true | `Icon`; `Background > Value > Button` |
| `Elemental` | 27 | true | `Icon`; `Background > Value` |
| `Post` | 29 | true | inactive `New` badge |
| `StageButton` | 341 | true | `Text` |
| `Human` | 349 | true | `Icon`; `Value` |
| `StageButton` | 1751 | true | `Text` |
| `Border` | 1974 | true | `Grid` containing serialized inactive buff/event/premium/status indicators |
| `Setting` | 2335 | true | none |
| `StatusBar` | 2345 | true | `Line` |
| `BoostBack` | 9716 | true | none; serialized local scale is approximately 1.9 x 1.9 |
| `BoostText` | 10675 | true | none |
| `Book` | 11479 | true | `Background > Icon`, `Value`, and `Token > Background > Value/Icon` |
| `ChaosBoostText ` | 12759 | true | none |
| `AbyssBoostText ` | 15198 | true | none |
| `SuperBoostText` | 19383 | true | none |
| `ItemCollection` | 20562 | true | inactive `New` badge |

The `Border > Grid` branch contains 61 serialized inactive indicators, including building buffs, pet buffs, event modifiers, premium/newbie states, guild buffs, inventory/hunter/battle presets, dungeon/colosseum/rift states, and chief blessings. Their inactive flags describe the saved scene defaults, not the conditions that enable them during play.

`BottomView` (GameObject 221) is active, is a `RectTransform`, and is parented to active `BottomUIBorder` (GameObject 278) under `UICanvas`. Its five direct navigation children are exact:

| Direct child | ID | Active | Confirmed immediate contents |
| --- | ---: | --- | --- |
| `CharacterButton` | 215 | true | `Text`, `Image` |
| `BuildButton` | 216 | true | `Text`, `Image`, inactive `Touch > boxImg` |
| `ArchiveButton` | 217 | true | `Text`, `Image`, inactive `DropEffect`, inactive `Touch > boxImg` |
| `StoreButton` | 219 | true | `Text`, `Image`, inactive `Touch > boxImg` |
| `RaidButton` | 223 | true | `Text`, `Image`, inactive `Touch > boxImg` |

The scene graph therefore proves the primary bottom menu membership. It does not yet prove localized button labels, assigned sprite names, sibling order within Unity's serialized transform array, or the final pixel positions produced by layout components and canvas scaling.

### Remaining layout and binding limits

The UnityPy extraction now includes parent IDs, active flags, layer, component IDs/types, local position/scale, and, for `RectTransform`, anchors, anchored position, size delta, and pivot. The following must still be extracted before a pixel-fidelity implementation can use the scene as a complete specification:

- `Image`/`RawImage` sprite and texture PPtrs, material, color, fill mode, and preserve-aspect settings;
- `Text`/TextMeshPro content, font, alignment, size, outline, and localization-key binding;
- Canvas Scaler reference resolution and scaling mode;
- Horizontal/vertical/grid layout, Content Size Fitter, aspect-ratio, and safe-area behavior;
- Unity sibling index, renderer sorting layer/order below each Canvas, masks, and clipping relationships;
- Button target graphics, transition states, `UnityEvent` callbacks, scroll bindings, and navigation rules;
- Animator/controller bindings and runtime code that changes active state, labels, counters, badges, or hierarchy;
- reference screenshots/video needed to validate responsive behavior and runtime composition.

## UI surface inventory by player flow

The names below are grouped from first-party classes and direct asset names. A class name proves the feature exists; it does not prove its exact entry point or button order.

### 1. Boot, title, loading, account

**Observed classes:** `IntroManager`, `LoadingPop`, `LoginManager`, `LoginResult`, `IntroNotiPop`, `SpinSplashCtrl`, `LocalizeManager`, `SoundManager`, `SettingPop`.

**Direct assets:**

- `game-assets/extracted/exported/sprites/intro_bg_new__1695.png`
- `game-assets/extracted/exported/sprites/intro_img_glo_new__2141.png`
- `game-assets/extracted/exported/sprites/intro_img_glo_new_2__1993.png`
- `game-assets/extracted/exported/sprites/intro_glo_touchtostart__7172.png`
- `game-assets/extracted/exported/sprites/intro_glo_t_eng__4557.png`
- `game-assets/extracted/exported/sprites/intro_glo_t_jpn__5534.png`
- `game-assets/extracted/exported/sprites/intro_glo_t_ch_b__7652.png`
- `game-assets/extracted/exported/sprites/intro_glo_t_ch_g__1844.png`
- `game-assets/extracted/exported/sprites/cloud_loading_btn__4266.png`

**Migration implication:** the web boot flow must begin with title/loading/localization state, not directly with the combat fixture.

### 2. Main village / idle management

**Observed classes:** `BuildManager`, `BuildCtrl`, `BuildingData`, `BuildingPop`, `BuildList`, `BuildSelectCtrl`, `BuildInfoList`, `BuildSkinChangePop`, `BuildingReviveCheckPop`, `HunterManager`, `HunterCtrl`, `NpcManager`, `FieldNpcManager`, `VillageNewsCtrl`, `VillageNewsPop`, `ChatManager`, `ChatPop`.

**Direct scene names:** `BuildGroup`, `BuildSelect`, `Building`, `BuildButton`, `CharacterButton`, `ArchiveButton`, `StoreButton`, `RaidButton`, `ChattingView`, `StorageView`, `TopView`, `BottomView`.

**Representative assets:**

- `background_01__1548.png`, `background_02__1515.png`, `background_05__1522.png`, `background_06__1547.png`, `background_07__1533.png`, `background_08__1530.png`, `background_11__1508.png`, `background_12__1519.png`, `background_13__1506.png`, `background_14__1541.png`, `background_15__1542.png`, `background_16__1517.png`, `background_17__1516.png`, `background_18__1535.png`
- `menu_bg_9__5116.png`, `menu_bg_t_9__3270.png`, `menu_ic_01__6756.png` through `menu_ic_05__6398.png` and their `_t` variants
- `open_menu_bg_9__3135.png`, `open_menu_top_bg_9__3281.png`, `open_menu_top_on_9__3577.png`, `open_menu_top_off_9__3425.png`, `open_menu_top_off2_9__3822.png`
- `bg_VillageNews_UI__4442.png`
- `btn_build_one_9__2921.png`, `btn_build_l_9__5323.png`, `btn_build_r_9__3540.png`, `btn_build_top_box_10__3717.png`, `build_arrow_01__6174.png` through `build_arrow_04__6480.png`, `build_check__4870.png`, `build_cancel__2697.png`, `build_turn__1375.png`
- `top_ic_00_bg_9__3155.png`, `top_ic_01_gold_24__4677.png`, `top_ic_02_gem_24__4214.png`, `top_ic_quest__4944.png`, `top_ic_man_bg_9__2450.png`

**Confidence:** high that the game is village/building-centric and has persistent top-resource/bottom-menu UI; exact main-village background selection and camera framing are not yet proven.

### 3. Hunter roster, detail, growth, jobs, equipment

**Observed classes:** `HunterInfoList`, `HunterDetailPop`, `HunterGrowUpPop`, `HunterGrowUpPropertyPop`, `HunterSkillPop`, `HunterGearDetailPop`, `HunterRevivePropertyPop`, `HunterSelectPop`, `HunterSortPop`, `HunterThumbUIFormFactor`, `HeroicJobPop`, `JopChangeResultPop`, `GearStoragePop`, `UserGearDetailPop`, `GearGrowPop`, `GearCreatePop`, `GearDisassemblePop`, `GearSkillPop`, `GearSlotEngravingPop`.

**Representative assets:**

- `hunter_area_bg__6171.png`, `hunter_shadow__5096.png`, `character_toggle_bg__3724.png`, `character_toggle_off__6544.png`
- `character_info__7588.png`, `character_graph0__3588.png` through `character_graph5__5089.png`, `character_star_on__2860.png`, `character_star_off__1668.png`
- `character_class_box_ 01__1893.png`, `character_class_box_ 02__7365.png`, `character_class_box_ 03__3653.png`, `character_class_namebox_0__8405.png` through `_2__564.png`
- `hero_job_bg_01__1686.png`, `hero_job_bg_02__2996.png`, `aca_2job_bg_01__2692.png`, `aca_2job_bg_02__5254.png`, `aca_3job_bg_01__4541.png`, `aca_3job_bg_02__4587.png`
- `ic_hunter_gear_0__691.png` through `ic_hunter_gear_15__317.png` (not every number is contiguous in the export)
- `equip_bg_9__2684.png`, `equip_sel_bg_9__2167.png`, `equip_gold_bg_9__3649.png`, `weapon_info_icon__2337.png`, `costume_info_icon__4922.png`, `gearskill_info_icon__2871.png`
- `growth_top_bg__3232.png`, `levelup_pop_content_box__2697.png`, `levelup_pop_relic_box__2635.png`, `levelup_pop_grow_gage_2__202.png`

The hunter atlas and Spine bundle are separate high-confidence actor evidence; UI composition must use the roster/detail assets above rather than the current single-Hunter HUD.

### 4. Field, hunting, combat, death and revival

**Observed classes:** `MapManager`, `HunterCtrl`, `EvilCtrl`, `DamageManager`, `DamageCtrl`, `DamageEffectCtrl`, `HunterPatternCtrl`, `TargetBuildData`, `ReviveBuildingCtrl`, `ReviveHunterInfoList`, `ReviveHunterInfoRowList`, `FieldNpcCtrl`, `FieldBossPop`, `FieldBossLevelPop`, `FieldBossRewardPop`.

**Direct scene evidence:** `WorldCanvas`, `CharacterView`, `EnemyGroup`, `EvilGroup`, `HpBar`, `StatusGroup`, `EnemyDamageBorder`, `MyDamageBorder`, `NextStage`, `Revive`, `WaitUpButton`, `UIAdventureDamageCanvas`, `UIRaidCanvas`.

**Representative assets:** `hunter_area_bg__6171.png`, `top_mon_lv_bg__1740.png`, `hp_lv_bg_9__3211.png`, `ingame_btn_deco__2187.png`, `dps_timer_icon__2595.png`, `dps_detail_icon__2953.png`, `revive`-named sprites from `game-assets/asset-index.json`, plus the original actor/monster bundles documented in `docs/migration/slice-1-legacy-dossier.md`.

**Important correction:** the extracted class/scene evidence supports multiple combat modes, but it does not bind `mon_a_01_1` or Goldblin to the original first field scene. The current technical fixture must not be presented as the legacy field.

### 5. Adventure / expedition

**Observed classes:** `AdventurePop`, `AdventureMapPop`, `AdventureInfo`, `AdventureHunterCtrl`, `AdventureHunterList`, `AdventureNpcPop`, `AdventureNpcList`, `AdventureEvilCtrl`, `AdventureMapPop`, `AdventureUserData`, `RaidSelectMapPop`.

**Direct assets:**

- `AdventureMap_01__3469.png` through `AdventureMap_04__7223.png`
- `AdventureMap_devil_front_yard__2338.png`, `AdventureMap_ic_town__4798.png`, `AdventureMap_ic_targetpoint_0__8982.png` through `_2__434.png`
- `AdventureMap_monster_01_dragon_*` through `AdventureMap_monster_11_shark_*`
- `Adventure_node_01_*`, `Adventure_flag_*`, `Adventure_box_*`, `Adventure_Event_*`, `Adventure_End_*`, `Adventure_Search_ic__6812.png`, `Adventure_time_box__4285.png`
- `adven_map_frame__5362.png`, `adven_team_make_bg_top__2526.png`, `adven_team_make_bg_bot__2296.png`, `adven_team_make_btn__5049.png`, `adven_pop_bg_top_9__2910.png`, `adven_pop_bg_bot_9__1608.png`, `adven_trip_result_btn__1461.png`

This is a distinct map-and-party flow, not a reskinned village combat panel.

### 6. Raid, field boss, world boss, and PvP

**Observed classes:**

- Raid: `RaidSelectPop`, `RaidSelectMapPop`, `RaidSelectMapList`, `RaidHunterCtrl`, `RaidLogPop`, `RaidMoreInfoPop`, `RaidWaitHunterList`.
- Field/world boss: `FieldBossPop`, `FieldBossRewardPop`, `WorldBossPop`, `WorldBossDetailDamagePop`, `WorldBossResultPop`, `WorldBossRankList`, `WorldBossHunterCtrl`.
- PvP/colosseum: `PvPEntryPop`, `PvPSelectPop`, `PvPHunterCtrl`, `PvPRankPop`, `PvPInfo`, `PvPStatusData`, `ColoEntryPop`, `ColoRewardPop`.

**Representative assets:** `dg_bg_img_01__1376.png`, `dg_bg_img_05__1839.png`, `adventure_dg_bg_img_00__7606.png` through `_04__6855.png`, `wb_bg_huntcount__3003.png`, `wb_title_bn1__1597.png` through `wb_title_bn4__1715.png`, `wb_skill_deco_bg__2453.png`, `pvp_bg_01__2578.png`, `pvp_bg_2__7660.png`, `pvp_bg__7042.png`, `pvp_form_bg__3342.png`, `pvp_fight_bg__5211.png`, `pvp_result_win__6632.png`, `pvp_result_lose__3797.png`.

Each mode has its own selection/result/detail surfaces. Do not route all modes through the Slice 1 combat HUD.

### 7. Shop, currency, rewards, mail, quests and social

**Observed classes:** `ShopPop`, `ShopTopMenuCtrl`, `ShopItemList`, `ShopItemDetailPop`, `ShopGearDetailPop`, `LuxuryBuyPop`, `TokenShopPop`, `MailPopV3`, `MailListV3`, `QuestPop`, `QuestList`, `MissionPop`, `AchievePop`, `GuildPop`, `GuildList`, `GuildMemberList`, `GuildBattleEntryPop`, `GuildBattleRankList`, `ChatPop`, `ChattingList`, `RequestPop`.

**Representative assets:** `shop_menu_bg_9__5179.png`, `shop_menu_top_bg_9__1941.png`, `shop_menu_on_9__6765.png`, `shop_menu_off_9__3355.png`, `shop_pop_bg_top_9__5616.png`, `shop_pop_bg_bot_9__1479.png`, `shop_product_00__6979.png` onward, `shop_01_recommend_*`, `shop_02_jewel_*`, `shop_03_bonus_*`, `chat_bg_9__3721.png`, `guild_ui_bg__2448.png`, `guild_ui_title__2323.png`, `guild_list_bg__6672.png`, `quest_box_01__1440.png`, `reward_on_bg__3862.png`, `reward_off_bg__5012.png`, `setting_bg_9__3704.png`.

These flows should be implemented after the core village/roster flow and backed by independent server contracts. The asset names do not prove prices, rewards, rates, or remote API semantics.

## Required A-to-Z migration order

1. **A: Evidence lock.** Preserve the recovered `level1` hierarchy/transform/Canvas evidence, then extend extraction with sprite/text/layout/event bindings, renderer sorting, colliders, camera bounds, and localization keys. Record hashes for every generated evidence file.
2. **B: Boot/title.** Reproduce intro/loading/login/localization and first-run save identity using the intro assets above.
3. **C: Village shell.** Rebuild camera, backgrounds, top resources, bottom menu, buildings, NPC touch targets, news/chat/storage overlays.
4. **D: Hunter roster.** Implement list/detail/growth/jobs/equipment UI using the identified frames and the real hunter composition data once observed.
5. **E: Field combat.** Bind a verified original field actor/monster/map; keep server authority while matching the original HUD, target/HP/status/damage layout.
6. **F: Adventure.** Implement the map, node, party setup, events, travel timer, and result flow using the Adventure assets.
7. **G: Raid/boss/PvP.** Add each mode as a separate screen/state machine and visual asset set.
8. **H: Economy/social.** Add shop, mail, quests, guild, chat, rank, ads/purchase placeholders with locally controlled educational backend contracts.
9. **I: QA.** Run only basic build/startup smoke per the current request, then compare each screen against captured legacy references. Do not mark a screen complete from compile success alone.

## Current mismatch register

| Current web behavior | Legacy evidence | Required correction |
| --- | --- | --- |
| Training Ground title and generic combat panel | Intro/village/menu/field canvas objects and hundreds of named UI sprites | Replace entry route with boot -> village flow |
| Hunter + `mon_a_01_1` fixture | Original catalog contains many monster/field/adventure assets; actor binding is unresolved | Use a fixture label in development only; block compatibility claims |
| Manual inventory/equip HUD as primary UX | Legacy has dedicated roster/detail/gear popups and menu assets | Implement the original navigation hierarchy first |
| Client-visible revive action | Scene has `Revive`, `WaitUpButton`, revive-property/building classes | Reconstruct exact revive screen/trigger before exposing control |
| One canvas for all interactions | `MainCanvas`, `WorldCanvas`, `UIPopupCanvas`, `UIAdventureCanvas`, `UIRaidCanvas`, `FocusCanvas` | Preserve separate render/UI layers and mode-specific canvases |

## Phase gate

The machine-readable scene-graph part of Phase A is now complete: UnityPy recovered 23,286 GameObjects, 23,286 transforms, and 16 canvases from `level1`. Phase A/B remains open until component bindings and at least one captured reference exist for boot, village, roster, field, adventure, and one result popup. Until those remaining items are available, the web implementation may use the recovered topology but must not claim pixel-perfect visual migration.
