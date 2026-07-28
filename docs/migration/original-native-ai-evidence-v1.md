# Original Native AI Evidence v1

## Scope and capture boundary

This report records only behavior visible in bounded, live-decrypted ARM64 method bodies from the Android API 35 emulator. It does not assign meanings to obfuscated fields, helpers, FSM integers, job values, RNG branches, or damage formulas without an independent source.

The live `libil2cpp.so` mapping base was `0x73a7e00000`. Every module offset below is added directly to that mapping base; no APK file offset or ELF load-bias correction is applied.

Method pointer index was derived as `(token & 0x00FFFFFF) - 1`. A body ends at the nearest higher unique Assembly-CSharp method pointer. Aliases at the same pointer share one body and do not create a boundary. The same procedure reproduces the independently known 68-byte `RandDamage` body.

Raw decrypted method binaries remain temporary under `/tmp/evil-ai-native-20260726/` and are not committed. The committed evidence retains bounded size, SHA-256, normalized disassembly, direct-call summaries, and schema correlations.

## Reproduction commands

```sh
python3 tools/analyze-il2cpp-native-ai.py \
  --capture /tmp/evil-ai-native-methods-live.json \
  --binary-dir /tmp/evil-ai-native-20260726 \
  --disassembly-dir /tmp/evil-ai-native-20260726/disasm \
  --schema reverse-engineering/evidence/evil-ai-drop-runtime-schema-android-api35-v1.json \
  --output reverse-engineering/evidence/original-native-ai-runtime-v1.json \
  --disassembly-output reverse-engineering/evidence/original-native-ai-disassembly-v1.txt

python3 -m py_compile \
  tools/analyze-il2cpp-native-ai.py \
  tools/runtime/capture-il2cpp-native-methods.py

jq empty reverse-engineering/evidence/original-native-ai-runtime-v1.json
git diff --check
```

## Exact native boundaries

| Method | Token | Module offset | Bytes | SHA-256 |
| --- | --- | ---: | ---: | --- |
| `EvilCtrl.Move` | `0x06002FB5` | `0x2f156d8` | 356 | `e3ba9c46084c3a11677a76e7c5c6fa0e0d781abc9b2f644cdb034740af1fcbf2` |
| `EvilCtrl.FsmMoveEnd` | `0x06003003` | `0x2f2a334` | 612 | `48646b1b91cf56ac5c56faaf62f36126331726f93aa67f7504a0456a4cb4317d` |
| `EvilCtrl.Dead` | `0x06002FE7` | `0x2f233e8` | 1,484 | `7e3d9c552f1ecea2e14d00fa1f9bfc9a1ba2f447ad29505ea3b3cb570ec7552e` |
| `EvilCtrl.FixedUpdate` | `0x06002FFE` | `0x2f290fc` | 2,936 | `fdfcf59b9ff3a4a50f8a4414cc6b01734bb10242581735827794e4af18b814a4` |
| `EvilCtrl.UnitAttack` | `0x0600300D` | `0x2f156ac` | 44 | `ec9f2a92603308602a0d1012f0191e65122fdd91f96b464925bcd86ac1519571` |
| `HunterCtrl.HuntingSecond` | `0x06005B5F` | `0x3408264` | 1,180 | `e43b373b42762e3ab0a7fb6bad8791075169e786c29fbb2ca06bb5eed2106171` |
| `HunterCtrl.HuntingFirst` | `0x06005B84` | `0x341aeb0` | 1,180 | `d142aa7b2d86e18fc7a6307289abf959f626adfd0b918ba94d95b70a71ea4083` |
| `HunterCtrl.HuntingAttackSetting` | `0x06005BE4` | `0x34012f8` | 660 | `6119134928144d1fcc8dd3a24c5d119274bf3fc3010784bd3fb36cc13d63deef` |
| `HunterCtrl.HuntingAttackAction` | `0x06005C28` | `0x3416a40` | 8,016 | `ba7fd4e42daefba169ded822a4aaef70cee0371609f8eb2804f3b8cecf65bf59` |
| `HunterCtrl.Hunting` | `0x06005C29` | `0x34548a4` | 416 | `12ff4cc111ec7d5cd9e1c7b87d542cb14af5f3be2156fd42d889a748ad0edd92` |

## Evidence-only state machine

