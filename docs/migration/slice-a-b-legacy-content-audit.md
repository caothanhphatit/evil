# Phase A/B Legacy Content Audit

## Scope and evidence boundary

This audit covers the locally available Evil Hunter Tycoon 1.411 XAPK analysis outputs. It is a static, clean-room inventory: no original production service was contacted and no credentials, tokens, or proprietary server implementation were reused. It records what can be implemented now and what still requires runtime observation or a Unity-compatible serialized-data extraction.

Primary evidence:

- `reverse-engineering/REPORT.md` (APK/XAPK integrity, IL2CPP findings, protection, networking interpretation)
- `reverse-engineering/evidence/assembly_csharp_classes.txt` (919 unique class names; approximately 920 Assembly-CSharp types)
- `reverse-engineering/evidence/monoscripts.csv` (4,436 MonoScript records with path ID, namespace, and assembly)
- `reverse-engineering/evidence/api_routes.txt` (partially recovered route fragments)
- `game-assets/extracted/exported/metadata/inventory.json` (Unity object inventory)
- `game-assets/asset-index.json` (9,359 exported files, 190,429,626 bytes)
- `/Users/trana/Downloads/Evil_Hunter_Tycoon_1.411_decoded/resources/AndroidManifest.xml` and `xapk_manifest.json`
- `/Users/trana/Downloads/Evil_Hunter_Tycoon_1.411_CSharp_mined/REPORT.md` (same report in the immutable mining output)

Confidence labels are `confirmed` (direct artifact evidence), `strongly inferred` (multiple independent structural signals), `tentative` (single naming/string signal), and `unknown` (not recoverable from current artifacts).

## Artifact and platform facts

| Fact | Evidence | Confidence | Migration consequence |
| --- | --- | --- | --- |
| Package is `com.superplanet.evilhunter`, version `1.411`, version code `26071501` | `xapk_manifest.json`, `AndroidManifest.xml` | confirmed | Keep source compatibility metadata, but use a new web package identity. |
| XAPK contains base APK, `base_assets`, and ARM64 split; total size 346,656,029 bytes | `xapk_manifest.json` | confirmed | Asset migration must account for split delivery and addressable content. |
| Game logic is Unity IL2CPP, Unity `6000.3.9f1`, metadata v39 | `reverse-engineering/REPORT.md` | confirmed | Decompiled C# is a type/behavior reference, not buildable source. |
| Managed layer contains `Assembly-CSharp.dll`, Spine, BestHTTP, localization, addressables, billing and analytics assemblies | `reverse-engineering/metadata/ScriptingAssemblies.json` | confirmed | Rebuild needs independent domain modules; do not copy third-party runtime assumptions. |
| Current metadata transform prevents reliable method-body recovery | `reverse-engineering/REPORT.md` | confirmed | Formula/rate claims must not be derived from guessed pseudocode. |

The Android manifest requests network, billing, ad ID/ad-services, notifications, wake lock, storage, Firebase messaging, foreground sync, vibration and Wi-Fi permissions. These are evidence of integrations, not evidence that gameplay authority resides in the APK. See `xapk_manifest.json` and `resources/AndroidManifest.xml` for the complete list.

## Recoverable domain catalog

The following domains are implementable as clean-room modules now at the shape/flow level. Numeric rules, IDs, and exact persistence semantics remain separate evidence tasks unless explicitly marked otherwise.

### Village, buildings, and map

Confirmed first-party types include `BuildManager`, `BuildCtrl`, `BuildingData`, `BuildInfo`, `BuildList`, `BuildSelectCtrl`, `BuildSkinChangePop`, `BuildSkinCreatePop`, `BuildingCompCheckPop`, `BuildingDeletePop`, `BuildingPop`, `BuildingReviveCheckPop`, `AdminBuildData`, `AdminBuildSkinData`, `AdminReviveBuildingData`, `AdminMapData` through `AdminMapData4`, `AdminVillSkinData`, `MainCameraData`, and `MapEditor` (`assembly_csharp_classes.txt`; `monoscripts.csv`).

The Unity inventory contains 26,853 `GameObject`s, 19,411 `RectTransform`s, 7,442 `Transform`s, 103 `SortingGroup`s, 105 `CircleCollider2D`s, 32 `BoxCollider2D`s, one `NavMeshSettings`, and named assets such as `map_new01`, `Pasture`, `devil_cloister_map`, `background_01`-`background_18`, `build_*`, `buildSkin_*`, and `build_run_*` (`metadata/inventory.json`). This confirms substantial scene/content material but does not yet reconstruct hierarchy, collision, sorting, or spawn coordinates.

