# Hunter runtime schema capture v1

Date: 2026-07-26

## Result

An authorized Android 11 ARM64 emulator capture resolved the live IL2CPP schema
for the Hunter information, nested domain, manager, and collection boundaries.
The capture used Frida client/server `17.16.4` against package
`com.superplanet.evilhunter` version `1.411` (`26071501`). It enumerated schema
metadata only and did not read account values, arbitrary managed objects,
network traffic, or the original backend.

All requested classes were found. The runtime reflection result supersedes
protected-metadata field-count and type guesses where they disagree.

## Reproducible evidence

| Evidence | Bytes | SHA-256 |
| --- | ---: | --- |
| `reverse-engineering/evidence/hunter-info-runtime-schema-android-api30-v1.json` | 670,225 | `75f83a70ca6777d13fd78d1fcc89ddce52b7b605b6e80d31183af64c3742536e` |
| `reverse-engineering/evidence/hunter-domain-runtime-schema-android-api30-v1.json` | 480,976 | `9e35b239ab0ccf70cc9182db1d97f1f525f7d7e61b4094f4883821dfdc4ad558` |
| `reverse-engineering/evidence/hunter-manager-runtime-schema-android-api30-v1.json` | 362,053 | `759521d37fba94a69b407087e7c150e83db9778d948279cfe26dade66eee8f4c` |
| `reverse-engineering/evidence/hunter-collection-runtime-schema-android-api30-v1.json` | 39,788 | `bdc0de64fd0e4381f8c9f3dae1cb45022ef9b5a4f877821dde36c8e6ada69495` |
| `reverse-engineering/evidence/hunter-info-runtime-schema-android-api35-v1.json` | 670,307 | `6cc2faa575ed87567ed2262f2910a372596421d5d3b92264288639faa47da678` |

The deterministic host wrapper is
`tools/runtime/capture-hunter-info-schema.py`. Each evidence file records UTC
time, package/version, device ABI/model/API, Frida versions, exact action, PID,
script, and requested target types.

The API 35 primary capture was taken after a clean new-game session reached the
tutorial town and displayed Hunter `Sharon`. Its normalized class payload is
byte-for-byte equivalent to the API 30 primary capture after canonical JSON
serialization. See `docs/migration/android-api35-runtime-session-v1.md` for the
clean-launch and post-attach process-exit boundary.

## Corrected save boundary

Static protected metadata previously exposed `SaveData` as a one-field wrapper.
Runtime reflection proves the exact live shape is four fields and 23 methods:

| Field | Type | Offset |
| --- | --- | ---: |
| `index` | `System.String` | 16 |
| `data` | `System.String` | 24 |
| `action` | `System.Boolean` | 32 |
| `clear` | `System.Boolean` | 33 |

`SaveDataDic.data` is exactly
`Dictionary<System.String, SaveData>`. `GameManager` contains `mSaveData` of
that wrapper type plus a separate `hunterSaveData: System.String` field and
save-state booleans. This proves a string-oriented save staging boundary, but
not its encoding, encryption, file name, key convention, or cloud merge rules.

## Hunter ownership and collection shape

`GameManager` owns the live data aggregates:

- `mHunterData: HunterDataDic` for active Hunter data;
- `mHunterWaitData: HunterDataDic` for waiting Hunter data;
- `mVisitHunterData: VisitHunterDataDic` for visiting Hunters;
- `mHunterCtrl: Dictionary<String, HunterCtrl>` for active controllers;
- `mStatusData: Dictionary<String, StatusData>` for active status projections;
- `mUserData: UserData`, `mGearData: GearDataDic`, `mItemData: ItemDataDic`,
  `mConsumData: ConsumDataDic`, and `mSaveData: SaveDataDic`.

The collection wrappers are exact:

- `HunterDataDic.data` is `Dictionary<String, HunterData>`;
- `HunterDataDic.ridingPetData` is `Dictionary<String, RidingPetData>`;
- `VisitHunterDataDic.visitData` is `Dictionary<String, HunterData>`;
- `ItemDataDic.data` is `Dictionary<String, ItemData>`;
- `ConsumDataDic.data` is `Dictionary<String, Dictionary<String, ConsumData>>`;
- `ConsumDataDic.runesData` is `Dictionary<String, ConsumData>`;
- `ConsumDataDic.ridingPetGearData` is
  `Dictionary<String, Dictionary<String, RidingPetGearData>>`.

This resolves the active/wait collection and dictionary-value types. It does
not prove the semantic format of dictionary keys or the order in which the
collections are serialized.

## Exact per-Hunter state schema

`HunterData` has 109 fields and 236 methods. Important exact fields include:

- progression: `index`, `job`, `subJob`, `thirdJob`, `fourthJob`, `level`,
  `exp`, `personality`, `gradeRankUp`, `DSoul`, `UseDS`, `UseJT`;
- economy and vitals: `money`, `hp`, `nowHp`, `feel`, `nowFeel`, `hungry`,
  `nowHungry`, `tire`, `nowTire`;
- combat: `damage`, `armor`, `critical`, `attackSpeed`, `dodge`, rank fields,
  revive properties, state, pattern, area, and building/service state;
- appearance: `bodyIndex`, costume/fairy/weapon/wing/seal indices and hide
  flags, hat flags, ramble pet fields, and riding-pet linking fields;
- owned content: `gearInventory: Dictionary<String, GearData>`,
  `itemInventory: Dictionary<String, ItemData>`,
  `consumInventory: Dictionary<String, ConsumTotalData>`,
  `skill: Dictionary<String, SkillData>`, `JobTraitDic`,
  `GearSlotEngraving`, and `GUP_Property_LV: ObscuredInt[]`.

`HunterLookData` is exactly 11 strings: `acenum`, `acebody`, `acejob`,
`acecostume`, `acerevive`, `acernick`, `acehat`, `acesubjob`, `acewing`,
`acethirdjob`, and `acefourthjob`.

## Nested owned-data schemas

Runtime reflection resolves the previously unknown owned data shapes:

- `SkillData`: `index`, `skillIndex`, `coolTime`, `level`;
- `ItemData`: `newCheck`, `index`, `count`, `reservation`, `infinityCheck`;
- `ConsumTotalData`: total count plus `Dictionary<String, ConsumData>`;
- `GearData`: definition/inventory indices, quality, level, rating, group,
  rolled plus/minus arrays, buy data, lock/potential/rune values, and counters;
- `RidingPetData`: pasture/index/master/rating, skill, trait and trait level,
  soul/growth-stone usage, lock state, and pet gear inventory.

The table row schemas are also exact at runtime. Examples include
`subJobSkillData` cooldown/duration/value/cost fields, `jobTraitData` job/tree
position/prerequisite/value fields, `growupPropertyData.upvalue`, and the
riding-pet definition, skill, trait, and gear schemas.

## Generation and UI method boundaries

`HunterManager` exposes exact surviving method names and signatures including:

- `AddWaitHunter(String)`;
- `AddHunter(Int32, ObscuredInt, Boolean) -> HunterData`;
- `FixRangeOverBodyIndex(ObscuredInt) -> ObscuredInt`;
- `GetHunterDefaultMoney(Int32) -> ObscuredLong`;
- `getHunterSkin(SkeletonData, HunterData, Int32, Int32) -> Spine.Skin`;
- `UseHunterInvitation(ObscuredInt, ObscuredInt, Boolean, Int32)`.

`GameManager` exposes `getHunterData`, `getHunterWaitData`, `Save`,
`ThreadSaveString`, `GetHunterSkillIcon(Int32, Int32)`, and
`GetJobSkillDataIndex` with five integer-like inputs. These signatures narrow
the next trace targets but do not expose method bodies or argument semantics.

`HunterDetailPop` resolves 194 UI fields, including the exact skill title,
description, level, icon, inventory image, growth-row, pet skill/trait/icon,
pet gear, heroic skill, tab, status, and appearance widget arrays. This proves
the live component types, not the runtime array-index expressions.

## Remaining blockers

- No live `HunterData` instance values were read in this capture.
- Dictionary key semantics and the `SaveData.data` encoding remain unknown.
- The remaining skill row-to-icon mapping is not proven by method signatures.
- Growth costs/effects and learned state still require value capture.
- Riding-pet ownership values and gear contents still require value capture.
- Generation RNG distribution and exact `AddHunter` argument meanings remain
  unresolved.
- The API 30 Google APIs emulator returns to the Android launcher shortly after
  startup. A clean confirmation with non-root ADB and no Frida server produced
  the same exit, so the behavior is not caused only by the capture attachment.
  Logcat reports Google Play Services `201817019` while the package requests
  versions up to `213000000`. No Java exception or native fatal signal was
  emitted.
- The API 35 Google APIs emulator reaches the tutorial town and remains stable
  without Frida. A schema-only attach succeeded, but Android recorded the game
  process as `SIGNALED`, status `9`, about ten seconds later after a Frida child
  appeared as a dead phantom process. No Java/native fatal or ANR was logged.
  The exact exit cause remains unresolved.

Value capture therefore still requires an authorized ARM64 session that stays
stable after attachment, preferably a physical device as documented in the
runtime guide.

The next safe pass is a separately reviewed typed value dumper that captures
one known Hunter before and after one controlled action. It must use the exact
runtime types above and must not scan arbitrary managed memory.
