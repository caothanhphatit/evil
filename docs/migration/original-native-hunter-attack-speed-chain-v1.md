# Original Native Hunter Attack-Speed Chain V1

## Scope

This pass follows the original Evil Hunter Tycoon `1.411` ARM64 native code from
`HunterCtrl.HuntingAttackAction()` into the fields that control attack
presentation timing. It also resolves the previously unknown native target at
`libil2cpp+0x33f79bc` and checks two nearby candidate writer methods.

This is evidence only. It does not change the Rust simulation or web renderer,
and it does not assign gameplay meanings to obfuscated fields without a proven
writer chain.

Machine-readable output:

- `reverse-engineering/evidence/original-native-hunter-attack-speed-chain-v1.json`

Deterministic analyzer:

- `tools/analyze-il2cpp-native-hunter-attack-speed.py`

## Exact boundary resolution

The exact `HunterCtrl.getStatusData()` body starts at `0x33f7900`, has 188
bytes, and ends at `0x33f79bc`. A pointer-only IL2CPP method-table scan resolves
that end address to:

| Property | Value |
| --- | --- |
| Class | `HunterCtrl` |
| Method | `InitHunterHpBar` |
| Token | `0x06005C11` |
| Parameters | none |
| Return | `System.Void` |

Therefore `0x33f79bc` is the next managed method boundary. It is not an
attack-speed helper and must not be named or ported as one.

## Confirmed cadence equation

The exact `HuntingAttackAction()` body performs this sequence:

```text
read HunterCtrl.DANCPPLMKIK at 0x3D8
read and decode HunterCtrl.BCEBGLKCDHN at 0x6AC
composite = DANCPPLMKIK * decode(BCEBGLKCDHN)
AttackAniTime = composite > 1.0 ? 0.333 / composite : 0.7
write HunterCtrl.AttackAniTime at 0x1AC
```

The analyzer pins the ARM64 instructions at method-relative offsets
`0x1A10..0x1A70`, the exact body boundary, and the runtime schemas for all three
fields. This confirms the arithmetic and destination but not the gameplay
meaning or writers of the two inputs.

## Confirmed `mAttackDelay` source

At the tail of `HuntingAttackAction()`, the native body calls
`HunterCtrl.getStatusData()` at `0x33f7900`. If the returned object is non-null,
it copies the complete 20-byte ACTk `ObscuredFloat` representation:

```text
StatusData.<CalcAttackSpeed>k__BackingField at 0x88
    -> HunterCtrl.mAttackDelay at 0x194
```

The copy is not a decoded float assignment. Native code loads 16 bytes from
`StatusData+0x88` and the final 4 bytes from `StatusData+0x98`, then stores them
at `HunterCtrl+0x194` and `HunterCtrl+0x1A4`. This mechanically proves:

```text
HunterCtrl.mAttackDelay = raw copy of StatusData.CalcAttackSpeed
```

It does not yet prove where `StatusData.CalcAttackSpeed` is calculated or which
attack-FSM reader consumes `mAttackDelay`.

## Writer checks

`HunterCtrl.SettingProperty()` does not write `BCEBGLKCDHN` at `0x6AC`. Its
captured exact body writes the neighboring ACTk field `PGDMKPKELMM` beginning
at `0x698`, including the final word at `0x6A8`.

`HunterCtrl.RefreshAnimation()` does not reference any of these requested
offsets in its captured exact body:

- `mAttackDelay` at `0x194`
- `AttackAniTime` at `0x1AC`
- `DANCPPLMKIK` at `0x3D8`
- `BCEBGLKCDHN` at `0x6AC`

These are scoped negative findings. They do not prove that no other native
method writes the fields.

## Relationship to weapon presentation

The separate packaged presentation evidence already proves that Hunter weapons
are Spine skin attachments on the `sword`, `hammer`, `bow`, `wand`, `spear`,
and secondary weapon slots, and that basic attacks expose exact front/back
0.3333-second clip pairs. See
`docs/migration/original-hunter-weapon-attack-presentation-evidence-v1.md`.

This pass adds the native cadence bridge used during attack action. It does not
resolve the remaining gear-index-to-Spine-skin mapping, so the rebuild must not
invent which equipped gear row selects which attachment.

## Remaining blockers

- Writers and semantic sources for `DANCPPLMKIK` and `BCEBGLKCDHN`.
- The formula and aggregation chain that produces `StatusData.CalcAttackSpeed`.
- The exact attack-FSM reader/gate that consumes `HunterCtrl.mAttackDelay`.
- Exact gear-index-to-weapon-skin mapping and native target-axis facing rule.

Until those are recovered, the live rebuild may use clearly named temporary
tuning but must not claim original attack-speed or weapon-selection parity.

## Reproduction

```sh
python3 tools/analyze-il2cpp-native-hunter-attack-speed.py
python3 -m unittest tools.tests.test_analyze_il2cpp_native_hunter_attack_speed
```
