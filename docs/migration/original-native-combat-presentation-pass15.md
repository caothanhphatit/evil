# Original Native Combat Presentation Pass 15

This pass binds the Android API 35 `1.411` combat text discriminator and its
primary pooled motion coroutine. It is presentation evidence only; it does not
add or infer a `CalcDodge` formula.

Normalized evidence:

`reverse-engineering/evidence/original-native-combat-presentation-pass15.json`

## Damage discriminator

`DamageManager.Show(Int32,Int64,Vector3,Int32,Boolean)` instantiates one
`DamageCtrl`, forwards every argument unchanged to `DamageCtrl.Show`, and
starts an independent coroutine for that instance. `DamageCtrl.Show` stores
the first integer at `Type@0x38` and dispatches exact values `0..17`.

The combat-facing bindings are:

| Type | Proven presentation | Rich text |
| --- | --- | --- |
| `0` | incoming damage shown by `HunterCtrl.Damaged` | `<color='#DE3232'>{damage}</color>` |
| `1` | normal Hunter outgoing damage | `<color='#AF70E0'>{damage}</color>` |
| `2` | critical Hunter outgoing damage | yellow 20px `CRIT`, newline, then the type-1 damage body |
| `3` | evade | `<color='#81F7F3'>Evade</color>` |
| `15` | invulnerable | `<color='#8c82fa'>Invulnerable</color>` |
| `16` | miss | `<color='#D43D3D'>Miss</color>` |
| `17` | percentage recovery | green wrapper around `+{damage/100:f2}% Recovery` |

The type-1/type-2 distinction is not inferred from color. Pass 12 proves the
baseline `getDamage` result discriminator is `1`, while its critical branch
calls `getCriticalDamage` and changes the discriminator to `2`. Only type `2`
prepends localized `status_6`, which resolves to `CRIT` in the English locale.

Incoming damage is independently bound to type `0`: the positive
`HunterCtrl.Damaged` path passes the computed damage to
`DamageManager.Show(type=0)`, while its non-positive clamp passes damage `1`
to the same call. Its invulnerability branch uses type `15` instead.

The captured effect-54 abort path calls
`DamageManager.Show(16,0,position,0,false)` and returns before
`HunterCtrl.Damaged`. Runtime localization resolves type `16` to `Miss`.
Type `3` separately resolves to `Evade`; this presentation distinction still
does not identify the missing global `CalcDodge` consumer.

Other exact type labels are retained in normalized evidence: type `4` EXP,
types `5..9` colored purchase/item-name variants, type `10` ELE, type `12`
Penalty, type `13` Lifesteal, and type `14` SOUL. Type `11` is a numeric
variant with color `#997C8A`; no public label is assigned.

## Prefab and position

The packaged `Damage Text` prefab has:

- default sample text `<size=20>치명타</size>\n1000`;
- `DefaultFont2`, shared-assets path ID `197`;
- font size `32`;
- rect size `50 x 20`;
- local scale initialized to `(1,1,1)` by `DamageCtrl.Show`.

World position is converted through the camera/canvas path. When the final
Boolean argument is true, native code adds exactly `60` to both the applied
canvas y and the stored `NowPos.y`. False retains the converted position.

## Primary text coroutine

The coroutine used by ordinary `DamageManager.Show` is nested iterator
`DamageManager.FJCKKCGAOJB.MoveNext`. It yields the manager's cached
`WaitForFixedUpdate@0x50` and moves `Rect.localPosition.y` relative to the
stored `NowPos.y`:

| Offset interval | Speed |
| --- | --- |
| `0 -> 5` | `20 units/s` |
| `5 -> 15` | `120 units/s` |
| `15 -> 20` | `80 units/s` |
| `20 -> 35` | `20 units/s` |

The continuous ideal is `1.1458333333s`, but the original completion boundary
is frame-quantized: each iteration yields `WaitForFixedUpdate`, and the object
is returned through `PoolingSystem.DestroyList` after local y passes
`NowPos.y + 35`.

While `localScale.x > 0.4`, the same loop subtracts `deltaTime / 3` from scale
x and y and writes z as zero. Each accepted Show call owns a fresh prefab and
coroutine; no text merge or coalescing path is present in this method.

## Dodge asset

The separate packaged `DodgeMent` animation clip has duration
`1.0166666507720947s` and the symmetric sprite sequence:

```text
7240, 7744, 7788, 7830, 7788, 7744, 7240
```

English text sprite path ID `7815` is `0007_text_3`. This asset contract is
separate from `DamageCtrl` type `3`, although both present the evade/dodge
outcome family.

## Damage effect lifetime

`DamageEffectCtrl.Show` activates its GameObject and sets local position to
`(0,0.15,0)`. `FixedUpdate` waits until its `ParticleSystem` reports stopped,
then returns the GameObject through `PoolingSystem.DestroyList`; there is no
independent hard-coded lifetime in the controller.

Validation:

```sh
python3 tools/analyze-original-native-combat-presentation-pass15.py
python3 -m unittest tools.tests.test_original_native_combat_presentation_pass15
```
