# Original Hunter Movement, Animation, and Revival Evidence v1

## Boundary

This report uses only the shipped XAPK/Unity bundles, Spine JSON, IL2CPP reflection metadata, and bounded live-decrypted native methods from the Android API 35 emulator. It does not assign meanings to obfuscated helpers, static FSM values, or animation-to-job branches without a direct source.

The live native captures use stable `libil2cpp.so` module offsets. ASLR addresses are omitted. Temporary captures are:

- `/tmp/evil-hunter-movement-revive-schema-20260727.json`
- `/tmp/evil-hunter-command-menu-schema-20260727.json`
- `/tmp/evil-hunter-movement-revive-native-20260727.json`
- `/tmp/evil-hunter-command-menu-native-20260727.json`
- `/tmp/evil-hunter-movement-ui-method-map-20260727.json`
- `/tmp/evil-hunter-selectctrl-method-map-20260727.json`

No save values or managed object instances were read.

## Hunter click to Movement submenu

The Vietnamese XAPK string table gives the user-visible sequence verbatim:

> `Chạm vào thợ săn và chọn 'Di Chuyển' > 'Thuộc Địa' để nhắc thợ săn…`

This proves the labels `Di Chuyển` and `Thuộc Địa`, and proves their parent/child menu order.

The native and schema boundary is:

| Boundary | Token | Module offset | Exact evidence |
| --- | --- | ---: | --- |
| `HunterClickCtrl.OnMouseUp()` | `0x06007AA6` | `0x28c4220` | Tail-branches at method `+0xbc` to `HunterClickCtrl.Select()`. |
| `HunterClickCtrl.Select()` | `0x06007AA0` | `0x28c3e50` | Tail-branches to either `HunterCtrl.SpeakTextShow(...)` or `HunterCtrl.selectUnit()`. |
| `HunterCtrl.selectUnit()` | `0x06005C7A` | `0x345e4a8` | Performs the selection/UI-manager path; the final manager helper that owns `HunterSelectCtrl.Show(...)` remains unnamed. |
| `HunterSelectCtrl.Show(HunterCtrl)` | `0x060055C2` | `0x3382624` | Stores the selected Hunter at field offset `32`; initially disables all four submenu groups. |
| `HunterSelectCtrl.HuntingGroupClick()` | `0x060055BD` | `0x33821d8` | Reads `mHuntingGroup` at offset `104`, activates it, and deactivates `mItemGroup` (`96`), `mSkillGroup` (`112`), and `mFoodGroup` (`120`). |
| `HunterSelectCtrl.HuntingGroupAction(int)` | `0x060055C3` | `0x3382790` | Dispatches accepted integer choices and then tail-calls `HuntingGroupClick()`. |

`HunterSelectCtrl` serialized/runtime fields independently identify the visible command groups: `InfoBtn`, `CancelBtn`, `ItemBtn`, `HuntingBtn`, `SkillBtn`, `FoodBtn`, followed by `mItemGroup`, `mHuntingGroup`, `mSkillGroup`, and `mFoodGroup` in that order.

`HuntingGroupAction(int)` has exact direct-call branches:

| Input | Direct native boundary |
| ---: | --- |
| `0` | `HunterCtrl.Tranning()` |
| `1` | `HunterCtrl.HuntingSecond(bool)` (`0x06005B5F`) after HunterData checks |
| `2` | `HunterCtrl.HuntingThird()` (`0x06005CC4`) |
| `3` | `HunterCtrl.ComebackHome()` (`0x06005CA4`) |
| `4` | `HunterCtrl.HuntingFirst(bool)` (`0x06005B84`) after HunterData checks |

The labels attached to these five integer choices are not serialized in the captured IL2CPP schema. Do not infer them only from call names.

## Three ordinary hunting regions

The `HuntingSelectPop` scene hierarchy and raw Text payloads establish this order:

| Runtime field | Scene object | Korean source Text | Vietnamese XAPK label | Serialized `AllMoveArea` argument |
| --- | --- | --- | --- | ---: |
| `mArea1Grid` offset `40` | `Area1` path ID `9642` | `침략의땅` from component `85426` | `Thuộc Địa` | `1` on `Button (1)` component `85483` |
| `mArea2Grid` offset `48` | `Area2` path ID `9647` | `망자의땅` from component `85445` | `Tử Địa` | `2` on `Button (2)` component `85458` |
| `mArea3Grid` offset `56` | `Area3` path ID `9661` | `파멸의땅` from component `85486` | `Ma Giới` | `3` on `Button (3)` component `85456` |

The same hierarchy contains `mVillGrid` at offset `64` and an unnumbered `Button` whose serialized `AllMoveArea` argument is `0`. This proves the complete serialized argument set `0, 1, 2, 3`; the field and object ordering structurally associates `0` with `Vill`, but no enum type name was recovered.

Relevant methods are `HuntingSelectPop.AllMoveArea(int)` token `0x06007B6A`, `SetHuntingArea()` token `0x06007B53`, and `PopupShow()` token `0x06007B4C`. `SetHuntingArea()` directly reaches `HuntingFirst`, `HuntingSecond`, and `HuntingThird` on separate ordinary-hunting branches, after checks including `IsDeadHunter`, `IsAdventureMember`, `IsUnderGroundMember`, and `IsRiftOrBossMode`.

## Animation inventory and autonomous hunting boundary

The shipped Spine actor JSON contains these exact names:

