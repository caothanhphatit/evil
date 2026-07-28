# Original Native Hunter Attack-Speed Producer Chain V2

## Scope

Pass 5 traces the API 35 ARM64 producer and consumer paths behind the previous
cadence equation. It covers `StatusData.CalcAttackSpeed`, the two
`HunterCtrl` cadence factors, and the direct `mAttackDelay` timer reader. The
machine-readable evidence is
`reverse-engineering/evidence/original-native-hunter-attack-speed-producer-chain-v2.json`.

All mappings below are from exact live-decrypted method bodies and runtime
schemas for Evil Hunter Tycoon `1.411`. This pass remains evidence-only; it does
not connect the recovered arithmetic to the live Rust simulation.

## `StatusData.CalcAttackSpeed` producer

`StatusData.COJNMPDBOOO()` (`0x06001D19`, module offset `0x2d5e1f8`, 880 bytes)
is the exact producer. It first computes:

```text
AttackSpeed = WeaponSpeed * (1 + 0.01 * (
    PersonalAttackSpeed
  + OptionAttackSpeed
  + RankAttackSpeed
  - UserData.mGuildAttackSpeedUp
  - GUP_Property[7]
  - RidingPetAttackSpeedUp
))
```

The method then selects the denominator used for `CalcAttackSpeed`:

```text
if FuryValue > 1.0:
    denominator = Quicken + FuryValue + SpeedPotionValue
else:
    denominator = Quicken + SpeedPotionValue + PersonalAttackSpeed

CalcAttackSpeed = max(0.25, AttackSpeed / denominator)
```

The resulting ACTk `ObscuredFloat` is written to
`StatusData.<CalcAttackSpeed>k__BackingField` at offset `0x88`. The
`UserData` chain is resolved through `GameManager.getInstance()` at module
offset `0x26c0238`, then `GameManager.mUserData` at offset `0x608` and
`UserData.<mGuildAttackSpeedUp>k__BackingField` at offset `0xbd8`.

`GUP_Property` is an `ObscuredFloat[]`; the exact native element address is
`array + 0xac`, which is element index 7 under the managed-array data layout.

## Fury and `BCEBGLKCDHN`

`StatusData.FGCEFJCHNCK(float)` (`0x06001CD9`, `0x2d5e1ac`, 76 bytes) is the
small writer for the Fury status:

```text
FuryValue = input * 0.01
then call COJNMPDBOOO()
```

`HunterCtrl.BuffSetting(int type, int value, bool, int)` has an exact type-zero
branch (`type == 0`) that performs both writes:

```text
FuryValue = value * 0.01
BCEBGLKCDHN = value * 0.01
```

`HunterCtrl.BuffEndSetting(int)` has a matching `type == 0` branch. It clears
Fury through `FGCEFJCHNCK(0)`, recomputes status, then resets
`BCEBGLKCDHN` to `1.0`.

Other exact writers found by scanning all 391 `HunterCtrl` method bodies are:

- `Init(ObscuredInt, ObscuredString, Boolean)`: writes `1.0`.
- `Init(Int32, Boolean)`: writes `1.0`.
- `CKKBPHNBKLC(...)`: writes the runtime float constant `535.0`.

No other direct managed writer for `BCEBGLKCDHN` was found. The 535.0 path is
recorded exactly, but its product-facing meaning is unresolved.

## `mAttackDelay` FSM reader

`HunterCtrl.HuntingAttackAction()`, `CGAHEABLJMF()`, and `NBOMDKMCGND(int)`
copy the complete 20-byte `StatusData.CalcAttackSpeed` representation into
`HunterCtrl.mAttackDelay` at offset `0x194`.

The only direct managed reader in the class-wide scan is
`HunterCtrl.FixedUpdate()` (`0x06005B75`, `0x340fcf8`, 6,800 bytes). Its exact
timer behavior is:

```text
decoded = decode(mAttackDelay)
if decoded > 0:
    decoded = decoded - UnityEngine.Time.deltaTime
    mAttackDelay = encode(decoded)
else:
    mAttackDelay = encode(0)
```

This proves the client-side countdown gate that can be used by the attack FSM.
No separate direct managed field reader was found in the 391-method scan.

## `DANCPPLMKIK` boundary

`HunterCtrl.DANCPPLMKIK` at offset `0x3d8` is read by
`NICAFPDFNPG()`/`CGAHEABLJMF()`/`NBOMDKMCGND(int)` and by
`HuntingAttackAction()`. The exact class-wide scan found zero direct managed
writers across all 391 `HunterCtrl` methods, including `.ctor`.

The field therefore remains unresolved at the authority boundary: it may be
populated by Unity serialization/default injection or an indirect native path,
but the opaque `HunterCtrl` prefab payload has not been decoded sufficiently to
claim that mapping. It must remain fail-closed in the rebuild.

## Weapon relationship

This pass does not change the separate weapon-presentation evidence. Packaged
weapons remain Spine skin attachments with directional attack clips; exact
gear-index-to-skin selection and facing rules remain unresolved and must not be
inferred from the attack-speed factors.

## Reproduction

```sh
python3 tools/analyze-il2cpp-native-hunter-attack-speed-pass5.py
python3 -m unittest tools.tests.test_analyze_il2cpp_native_hunter_attack_speed_pass5
```
