# Original Hunter ConstantData Coefficients v1

This evidence-only pass classifies six exact `HunterCtrl.getDamage` callers whose
native bodies read named `ConstantData` fields. It does not connect any formula
to live Rust combat and does not assign public skill rows to obfuscated methods.

The deterministic analyzer is
`tools/analyze-original-native-hunter-constant-coefficients-pass16.py`; generated
evidence is
`reverse-engineering/evidence/original-native-hunter-constant-coefficients-pass16.json`.

## Confirmed boundaries

| Native caller | Exact ConstantData participation | Final arithmetic boundary | Route |
| --- | --- | --- | --- |
| `BPOGPFGALFD(EvilCtrl)` | `POISON_AURA_POWER_VALUE` (`ObscuredInt`) | Adds two decoded modifiers before multiplying `baseDamage * power * 0.01f`, then truncates. A target-side discriminator value `2` enables a later multiplier and second truncation. | `EvilCtrl` virtual slot `+0x2A8`, damage parameter 2, parameter 5 = `2` |
| `PCGIDDENIJL(EvilCtrl)` | `CURSEAURA_POWER_VALUE` (`ObscuredFloat`); `DARK_KNIGHT_DAMAGE_TYPE_INDEX` | First truncates `baseDamage * power * 0.01f`, then multiplies by the sum of two modifiers and truncates again. The same target-side discriminator branch can apply a later multiplier and third truncation. | `EvilCtrl` virtual slot `+0x2A8`; parameter 5 is the decoded damage-type index |
| `NMAIFPMMBHE(Single,Single,Single)` | `GEAR_PROP_25_VALUE`, `GEAR_FROZEN_HEART_PROPERTY_INDEX`, `FROZEN_HEART_SPIN_SPLASH_SKILL_UP_VALUE` | Final damage is `trunc(float32(baseDamageAfterOptionalIntegerScale) * parameter1 * coefficient * 0.01f)`. A gated Frozen Heart branch adds the named Spin Splash value. | `BlizzardCtrl.Action`, damage parameter 6 |
| `DNPJKKJPHLD(EvilCtrl,Single)` | `GEAR_PROP_25_VALUE`, Frozen Heart property index and Shadow Strike value, Shadow Skin index, damage-type index | Final damage is `trunc(float32(baseDamageAfterOptionalIntegerScale) * parameter2 * coefficient * 0.01f)`. Frozen Heart adds the named Shadow Strike value; a Shadow Skin indexed helper contributes another still-unnamed float. | `EvilCtrl` virtual slot `+0x2A8`, damage parameter 2 |
| `NPIAALIFANE(Single,Single)` | `GEAR_PROP_25_VALUE`, `FROST_ARCHER_SNIPING_SKILL_UP_VALUE` | A gated branch adds the Sniping value to a dynamic coefficient. The final factor is `parameter1 * (1 + coefficient)` and damage is truncated after multiplying by `baseDamage * 0.01f`. | Native target `0x32ff6cc`; damage is in ARM64 argument register `x4`, but the managed parameter identity is unresolved |
| `EHKBOGAOFEN(EvilCtrl)` | Dark Lightning base power plus regular/truthful Thunder Dragon Fury property and power pairs | Native integer arithmetic builds `basePower + selectedPower + selectedPower * selectedPropertyValue`; it multiplies this by `baseDamage` before float32 conversion, applies `0.01f`, then truncates. | `FlameExplosionCtrl.Action`, damage parameter 4, selector parameter 6 = `2` |

The `0.01f` value is the independently captured package literal at module offset
`0xD2AC8C` (`0ad7233c`, float32 `0.009999999776482582`). All final conversions
listed above use ARM64 `FCVTZS`, which truncates toward zero. `PCGIDDENIJL` has an
additional earlier `FCVTZS`; preserving that intermediate truncation is required.

## Important non-equivalences

- The Poison Aura and Curse Aura bodies are not interchangeable: their
  modifier order and intermediate truncation differ.
- `GEAR_PROP_25_VALUE` is not safely classifiable as a simple percent. In the
  Frozen Heart callers, a gated branch multiplies the decoded damage snapshot
  by this integer before the later float32 coefficient chain.
- The regular and Truthful Thunder Dragon Fury fields are selected through
  native inventory/property branches. Their product-facing precedence is not
  inferred beyond that exact control flow.

## Remaining blockers

- Resolve managed target `0x32ff6cc` and the `EvilCtrl` virtual slot `+0x2A8`.
- Resolve the semantic identities of the target-side modifiers/discriminator
  and the Shadow Skin helper-fed float.
- Recover public skill-row mappings and callers for the obfuscated methods.
- Do not reduce the remaining fully-resolved caller count by all six: this pass
  classifies exact ConstantData bindings and final arithmetic, while several
  gate/helper meanings remain open.
