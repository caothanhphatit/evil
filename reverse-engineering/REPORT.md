# Evil Hunter Tycoon 1.411 - Reverse Engineering Report

## 1. Executive summary

Goi phan tich la `Evil Hunter Tycoon 1.411`, package `com.superplanet.evilhunter`, phat hanh duoi dang XAPK gom base APK, Unity asset split va native ARM64 split.

Day la game Unity IL2CPP. Phan Android/Java chi la bootstrap, SDK quang cao, analytics, billing va bridge goi vao Unity. Logic game chinh da duoc AOT-compile tu C# vao `libil2cpp.so`; metadata type nam trong `global-metadata.dat`.

Ket luan ve nhan dinh "backend nam trong APK":

- APK chua phan lon **gameplay client**, UI flow, simulation, cac bang can bang `Admin*Data`, save model va danh sach request/API route.
- APK khong chua mot backend server hoan chinh: khong co server executable, database schema/runtime, private signing key hay code deployment server.
- Guild, chat, rank, PvP, world boss, purchase verification va dong bo tai khoan co dau hieu phu thuoc remote API.
- Mot ban offline/web clone co the viet lai tu client data; mot ban tuong thich online voi server goc khong the tai tao chi bang APK.

## 2. Input and integrity

- XAPK SHA-256: `69c74073dbe3fc67d7b228a6f9fe5ad34f352f7faa3552a1500fc76b731015c3`
- Base APK SHA-256: `5f9c3ac8b5373edcf4d61cae3dff7c4aea23aee173f6977ad392fd809bc0053b`
- Asset split SHA-256: `01d6f3784b81609d27bc647ed1a9763cdeaa85168cb186a02e7607343e426de5`
- ARM64 split SHA-256: `f4aa492ea7be9944c71251e7abbfde14f2dc9cb8e08a2bfe4b3416c46fea1398`
- Version: `1.411`, version code `26071501`
- Min SDK: 25; target SDK: 35
- Unity build identified in native symbols: `6000.3.9f1`
- IL2CPP metadata version: `39`

## 3. Recovered code surface

### Android/JVM layer

JADX recovered approximately 21,204 Java files. Almost all belong to third-party libraries:

- Google Play Services, Firebase, Play Billing and Play Asset Delivery
- Google Mobile Ads
- AppLovin MAX
- Pangle/ByteDance
- Unity Ads
- Facebook SDK
- Singular attribution
- AndroidX, Kotlin, OkHttp and supporting libraries

Only a generated resource class appears directly under `com.superplanet.evilhunter`. This confirms the game was not implemented primarily in Java/Kotlin.

### Unity/C# layer

`ScriptingAssemblies.json` identifies the original managed assemblies, including:

- `Assembly-CSharp.dll`
- `Assembly-CSharp-firstpass.dll`
- `ACTk.Runtime.dll`
- `BestHTTP.dll`
- `EnhancedScroller_asmdef.dll`
- `Google.Play.*`
- `GoogleMobileAds.*`
- `SingularSDK.dll`
- `Firebase.*`
- `Unity.Purchasing.*`
- `spine-csharp.dll` and `spine-unity.dll`

Unity serialized `MonoScript` records exposed 4,435 script definitions. Of these, 921 records belong to `Assembly-CSharp`, representing approximately 920 unique game classes.

The complete recovered class catalog is stored in `evidence/assembly_csharp_classes.txt` and the full script/namespace/assembly mapping is in `evidence/monoscripts.csv`.

## 4. Obfuscation and protection

The application uses several protection layers:

1. Java SDK code is minified with R8/ProGuard-style names such as `zza`, `zzb` and numbered classes.
2. Game C# is compiled with IL2CPP into a stripped 100 MB ARM64 ELF binary.
3. Local values and saves use Code Stage Anti-Cheat Toolkit (`ACTk.Runtime`).
4. `global-metadata.dat` has a valid Unity metadata header/version but its internal section/string layout does not load normally in current LibCpp2IL.
5. Current Cpp2IL detects the failure and invokes its Mfuscator metadata repair plugin, but this build uses a transform not recognized by the plugin's known header patterns.

Consequently, class and assembly names are recoverable, but complete method bodies cannot yet be reliably reconstructed into buildable C#. Any generated C# would be pseudocode/stubs, not the original source.