| Entry | Confirmed transition or mutation | Unresolved boundary |
| --- | --- | --- |
| `EvilCtrl.UnitAttack` | Reads offset `572`; sets Boolean offset `133`; computes/stores float offset `128`, including an offset-572 multiplier when the decoded value exceeds `1`. | Meanings of `EPIMNKLDPCC`, `AFNMKJOIPPD`, and `EPFLFJHGLKH`. |
| `EvilCtrl.FixedUpdate` | Decrements timers at offsets `124` and `584`; calls `CheckEvilData`; validates `mTargetUnit` at `160`; compares squared planar distance with offset `128`; outside range updates movement and calls `LAMNNAOEOIE`; invalid target calls `UnitAttackEnd`; an in-range/timer-expired path calls `EBNOJHOGEMM`. | Boolean gates at `132-135`, `344`, and `460-463`; GameManager enum values; Unity helper semantics. |
| `EvilCtrl.Move` | Calls `PANFOJGCGDE`; references vector offset `100` and object offset `312`. | Movement/path helper semantics and exact use of the two fields. |
| `EvilCtrl.FsmMoveEnd` | Calls `CheckEvilData`; selects a new planar point through an unresolved Unity/helper boundary; stores it at `AIJLHNOBNHD` (`100`); compares it exactly with the previous `DHHLBIJJCJN` (`108`); when the points match it dispatches the next virtual FSM boundary, while another branch calls `PINBKNPMMHA` when the queued-list count exceeds `AHHGGNMACOG` (`96`). | Meaning of the two EvilData-dependent map branches, point-selection helper, queued element type, and whether any native pause is scheduled outside this method. |
| `EvilCtrl.Dead` | Calls `PNCPAGGMBML(bool)`; one branch can read/update rift gauge; removes `mStringIndex` (`144`) keyed entries; stops coroutine `HPINIGEBEJM` (`624`); one cleanup path calls `ENFKGEBNIEE(false)`. | Boolean meaning, container owners, reward eligibility, and destruction helper semantics. |
| `HunterCtrl.Hunting` | Clears the FSM queue; unresolved selector called with `(0, 3)` returns only `0`, `1`, or `2`; inserts branch-specific values and shared suffix values. | FSM integer labels and selector semantics. |
| `HunterCtrl.HuntingFirst(bool)` | Closes speech and validates Hunter state; optional prefix inserts static values at `+0x0c`, `+0x24`; always inserts `+0x30`, `+0xd0`, then `+0xd8` with flag `1`; failure can call `OMEEOBPNOGH(bool)`. | Exact preliminary condition, Boolean argument meaning, and FSM labels. |
| `HunterCtrl.HuntingSecond(bool)` | Same boundary as First, with optional `+0x10`, `+0x28`; always `+0x34`, `+0xd0`, then `+0xd8` with flag `1`. | Exact preliminary condition, Boolean argument meaning, and FSM labels. |
| `HunterCtrl.HuntingAttackSetting` | Reads `mTargetEvil` (`888`); reads obscured HunterData values at `0x20` and `0x40`; writes obscured `mRange` (`436`); uses live constants approximately `0.08`, `0.20`, or immediate `1.5`; decoded range over `1` is multiplied by target offset `572`; sets `mAttackCheck` (`500`) true. | Job enum/value interpretation and exact range policy labels. |
| `HunterCtrl.HuntingAttackAction` | Mutates HunterData `nowHungry` (`452`); reads `mTargetEvil` (`888`); calls `getDamage` and multiple skill/trait handlers; sets Boolean `976`; writes `AttackAniTime` (`428`) and `mNowAnimation` (`496`); increments `TargetAttackCount` (`896`). | Damage/critical/dodge formulas, RNG boundaries, exact handler ordering, job/skill/status enum meanings. |

`HunterCtrl.Hunting` queue construction is exact at the static-value boundary:

| Selector result | Branch prefix | Shared suffix |
| ---: | --- | --- |
| `0` | `+0x0c`, `+0x24`, `+0x30` | `+0xd0`, `+0xd8` with flag `1` |
| `1` | `+0x10`, `+0x28`, `+0x34` | `+0xd0`, `+0xd8` with flag `1` |
| `2` | `+0x14`, `+0x2c`, `+0x38` | `+0xd0`, `+0xd8` with flag `1` |

These offsets identify the loaded static values, not their enum names.

`EvilCtrl.FsmMoveEnd` strengthens the patrol implementation boundary: original
movement is waypoint/FSM driven and has an explicit movement-end transition. It
does not recover a roam radius or a five-second idle timer. Any such values in
the rebuild remain named product tuning rather than original-game evidence.

## Committed evidence

- `reverse-engineering/evidence/original-native-ai-runtime-v1.json`: mechanical size/hash/call/field record; no raw body bytes.
- `reverse-engineering/evidence/original-native-ai-disassembly-v1.txt`: normalized annotated ARM64 disassembly with live addresses rewritten as `libil2cpp+0x...` where applicable.
- `reverse-engineering/evidence/evil-ai-drop-runtime-schema-android-api35-v1.json`: field offset/type correlation source.
