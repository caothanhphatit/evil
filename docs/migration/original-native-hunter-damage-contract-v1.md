# Original Native Hunter Damage Contract v1

## Scope

This pass covers Evil Hunter Tycoon `1.411` on the authorized Android API 35
guest runtime. It captures native code and method metadata only. The result is
evidence-only and is not connected to live Rust combat.

```sh
python3 tools/analyze-original-native-hunter-damage-contract-pass9.py
python3 -m unittest tools.tests.test_original_native_hunter_damage_contract_pass9
```

## `getDamage` caller boundary

The exact native bodies contain 49 direct calls to
`HunterCtrl.getDamage(Boolean,Boolean,Boolean)`. Their complete argument-vector
distribution is:

| Vector | Direct callers |
| --- | ---: |
| `(false, false, false)` | 2 |
| `(true, false, false)` | 10 |
| `(true, false, true)` | 36 |
| `(true, true, false)` | 1 |

`HuntingAttackAction()` is the confirmed ordinary attack boundary and calls
`getDamage(false,false,false)` at `libil2cpp+0x34173D4`. The second caller with
that vector and all 47 nonzero vectors remain cataloged under their exact
managed method/token identities. Obfuscated method names and vector frequency
are not sufficient to assign the three Boolean product meanings.

This result corrects an earlier critical-path label: the second Boolean
(`w2`, preserved in `w22`) bypasses the critical roll inside `getDamage` when
true. The third Boolean (`w3`, preserved in `w21`) instead bypasses recovered
target-specific critical-damage and Slayer/Rift helper branches. These are
mechanical roles only; product-facing names remain unresolved.

## Proven Blizzard coefficient segment

`HunterCtrl.GDBMICDJBOK(Int32,Single,Single,Single)` is an exact 1,800-byte
body. It calls `getDamage(true,false,true)`. The second `Single` argument is
preserved in ARM64 `s10` and is the coefficient-percent input in this segment:

```text
result = trunc_i64(
  baseDamage
  * coefficientPercent
  * (1 + modifierAggregate)
  * 0.01
)
```

The resulting `Int64` is forwarded as parameter 6 of
`BlizzardCtrl.Action(Int32,String,Int32,Single,Single,Int64,Int32)`. The first
and third `Single` inputs are forwarded separately to the Blizzard action and
do not occupy the coefficient slot.

This is a single proven skill-family contract. The modifier accumulator has
captured native structure, but not every contributor has a semantic name. It
must not be generalized to all skills.

## `EvilCtrl.Damaged` parameters 3-6

The exact signature is:

```text
Damaged(String, Int64, Int32, Boolean, Int32, Boolean)
```

The four trailing parameters are preserved at entry and have these mechanical
roles:

| Parameter | Proven native use | Semantic status |
| --- | --- | --- |
| 3: `Int32` | Value `2` adds `EvilCtrl+0x1F8` to the pre-armor bonus; forwarded as `DamageManager.Show` parameter 1 | Product name unresolved |
| 4: `Boolean` | Forwarded to a post-damage virtual callback | Product name unresolved |
| 5: `Int32` | Forwarded as `DamageManager.Show` parameter 4 | Product name unresolved |
| 6: `Boolean` | Enables the captured `RidingPetGearProperty[16]` branch; when true bypasses the normal effective-armor path | Mechanical armor-bypass gate proven; product name unresolved |

`DamageManager.Show` is independently resolved as
`Show(Int32,Int64,Vector3,Int32,Boolean)`. Therefore parameters 3 and 5
participate in damage presentation, but their enum/display meanings are not
invented.

## Remaining boundary

- Product-facing meanings of all three `getDamage` Boolean arguments.
- Product-facing names of `EvilCtrl.Damaged` parameters 3 through 5.
- Virtual projectile/effect call paths that ultimately invoke
  `EvilCtrl.Damaged`; there is no direct ARM64 `BL` to that method in the
  captured Hunter code.
- Skill coefficient contracts other than the proven Blizzard builder segment.
- Exact target/tag semantics and content rows selecting each obfuscated caller.

These gaps stay fail-closed. No caller is mapped to a skill row from array
position, method order, localization similarity, or presentation asset name.