No source comments, original local-variable names, project layout, prefabs-as-code or Git history can be recovered from IL2CPP.

## 5. Gameplay architecture

The recovered game class catalog shows the following major systems.

### Village and buildings

Representative classes:

- `BuildManager`, `BuildCtrl`, `BuildingData`
- `BuildInfo`, `BuildList`, `BuildSelectCtrl`
- `BuildSkinChangePop`, `BuildingReviveCheckPop`
- `AdminBuildData`, `AdminBuildSkinData`, `AdminReviveBuildingData`

This suggests a data-driven village model where building definitions and costs come from admin tables while scene controllers handle placement, interaction and UI.

### Hunters

There are at least 55 hunter-named classes:

- `HunterManager`, `HunterCtrl`, `HunterData`
- `HunterDetailPop`, `HunterSkillPop`, `HunterGrowUpPop`
- `HunterRevivePropertyPop`, `ReviveHunterInfoList`
- `HunterRaidDropUtility`, `HunterSortDropUtility`
- `AdminHunterData`, hunter name and speech tables

Hunters have jobs, traits, skills, gear, growth, revival, sorting and multiple mode-specific controllers.

### Combat and damage

Representative classes:

- `DamageManager`, `DamageCtrl`, `DamageEffectCtrl`
- `AreaEffectCtrl`, `ArrowCtrl`, `BlizzardCtrl`
- `DamageTestHunterCtrl`, `DamageTestStatusData`
- multiple boss, raid and PvP controllers

Combat appears substantially client-simulated. Server-backed modes may validate or upload results rather than simulate every frame remotely.

### Adventure, raid and bosses

- 20 adventure-named classes
- 25 raid-named classes
- 18 world-boss-named classes
- classes for fallen pasture/ranch, rift raid, field boss and new raid content

Examples include `AdventureHunterCtrl`, `RaidHunterCtrl`, `WorldBossHunterCtrl`, and corresponding `Admin*Data` reward/stage tables.

### Guild, social and competitive systems

- `GuildManager`, `GuildPop`, `GuildMemberList`
- `GuildBattleRankList`, `GuildBattleMatchTeamList`
- `GuildBattleGiveUpPop`, `GuildBattleEntryPop`
- `ChatManager`, `ChatPop`, `ChattingList`
- PvP, colosseum, ranking and union-related routes/strings

These are strong indicators of remote persistent state: membership, rankings, matchmaking, chat and world-state cannot be authoritative in a single APK.

### Items, crafting and economy

The catalog includes:

- `AdminItemData`, `AdminGearData`, `AdminGearPropertyData`
- `AdminShopData`, `AdminLuxuryShopContentsData`
- `AdminProductData`, `AdminInAppRewardData`
- rune crafting, cube composition/conversion, consumable creation, exchange and trader systems
- gear sets, engraving, costumes, relics, riding pets and limit breaks

Most content/economy rules appear data driven. This is the portion most reusable when recreating an educational offline simulation.

### Missions, quests and progression

- `AdminQuestData`, `AdminMissionData`, `AdminAchieveData`
- quest, achievement, attendance, mileage, honor and rank reward UI/data classes
- `AdminExpData`, `AdminGrowUpData`, `AdminUnitCreateData`

## 6. Local data and save system

Recovered first-party classes include:

- `SaveData`
- `TestSaveButton`
- `RequestData` and request list/info classes
- numerous `*UserData`, `*StatusData` and `Admin*Data` types

Google Play Games Saved Games is bundled, and ACTk provides:

- `ObscuredPrefs`
- `ObscuredFile` and `ObscuredFilePrefs`
- `ObscuredFileCrypto`
- binary/JSON serialization helpers
- device lock and device identifier support

This indicates local state is likely encrypted/obscured, with optional cloud/account synchronization. Presence of `SaveData` does not mean every authoritative online value is local.

## 7. Networking and backend boundary

The client bundles BestHTTP, TLS/BouncyCastle, SignalR, Server-Sent Events and Socket.IO-related types. Recovered metadata strings include fragments of routes for:

- authentication and restore
- guild membership, requests, search and modification
- union registration and information
- arena/PvP
- battle-field rankings
- adventure matching and save
- mail and rewards
- world boss and other world content
- in-app purchase handling

