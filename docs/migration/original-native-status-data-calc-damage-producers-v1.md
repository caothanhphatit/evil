# Original StatusData CalcDamage Producers v1

## Scope

This pass resolves the two base multipliers consumed by
`StatusData.EBNGMMPBEDA()` while leaving the live rebuild combat path unchanged.
The evidence is from Evil Hunter Tycoon `1.411` ARM64/API35 and does not contain
player save values.

## CalcLevel

`StatusData.OLHJKKDDMHM()` reads `HunterData.level` from the confirmed
`ObscuredInt` field at offset `0x88`. Its native float32 arithmetic is:

```text
CalcLevel = float32(1.0 + float32(HunterData.level) * float32(0.003))
```

The `0.003` operand is the package literal at `libil2cpp+0xD2A96C`, captured as
raw little-endian bytes `a69b443b`.

## CalcRevive

The `StatusData` constructor initializes `CalcRevive` to `1`.
`StatusData.MDCNDJHNOAE()` reads the confirmed `HunterData.revive`
`ObscuredInt` field at offset `0xC4`. It returns without writing when the decoded
value is below `1`; otherwise it stores three times the decoded value:

```text
CalcRevive = HunterData.revive < 1
  ? 1
  : wrapping_i32(HunterData.revive * 3)
```

The wrapping notation records the native 32-bit `add` instruction exactly. It
is not a product claim that valid revive values can approach overflow.

## Evidence and reproduction

- `reverse-engineering/evidence/original-native-status-data-level-revive-producers-api35-v1.json`
- `reverse-engineering/evidence/original-runtime-status-data-static-factors-api35-v1.json`
- `reverse-engineering/evidence/original-native-status-data-level-revive-analysis-v1.json`
- `reverse-engineering/evidence/original-native-status-data-torment-static-owner-api35-v1.json`
- `reverse-engineering/evidence/guild-manager-runtime-schema-api35-v1.json`

The wider producer scan also corrects an earlier provisional name: the branch
IDs `78/418/599/600`, `360`, and `748/773` read `HunterData.fairyIndex`, not
`StatusData.PolyIndex`. They set `FairyAttackUp` to `0.02`, `0.04`, and `0.06`
respectively; unmatched fairy IDs store zero. A later, separate
`StatusData.PolyIndex == 49` branch multiplies damage by the package double
`1.2999999523162842`.

The previously unnamed singleton operand in the torment layer is now resolved
by its exact native target and class schema:

```text
damage *= 1 + UserData.mTormentAttackUp + GuildManager.mRankBuffAttack
```

Reproduce the deterministic analysis:

```sh
python3 tools/analyze-original-native-status-data-calc-damage.py \
  --methods reverse-engineering/evidence/original-native-status-data-level-revive-producers-api35-v1.json \
  --static-factors reverse-engineering/evidence/original-runtime-status-data-static-factors-api35-v1.json \
  --calc-damage-producer reverse-engineering/evidence/original-native-status-data-calc-damage-producer-api35-v1.json \
  --guild-owner-method reverse-engineering/evidence/original-native-status-data-torment-static-owner-api35-v1.json \
  --guild-schema reverse-engineering/evidence/guild-manager-runtime-schema-api35-v1.json \
  --output reverse-engineering/evidence/original-native-status-data-level-revive-analysis-v1.json
```

## Integration boundary

These producers are mechanically complete, but they are only two operands of
the wider `CalcDamage` chain. Live combat remains on the explicitly labeled
rebuild fixture until the remaining static operand, PolyIndex control flow,
monster armor/minimum-damage consumer, and caller ordering are all resolved and
covered by golden vectors.