| Actor | Walk | Attack/action family | Receiving hit | Death |
| --- | --- | --- | --- | --- |
| Hunter | `hunter_walk`, `hunter_walk_back`, `hunter_walk_vehicle`, `hunter_walk_back_vehicle` | `h1_a_hit`, `h1_hit`, `h2_hit`, `h3_hit`, `h4_hit`, `h5_a_hit`, `h5_hit`, with `_back`, `_vehicle`, and named skill variants present | `hunter_damage`, `hunter_damage_back`, `hunter_damage_vehicle`, `hunter_damage_back_vehicle` | `hunter_die`, `hunter_dying`, `hunter_die_vehicle`, `hunter_dying_vehicle` |
| `mon_a_01_1` | `walk`, `walk_b` | `atk`, `atk_b` | no separately named hit/damage clip | `die`, `dying` |
| `mon_goldblin` | `walk`, `walk_b` | no named attack clip | no named hit/damage clip | `die`, `die2` |

The names prove asset availability, not the exact job-to-clip selection formula. In particular, the Hunter attack family is named `h*_hit` in the bundle while the distinct receiving-hit family is named `hunter_damage`.

The autonomous hunting native boundaries are:

| Method | Token | Module offset | Confirmed transition boundary |
| --- | --- | ---: | --- |
| `HunterCtrl.Hunting()` | `0x06005C29` | `0x34548a4` | Clears the FSM queue and builds one of three branch prefixes plus a shared suffix. |
| `HunterCtrl.HuntingFirst(bool)` | `0x06005B84` | `0x341aeb0` | Optional first-region prefix; always appends its fixed FSM suffix. |
| `HunterCtrl.HuntingSecond(bool)` | `0x06005B5F` | `0x3408264` | Optional second-region prefix; always appends its fixed FSM suffix. |
| `HunterCtrl.HuntingThird()` | `0x06005CC4` | schema token confirmed; body not included in the bounded AI disassembly set | Third branch exists and is called by command and selection paths. |
| `HunterCtrl.HuntingAttackSetting()` | `0x06005BE4` | `0x34012f8` | Sets range/target attack state and `mAttackCheck`. |
| `HunterCtrl.HuntingAttackAction()` | `0x06005C28` | `0x3416a40` | Calls damage/skill handlers and writes attack animation timing/state. |
| `HunterCtrl.HuntingAttackEnd()` | `0x06005B54` | schema token confirmed | Named attack-end FSM boundary. |
| `HunterCtrl.FsmInsertQueue(int,bool)` | `0x06005B9C` | `0x33ff9e8` | Queue insertion primitive. |
| `HunterCtrl.FsmClearQueue()` | `0x06005C0E` | `0x33f4b5c` | Queue reset primitive. |

The exact FSM integer labels and the exact animation string chosen by each attack branch remain unresolved.

## Revival and monster respawn

| Method | Token | Module offset | Bytes | Confirmed calls |
| --- | --- | ---: | ---: | --- |
| `EvilCtrl.Respawn()` | `0x06003008` | `0x2f2b31c` | 124 | Calls `FsmInsertQueue` at `+0x58`, then tail-calls it again at `+0x78`. |
| `HunterCtrl.PatternRevive()` | `0x06005B8C` | `0x341c86c` | 780 | Calls `SpeakClose`, `Revive(bool)` at `+0x24c`, `FsmClearQueue` at `+0x2dc`, then unresolved cleanup helpers. |
| `HunterCtrl.Revive(bool)` | `0x06005C8A` | `0x341cb78` | 1,796 | Calls `ExpHunterDelete`, `FsmClearQueue`, and multiple `FsmInsertQueue` branches; one branch calls `RaidOut`. |
| `HunterCtrl.ReviveReset()` | `0x06005C84` | `0x345edbc` | 2,372 | Repeatedly reads/writes HunterData and StatusData; exact reset-field semantics remain unresolved. |
| `HunterCtrl.OutReviveBuilding()` | `0x06005B4F` | `0x33ed9a8` | 628 | Reads HunterData, calls `KNNOOFPEENH`, then `FsmActionQueue` at `+0x24c`. |
| `HunterCtrl.ReviveCheckComeBackHome(int)` | `0x06005CC6` | `0x346b0b4` | 224 | Calls `SpeakTextShow` and tail-calls `ComebackHome` at `+0xcc`. |
| `HunterCtrl.ComebackHome()` | `0x06005CA4` | `0x3400b20` | 192 | Calls `ChangePatternIndex`, `SpeakClose`, `OMEEOBPNOGH`, then inserts two FSM entries. |

`ReviveBuildingCtrl.OnMouseUp()` token `0x060091D2` tail-branches to `ReviveBuildingCtrl.Select()` token `0x060091D3`. `BuildingReviveCheckPop` exposes `PopupShow(string,int)` token `0x0600213F` and `Confirm()` token `0x06002131`. These names establish the revival-building UI boundary; the exact building world coordinate or spawn-vector constant was not recovered and remains unresolved.

## Reproduction commands

```sh
~/.local/share/evil-frida-venv/bin/python \
  tools/runtime/capture-hunter-info-schema.py \
  --adb "$HOME/Library/Android/sdk/platform-tools/adb" \
  --target-type HunterClickCtrl \
  --target-type HunterSelectCtrl \
  --target-type HuntingSelectPop \
  --target-type HunterCtrl \
  --target-type EvilCtrl \
  --target-type ReviveBuildingCtrl \
  --output /tmp/evil-hunter-movement-revive-schema-20260727.json \
  --action "Read-only movement, hunting-region, and revival schema capture"

LC_ALL=C rg -a -o -N \
  ".{0,180}chọn 'Di Chuyển' > 'Thuộc Địa'.{0,180}" \
  game-assets/source/Evil+Hunter+Tycoon_1.411_APKPure.xapk

jq -r '.animations | keys[]' \
  apps/web/public/content/releases/visible-world-v1/actors/hunter/hunter.json
```