Status: scene visual content `confirmed`; scene behavior and layout `strongly inferred`; exact map geometry/spawn/nav data `unknown`.

### Hunters, jobs, growth, and equipment

Confirmed types include `HunterManager`, `HunterCtrl`, `HunterData`, `HunterPatternCtrl`, `HunterDetailPop`, `HunterSkillPop`, `HunterGrowUpPop`, `HunterRevivePropertyPop`, `HunterSortDropUtility`, `HunterRaidDropUtility`, `AdminHunterData`, `AdminHunterNameMData`, `AdminHunterNameWData`, `AdminHunterSpeachData`, `AdminJobTraitData`, `AdminSkillData`, `AdminSubJobSkillData`, `AdminGrowUpData`, `AdminExpData`, `AdminPropertyData`, `AdminUnitData`, and `AdminUnitCreateData`.

The original hunter Spine bundle is present as `hunter.json`, `hunter.atlas`, and `hunter` texture (`sharedassets1.assets` path IDs 245, 258, and 166; exported copies under `game-assets/extracted/exported`). Its 70 animation names and 1,937 skin entries establish a modular actor pipeline. `All_h1` and `weapon_h1a_a_01` are valid visual skin names; they do not prove starter composition or gameplay IDs.

Status: actor rendering/animation schema `confirmed`; class/domain boundaries `strongly inferred`; starter loadout, stats, progression formulas, and job assignment `unknown`.

### Monsters, combat, skills, and effects

Confirmed types include `DamageManager`, `DamageCtrl`, `DamageEffectCtrl`, `DamageEffectRaidCtrl`, `AreaEffectCtrl`, `ArrowCtrl`, `BlizzardCtrl`, `AdventureEvilCtrl`, `RaidEvilCtrl`, `WorldBossEvilCtrl`, `FieldBossData`, `AdminFieldBossData`, `AdminEvilData`, `AdminDemonData`, `AdminDemonSkillData`, `AdminWorldBossSkillData`, `AdminBuffData`, `AdminDarknessPrinceData`, and `AdminSkillData`.

The Unity inventory contains 650 `AnimationClip`s, 526 `AnimatorController`s, 486 `ParticleSystem`s, and 486 `ParticleSystemRenderer`s. Spine bundles exist for `mon_goldblin`, `mon_a_01_1`, `mon_a_01_2`, `mon_a_02_*`, `mon_a_03_*`, `mon_b_*`, `mon_c_*`, `mon_dg_*`, `mon_go_*`, `mon_new_*`, and world-boss actors (`metadata/inventory.json`). Goldblin exposes movement/death clips but no verified attack clip; `mon_a_01_1` is a technical attack-animation fallback only.

Status: actor/effect inventory `confirmed`; presence of combat subsystems `confirmed`; formulas, cadence, target selection, damage order, and content binding `unknown`.

### Items, gear, crafting, and economy

Confirmed types include `ItemData`, `GearData`, `GearBuyList`, `GearCreateList`, `GearGrowPop`, `GearLimitBreakThroughPop`, `GearDisassemblePop`, `GearConvertCubePop`, `GearChaosCubePop`, `GearOptionModificationPop`, `GearStoragePop`, `UpgradeGearList`, `StorageGearList`, `AdminItemData`, `AdminGearData`, `AdminGearPropertyData`, `AdminGearSetPropertyData`, `AdminGearSkillData`, `AdminDropUniqueGearData`, `AdminEngravingData`, `AdminEngravingDropData`, `AdminMagicCubeData`, `AdminRuneCraftData`, `AdminRunesData`, `AdminRelicInfoData`, `AdminRidingPetGear*`, and `AdminLimitBreakData`.

`DropGearData`, `DropGearTouchCtrl`, `ContentRewardItemData`, `RewardItemData`, `GachaData`, `ShopData`, `AdminShopData`, `AdminLuxuryShopContentsData`, `AdminProductData`, `TokenShopPop`, `TradeWagon*`, and `Trader*` establish separate definition, owned-instance, presentation, and transaction concepts. No current artifact binds `item_unique_01`, `coin 1`, `ic_hunter_gear_0`, or `uniquedrop` to a specific monster or drop rate.

