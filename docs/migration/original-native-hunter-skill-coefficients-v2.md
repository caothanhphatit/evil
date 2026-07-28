# Original Native Hunter Skill Coefficients v2

## Scope

Pass13 reduces the next largest identical coefficient cluster among the exact
`HunterCtrl.getDamage` callers. It does not connect formulas to live combat or
map obfuscated methods to public skill rows.

```sh
python3 tools/analyze-original-native-hunter-skill-coefficients-pass13.py
python3 -m unittest tools.tests.test_original_native_hunter_skill_coefficients_pass13
```

## Internal `ObscuredInt` percentage family

Six exact methods share this arithmetic boundary:

```text
percent = float32(decode(sourceObscuredInt)) * 0.01f
percent = decode(ObscuredFloat(percent))
damage = trunc_i64(float32(baseDamage) * percent)
```

All multiplications are float32. The final ARM64 `FCVTZS` truncates toward
zero. The intermediate ACTk wrap/decode is preserved as an observed operation,
not optimized away in the evidence contract.

| Hunter method | Exact coefficient source | Route | Route selector |
| --- | --- | --- | --- |
| `KMFIIOFLHKC(EvilCtrl)` | `ConstantData.BLOW_DESTRUCTION_POWER_VALUE` | `FlameExplosionCtrl.Action`, damage argument 4 | parameter 6 = `0` |
| `BBOCACECAAO(EvilCtrl)` | `ConstantData.VENUM_RAIN_POWER_VALUE` | `FlameExplosionCtrl.Action`, damage argument 4 | parameter 6 = `1` |
| `CHKMAHCLJBN(EvilCtrl)` | `ConstantData.CURSE_CHAIN_POWER_VALUE` | `FlameExplosionCtrl.Action`, damage argument 4 | parameter 6 = `2` |
| `BCLCCDFCHFC(EvilCtrl)` | `ConstantData.DARK_RIFT_POWER_VALUE` | `FlameExplosionCtrl.Action`, damage argument 4 | parameter 6 = `0` |
| `GHFOIEIIDDF(EvilCtrl)` | nested `ObscuredInt` collection lookup | `FlameExplosionCtrl.Action`, damage argument 4 | parameter 6 = `0` |
| `IABOOKJBHHO(EvilCtrl)` | `ConstantData.POISON_FANG_POWER_VALUE` | target `EvilCtrl` vtable slot `+0x2A8`, damage argument 2 | parameters 4/6 = `false,false`; parameter 5 is runtime-derived |

The ConstantData names and offsets are runtime-schema bindings. They identify
coefficient sources, but they do not prove a public QuickSheet skill-row mapping
for the obfuscated Hunter method.

## Routing boundary

Five members route to the independently resolved
`FlameExplosionCtrl.Action(String,Int32,String,Int64,Int32,Int32)`. The final
member dispatches through the target `EvilCtrl` vtable slot `+0x2A8`; its
managed identity remains unclaimed because matching the six-argument shape is
not sufficient identity proof.

## Coverage

Pass9 and Pass11 previously resolved nine coefficient callers. Pass13 resolves
six more, leaving 34 of the 49 exact `getDamage` callers for later reduction.

Still fail-closed:

- public skill-row mappings for these methods;
- the collection/index identity in `GHFOIEIIDDF`;
- managed identity of the `EvilCtrl` virtual target;
- the runtime meaning of `IABOOKJBHHO` parameter 5;
- coefficient producers for the remaining 34 callers.

No runtime server formula changes are made by this pass.
