# Original Hunter Literal Multipliers v1

This evidence-only pass classifies three exact `HunterCtrl.getDamage` callers
that wrap an inline float32 multiplier as `ObscuredFloat`, decode it, multiply
it by the decoded base damage, and truncate with ARM64 `FCVTZS`.

The deterministic analyzer is
`tools/analyze-original-native-hunter-literal-multipliers-pass17.py`; generated
evidence is
`reverse-engineering/evidence/original-native-hunter-literal-multipliers-pass17.json`.

## Exact arithmetic and routes

| Native caller | Vector | Arithmetic | Route |
| --- | --- | --- | --- |
| `MEDDIMPJHDA(EvilCtrl)` | `(true,false,true)` | `trunc(float32(baseDamage) * 286.0f)` | `FlameExplosionCtrl.Action`, damage parameter 4, selector parameter 6 = `1` |
| `BOKBBDIDLJG(EvilCtrl)` | `(true,false,true)` | `trunc(float32(baseDamage) * 5.0f)` | `FlameExplosionCtrl.Action`, damage parameter 4, selector parameter 6 = `0` |
| `KKHDNNMAOKA(EvilCtrl)` | `(true,false,true)` | `trunc(float32(baseDamage) * 3.0f)` | Native target `0x2b2b734`, damage register `x3`; managed identity unresolved |

The first two callers load the exact `ConstantData.FLAMEEXPLOSION_OBJ_NAME`
string for the action. The third loads `ConstantData.DIVINEATTACK_OBJ_NAME`, but
the object-name evidence is not treated as proof of the target method's managed
class or public skill mapping.

## Result discriminator payload

Each body decodes the `HunterCtrl.PMPKOIHFNCE` result member at object offset
`+0x10` as the base damage. It separately decodes the member at `+0x30` and
forwards that value unchanged:

- `MEDDIMPJHDA` and `BOKBBDIDLJG`: `+0x30` becomes
  `FlameExplosionCtrl.Action` parameter 5.
- `KKHDNNMAOKA`: `+0x30` is passed in ARM64 `w4` to native target
  `0x2b2b734`.

This is an exact payload boundary, not an enum binding. No captured field name
or branch proves whether its values mean normal damage, critical damage, or a
different presentation category, so the evidence deliberately leaves that
semantic unresolved.

## Coverage policy

This pass classifies arithmetic for three callers and fully closes zero callers.
Public skill mappings remain unresolved, the `+0x30` result discriminator lacks
a proven enum meaning, and `0x2b2b734` still needs managed target resolution.
The formulas remain disconnected from live combat.