Status: schema boundaries `strongly inferred`; visual candidates `confirmed`; item IDs, rarity, costs, rates, and formulas `unknown`.

### Quests, missions, dungeons, raids, and bosses

Confirmed types include `QuestList`, `QuestPop`, `AdminQuestData`, `MissionCtrl`, `MissionPop`, `AdminMissionData`, `AdminAchieveData`, `AchieveList`, `AdminAdventureData`, `Adventure*`, `AdminRaidData`, `Raid*`, `AdminNewRaidData`, `AdminRift*`, `AdminFallenRanch*`, `AdminWorldBossBoxData`, `WorldBoss*`, `FieldBoss*`, `AdminChestData`, and `AdminMoleTimeRewardData`.

Named map/background/audio/animation assets support multiple modes, but current exports do not contain a validated table-to-scene binding or stage/reward values. Implement mode shells only after each mode receives a fixture or runtime trace.

Status: feature inventory `confirmed`; progression gates, stage tables, rewards, timers, and matchmaking semantics `unknown`.

### Social, account, mail, ranking, and remote boundaries

Confirmed types include `LoginManager`, `LoginResult`, `GoogleAuthrisationHelper`, `ServerUtility`, `UniversalAPI`, `RequestData`, `RequestInfoList`, `RequestList`, `RequestCharacterList`, `GuildManager`, `Guild*`, `ChatManager`, `Chat*`, `MailListV3`, `MailPopV3`, `UserData`, `UserMailData`, `UserQuestData`, `UserAchieveData`, `RankList`, `BasicRankRewardList`, `DpsRank*`, `PvPRankPop`, `RiftRankPop`, and `WorldBossRankList`.

Route fragments in `reverse-engineering/evidence/api_routes.txt` include `auth/back`, `auth/logiyY`, `auth/restoQ`, `auth/sett`, `game/adventure/match_lid`, `adventure/save/game/battle_field/prev_rank/gQ`, `game/arena`, `game/guild/*`, `game/chat/*`, `game/mail/re`, `game/world_*`, `game/union/*`, and rank/battle fragments. Corruption means exact HTTP verbs, payloads, hostnames, and authentication headers are not recoverable from this file.

Status: remote account/social/rank boundary `strongly inferred`; route names `tentative` where strings are truncated/corrupted; server implementation `not present`.

### Monetization, advertisements, and telemetry

`AdsPop`, `InAppPurchaser`, `AdminProductData`, `AdminInAppRewardData`, Unity Purchasing/Google Play Billing classes, and the manifest billing/ad permissions confirm monetization. Google Mobile Ads, AppLovin MAX, Pangle/ByteDance, Unity Ads, Firebase Analytics/Messaging/Crashlytics, Facebook, and Singular are present in the managed/Android dependency surface.

These integrations are external boundaries. The rebuild must use a local educational stub for ads and purchases; it must not emulate receipt verification against the original service or ship original ad IDs/secrets.

Status: integration presence `confirmed`; placement/reward frequency and product catalog `unknown` unless separately observed.

## Data extraction priorities for phases A-Z

1. Reconstruct `level1` scene hierarchy and references with a Unity 6000-compatible reader; export building anchors, colliders, sorting groups, map bounds, spawn points, and camera limits.
2. Extract serialized rows or runtime dumps for every `Admin*Data` class used by the first playable flow: map, unit/hunter, monster/evil, build, item, gear, drop, quest, and shop.
3. Capture an authorized clean-account trace for boot, starter composition, village, first field target, attack presentation, drop, equip, death, revival, and save/restore. Keep screenshots/video separate from source assets and hash them.
4. Correlate runtime content IDs with exported Spine/texture/animation objects using at least two signals (serialized reference plus runtime observation, or two independent logs).
5. Extract localized tables and text keys from Addressables; validate English/Japanese/Chinese coverage before adding other locales.
6. Build golden traces before implementing each vertical slice; tag every numeric value as `legacy-verified`, `observed`, or `migration-fixture`.

## Explicit non-claims

- The APK does not contain a recoverable production backend executable or database.
- Class names do not reveal method bodies, formulas, IDs, or server authority by themselves.
- Asset filenames and Unity `path_id`s are not gameplay IDs.
- Current exported assets are source evidence, not proof that every screen, animation, or behavior is runnable in the web client.
- The current Training Ground fixture must remain labelled as a fixture until replaced by a trace-backed scene and content binding.
