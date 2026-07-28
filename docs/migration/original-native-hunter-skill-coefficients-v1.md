# Original Native Hunter Skill Coefficients v1

## Scope

This Pass11 report clusters exact native coefficient formulas among the 49
captured callers of `HunterCtrl.getDamage`. It binds only managed method/token,
native operand order, coefficient parameter position, action target, and the
action parameter receiving damage. Obfuscated methods are not assigned public
skill names.

```sh
python3 tools/analyze-original-native-hunter-skill-coefficients-pass11.py
python3 -m unittest tools.tests.test_original_native_hunter_skill_coefficients_pass11
```

## Plain `Single` percentage

Four exact bodies use:

```text
trunc_i64(baseDamage * coefficientPercent * 0.01)
```

| Hunter method | Coefficient input | Action target | Damage argument |
| --- | --- | --- | ---: |
| `PIKOCNCIHNO(EvilCtrl,Single,Single)` | first `Single` | `FlameExplosionCtrl.Action` | 4 |
| `HIDAPNPHFCA(Int32,Single,Single,Single)` | second `Single` | `BlizzardCtrl.Action` | 6 |
| `PMFEHNBKEIL(EvilCtrl,Single,Int32)` | only `Single` | `AreaCheckDistanceEffectCtrl.Action` | 3 |
| `BOJFAPCOBCE(EvilCtrl,Single)` | only `Single` | `AreaCheckDistanceEffectCtrl.Action` | 3 |

The trailing `Int32` in `PMFEHNBKEIL` selects one of captured internal action
resource branches. Its product enum meaning is unresolved.

## `ObscuredFloat` percentage

Two exact bodies decode an `ObscuredFloat` coefficient before applying:

```text
trunc_i64(baseDamage * decode(coefficientPercent) * 0.01)
```

| Hunter method | Coefficient input | Action boundary | Selector tail |
| --- | --- | --- | --- |
| `CHOGGFICJPL(ObscuredFloat)` | first argument | `FlameExplosionCtrl.Action`, damage argument 4 | action parameter 6 = `1` |
| `OMIEHJOENAE(EvilCtrl,ObscuredFloat)` | second argument | target `EvilCtrl` vtable slot `+0x2A8`, damage argument 2 | parameters 5/6 = `4,false` |

The virtual target has the argument shape of the recovered damage boundary, but
the managed identity is deliberately not asserted from shape alone.

## Affine `ObscuredFloat` percentage

Two bodies share this structural family:

```text
trunc_i64(
  baseDamage
  * (basePercent + decode(coefficientPercent) * internalMultiplier)
  * 0.01
)
```

| Hunter method | Coefficient input | Action boundary | Selector tail |
| --- | --- | --- | --- |
| `JHAAACFJNPA(EvilCtrl,ObscuredFloat)` | second argument | `FlameExplosionCtrl.Action`, damage argument 4 | action parameter 6 = `0` |
| `MLLCFGJDLDA(EvilCtrl,ObscuredFloat)` | second argument | target `EvilCtrl` vtable slot `+0x2A8`, damage argument 2 | parameters 5/6 = `6,false` |

The arithmetic family is exact. `basePercent` and `internalMultiplier` remain
native operand boundaries without product-facing names.

## Action targets

Runtime method-offset resolution independently confirms:

- `0x2C572B0` is
  `FlameExplosionCtrl.Action(String,Int32,String,Int64,Int32,Int32)`.
- `0x3024E00` is
  `AreaCheckDistanceEffectCtrl.Action(String,Int32,Int64,Int32,Int32,String,Int32,Int32)`.
- `0x33E9D84` is
  `BlizzardCtrl.Action(Int32,String,Int32,Single,Single,Int64,Int32)`.

## Coverage boundary

Pass11 resolves eight coefficient callers. Pass9 separately resolves the
modified Blizzard builder `GDBMICDJBOK`; forty captured `getDamage` caller
bodies still require coefficient-producer reduction.

Still fail-closed:

- public skill-row names for all obfuscated methods;
- managed identity of the `EvilCtrl` virtual slot `+0x2A8`;
- selector enum meanings for the action controllers;
- coefficient producers for the remaining forty callers;
- semantic names of affine-family internal modifiers.

No formula in this report is connected to live combat.
