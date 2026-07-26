# Android runtime and Hunter save audit v1

Date: 2026-07-26

## Packaged Android/runtime inputs

The repository contains one original distribution package:

- `game-assets/source/Evil+Hunter+Tycoon_1.411_APKPure.xapk`
- XAPK SHA-256: `69c74073dbe3fc67d7b228a6f9fe5ad34f352f7faa3552a1500fc76b731015c3`
- package: `com.superplanet.evilhunter`, version `1.411`, version code
  `26071501` (from the existing verified reverse-engineering report)

The XAPK contains three APK splits rather than an AAB:

| Entry | Size | Role |
| --- | ---: | --- |
| `com.superplanet.evilhunter.apk` | 16,842,564 | base Android/bootstrap APK |
| `base_assets.apk` | 193,856,915 | Unity assets/data split |
| `config.arm64_v8a.apk` | 135,956,550 | ARM64 native split |

No `.aab` is present. The extracted Unity runtime is Unity `6000.3.9f1`, IL2CPP
metadata version 39. The two inputs needed for native correlation are present:

- `game-assets/source/unity-assets/bin/Data/Managed/Metadata/global-metadata.dat`
  (15,315,640 bytes)
- `reverse-engineering/native-libs/arm64-v8a/libil2cpp.so`
  (104,557,288 bytes)

The native library is stripped ARM64. Existing evidence identifies code
registration VA `0x5D99D98`, metadata registration VA `0x5F3EC28`, and 38,247
Assembly-CSharp method pointers. The protected metadata v39 transformation
still prevents reliable general-purpose C# reconstruction.

## Save/runtime surface

The package contains all of the following generic persistence facilities:

- Unity `PlayerPrefs` native bindings;
- Unity `Application.persistentDataPath`;
- Unity `JsonUtility` `ToJson`/`FromJson` native bindings;
- Code Stage Anti-Cheat `ObscuredPrefs`, `ObscuredFile`,
  `ObscuredFilePrefs`, crypto/header/settings and serializers;
- GPM MessagePack and LZ4 MessagePack implementation types;
- Google Play saved-game/cloud-related libraries noted in the prior report.

This is a library inventory, not a proven call graph. There is no player save,
PlayerPrefs XML, ACTk obscured file, cloud-save response or Android app data
directory in the repository. Therefore the original filename, key names,
encryption settings, serializer choice and cloud/local merge rules remain
unknown. In particular, dependency presence must not be converted into a
claim that the game saves HunterData with MessagePack, JSON or PlayerPrefs.

## Static Hunter serialization boundary

`tools/extract-hunter-save-evidence.py` performs a light, deterministic pass
over the protected metadata and checks exact runtime API markers. It records
four targeted Assembly-CSharp records without attempting decryption.

### `SaveData` wrapper

Metadata type index 521 has a surviving `SaveData` prefix, one field and 22
methods. Its only field name is poisoned, but nearby accessor fragments include
`set_dat...` and backing-field-like `...data>k__B...` text. This supports a
small wrapper/projection around another data object. The field's IL2CPP type
index is 5512, but that index cannot be mapped safely until native metadata
registration is recovered.

### `UserData` aggregate

Metadata type index 5 is cleanly named `UserData` and contains 527 fields and
1,019 methods. Surviving fields include two `EntryHunter...` fragments,
`SaveFormet...`, and two `HunterPack...` fragments. This is the stronger
durable player aggregate boundary. Static metadata does not yet reveal whether
the Hunter collection is a list, dictionary, array or a transformed DTO.

### `HunterData` instance snapshot

Metadata type index 1587 remains the strongest `HunterData` match: 109 fields
and 236 methods. The surviving field/accessor evidence covers:

- job/sub-job/fourth-job, money, position, area index and grade rank;
- current combat/state values, revive state and hunting/building occupancy;
- costume, hat/costume visibility, fairy, weapon/wing/seal costume, riding and
  body index;
- gear, item, consumable, skill and `JobTraitDic`;
- characteristic/personality-like state.

This is sufficient to design a clean-room per-Hunter durable snapshot. It is
not sufficient to read or write the original binary save format.

### `HunterLookData` projection

Metadata type index 1972 has 11 fields and a surviving `HunterLook...` prefix.
Readable fragments include `acenum`, `acebody`, weapon/costume-like fields,
`acerevive`, `acesubjo...` and `acewing`. It is a separate compact look
projection, but its consumer and exact field types remain unresolved.

## Existing tools and evidence

Relevant reusable tools:

- `tools/extract-il2cpp-hunter-generation.py`: protected-v39 Hunter type,
  field and method correlation;
- `tools/extract-hunter-generation-tables.py`: serialized class/stat and
  Characteristic tables;
- `tools/extract-hunter-name-pools.py`: embedded male/female QuickSheet pools;
- `tools/extract-hunter-info-tables.py`: Hunter info tables needed by UI;
- `tools/extract-hunter-save-evidence.py`: targeted save boundary added by
  this audit.

Generated evidence:

- `reverse-engineering/evidence/hunter-save-serialization-v1.json`
- `reverse-engineering/evidence/il2cpp-hunter-generation-v1.json`
- `reverse-engineering/evidence/hunter-generation-tables-v1.json`
- `reverse-engineering/evidence/hunter-info-tables-v1.json`

## Recommended next steps

1. Use the recovered field categories to define the rebuild's normalized
   Hunter snapshot; do not copy poisoned field order or IL2CPP indices into the
   web protocol.
2. Recover the native `Il2CppType` table through metadata registration before
   asserting concrete original field types or collection shapes.
3. If binary compatibility is still needed, capture one controlled local save
   from a test device before/after changing exactly one Hunter field and diff
   the files. A low-resource physical device or user-provided app-data export
   is preferable to starting an emulator now.
4. Only after a save sample exists, identify whether ACTk file headers,
   PlayerPrefs XML, JSON or MessagePack signatures are actually used.
5. Treat cloud sync and original backend formats as a separate boundary; the
   APK alone does not prove their schema.

No emulator is warranted for the current static question, and none was run.