Many route strings are partially corrupted because the metadata string section is protected. The raw recovered fragments are preserved in `evidence/api_routes.txt`.

Architectural interpretation:

- Local/client: rendering, movement, most battle presentation, UI, content tables and probably portions of combat simulation.
- Hybrid: save state, adventure results, ranking submission, cloud restore and purchase state.
- Remote/authoritative: auth, guild, chat, rankings, matchmaking, shared world events and receipt verification.

There is no evidence that the production backend implementation itself is packaged in the client.

## 8. Anti-cheat

The game includes Code Stage Anti-Cheat Toolkit with at least 137 script definitions:

- obscured numeric/string/vector types
- `SpeedHackDetector`
- `TimeCheatingDetector`
- `WallHackDetector`
- `InjectionDetector`
- `ObscuredCheatingDetector`
- code/file hash generation
- Android installation-source validation
- encrypted file/prefs storage

Strings such as `/system/bin/su` and `/data/local/tmp` are consistent with root/debug/integrity checks in Unity and advertising/anti-cheat libraries. They are not by themselves evidence of malicious behavior.

## 9. Monetization and telemetry

The app integrates:

- Unity Purchasing and Google Play Billing
- Google Mobile Ads mediation
- AppLovin, Pangle and Unity Ads
- Firebase Analytics, Messaging and Crashlytics
- Facebook SDK
- Singular attribution

The app requests advertising ID and Android Privacy Sandbox ad-service permissions. Privacy exposure is higher than a minimal offline game, but consistent with an ad-supported mobile title.

## 10. Assets recovered

The asset inventory contains approximately:

- 9,030 Texture2D/Sprite objects
- 116 AudioClip objects
- 106 TextAsset objects
- hundreds of animation clips/controllers
- Unity scenes/prefabs represented by over 100,000 serialized objects in `level1`
- Addressables localization bundles for English, Japanese and Chinese

Raw joined Unity files, exported PNG/audio/text assets and metadata inventory are under the extracted asset directory. Sprite export was intentionally stopped after more than 3,000 files because the requested grave/death assets had already been recovered.

## 11. Feasibility of a web reconstruction

A clean-room educational web reconstruction is feasible if it targets behavior rather than binary compatibility.

Recommended architecture:

- TypeScript plus Phaser 3 or PixiJS for rendering
- ECS or component-based simulation for hunters, monsters and projectiles
- JSON versions of `Admin*Data` tables
- deterministic combat simulation separated from presentation
- IndexedDB/local storage for offline saves
- optional Node/PostgreSQL backend for accounts, guilds and rankings

Suggested implementation order:

1. Asset catalog and animation mapping
2. Village scene and building interactions
3. Hunter/monster state machines
4. Combat, damage, loot and revival
5. Inventory, gear and progression
6. Offline save model
7. Quests, raids and world-boss simulations
8. A new educational backend for social/ranking features

Do not depend on the original production API. Reimplement interfaces and data models against a locally controlled backend.

## 12. Limitations and next research steps

Current hard blocker for full pseudo-C# recovery is the protected metadata v39 layout. Practical next steps are:

1. Capture metadata after runtime decryption from an authorized emulator/device process.
2. Identify the metadata loader/decryption routine inside `libil2cpp.so` using Ghidra/IDA and trace references to the metadata magic/header.
3. Patch or extend Cpp2IL's Mfuscator plugin for this specific transform.
4. Feed the decrypted metadata back into current Cpp2IL to produce `diffable-cs`, dummy DLLs and IL recovery output.
5. Correlate recovered classes with Unity `MonoBehaviour` instances and serialized fields to rebuild data schemas.

Even after this succeeds, recovered C# will remain an approximation of compiled behavior rather than the original project source.

## 13. Evidence locations

- Decompiled Android layer: `/Users/trana/Downloads/Evil_Hunter_Tycoon_1.411_decoded`
- Extracted assets: `/Users/trana/Downloads/Evil_Hunter_Tycoon_1.411_assets`
- Mining output/evidence: `/Users/trana/Downloads/Evil_Hunter_Tycoon_1.411_CSharp_mined`
- MonoScript catalog: `evidence/monoscripts.csv`
- First-party class list: `evidence/assembly_csharp_classes.txt`
- Recovered URL strings: `evidence/urls.txt`
- Recovered API fragments: `evidence/api_routes.txt`

