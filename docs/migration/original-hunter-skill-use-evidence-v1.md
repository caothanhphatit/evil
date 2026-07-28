# Original Hunter Skill Use Evidence v1

## Scope

This report connects the packaged skill definitions, per-Hunter runtime skill
state, `HunterCtrl` attack/skill boundaries, native `HuntingAttackAction()`
evidence, Hunter Spine animations, and confirmed projectile assets. It does not
assign unresolved icons, effects, or animations to skills by name similarity.

The machine-readable evidence is generated at
`reverse-engineering/evidence/hunter-skill-use-runtime-v1.json` by:

```sh
python3 tools/extract-hunter-skill-use-evidence.py
python3 -m unittest tools.tests.test_hunter_skill_use_evidence
```

Read this together with
`docs/migration/original-hunter-weapon-attack-presentation-evidence-v1.md`.
That report proves how the selected weapon is composed into the Hunter Spine
actor and how basic front/back attack clips expose the corresponding slot.

## Confirmed skill catalog

The packaged QuickSheet data contains ten basic skills, two for each base job:

| Index | Job | English | Vietnamese | Max level | Base cooldown |
| ---: | ---: | --- | --- | ---: | ---: |
| 0 | 0 | Fury | Cuồng Nộ | 10 | 15 s |
| 1 | 0 | War Cry | Thét Xông Trận | 5 | 16 s |
| 2 | 1 | Holy Light | Ánh Sáng Thần | 10 | 8 s |
| 3 | 1 | Barrier | Lá Chắn | 5 | 16 s |
| 4 | 2 | Multishot | Liên Thanh | 10 | 6 s |
| 5 | 2 | Dodge | Tránh Né | 5 | 16 s |
| 6 | 3 | Thunderbolt | Lôi Tiễn | 10 | 6 s |
| 7 | 3 | Ice Armor | Giáp Băng | 5 | 16 s |
| 8 | 4 | Round Slash | Chém Vòng Cung | 10 | 6 s |
| 9 | 4 | Concentrate | Tập Trung | 5 | 16 s |

Each row also contains its exact packaged level arrays for duration, effect
value, secondary value/count, study level, and study money. The evidence JSON
preserves those arrays without converting their units or interpreting malformed
localization placeholders.

The same source contains 40 sub-job skill definitions. Each row preserves its
`job`, `subJob`, `thirdJob`, and `fourthJob` path plus base and per-level deltas
for cooldown, duration, primary value, secondary value, count, and Soul study
cost. This proves the content definitions. It does not prove which nodes a live
Hunter has learned or which UI icon belongs to an unresolved row.

## Per-Hunter skill state

Runtime reflection confirms `HunterData.skill` is a
`Dictionary<String, SkillData>`. `SkillData` contains exactly four fields:

| Field | Runtime type | Offset |
| --- | --- | ---: |
| `index` | `ObscuredInt` | 16 |
| `skillIndex` | `ObscuredInt` | 32 |
| `coolTime` | `ObscuredDouble` | 48 |
| `level` | `ObscuredInt` | 88 |

This is direct evidence that learned skill identity, level, and current
cooldown can be snapshotted per Hunter. The captured schema does not resolve the
dictionary key format, the meaning of both integer IDs, or live values for a
specific Hunter.

`HunterCtrl.CheckManaOrb()` and `GetManaOrbPower()` prove a mana-orb mechanic
boundary exists. They do not prove that every skill consumes it, and the
captured 109-field `HunterData` has no confirmed generic mana-pool field.

## Attack-to-skill dispatch boundary

The recovered native body for `HunterCtrl.HuntingAttackAction()` is 8,016 bytes
and directly calls both ordinary/special attack helpers and trait/effect checks.
Confirmed named calls include:

- `HunterCtrl.FireDamageAction(long, int)`;
- `HunterCtrl.PulverizeAttack(EvilCtrl)`;
- `HunterCtrl.GetMisticArrow()`;
- `EvilCtrl.WarcryAction()` and `EvilCtrl.CurseDamage()`;
- `HunterCtrl.CheckJobTrait()`, `CheckFamiliar()`, and `CheckHandsOfGod()`;
- `HunterCtrl.getDamage()`.

The same body references `mTargetEvil`, `AttackAniTime`, `mNowAnimation`,
`TargetAttackCount`, `mEffect`, and `mSkillMent`. This supports the following
bounded action flow:

1. `HuntingAttackSetting()` enters attack state after target/range handling.
2. `HuntingAttackAction()` selects an ordinary, skill, trait, familiar, or
   effect branch and updates the Hunter attack-presentation state.
3. Melee and ranged branches can use different helpers. The ranged boundary is
   explicit through `RangeFireDamageAction`, `RangedDualAttack`, and the
   coroutine-returning `FireDamageAction`.
4. `HuntingAttackEnd()` closes the attack action, while `RefreshAnimation()`
   refreshes presentation after equipment/job state changes.

This is not yet an exact decompile of the branch conditions. Do not hard-code a
skill priority or proc probability from call order alone.

## Animation and projectile presentation

Besides the six base weapon-family attack pairs, the Hunter Spine actor contains
named advanced clips including:

- `h1_hit_whirlwind`;
- `h2_hit_executor` and `_back`;
- `h3_hit_arcane` and `_back`;
- `h4_hit_darkload` and `_back`;
- `h5_hit_roundslash` and `_back`;
- `h5_hit_shadejavelin`;
- `h5_hit_dragonbreath_vehicle` and `_back_vehicle`.

These names prove packaged presentation capabilities. They do not by themselves
prove exact QuickSheet row bindings.

`atk_ranger__3599.png` is stronger evidence: its Unity sprite name is
`atk_ranger`, its migration role is `ranger-arrow-projectile`, and its binding
state is `scene-component-confirmed`. `atk_sorcerer__4256.png` is present in the
export, but no equally strong scene-component binding has been recovered, so it
remains a candidate rather than an implementation mapping.

## Rebuild implementation boundary

The clean implementation should preserve the confirmed original presentation
while following the rebuild's server-authoritative architecture:

1. Equipment resolution selects one valid weapon skin family and composes that
   attachment into the existing Hunter Spine actor.
2. The server owns target acquisition, skill readiness, cooldown, proc/RNG,
   damage, HP, death, XP, drops, and resulting durable state.
3. A server attack event carries the selected attack/skill presentation key,
   target ID, locked facing, timing sequence, and optional projectile/effect key.
4. The browser starts the selected Spine clip once per event and advances it
   locally without restarting from repeated snapshots.
5. Ranged presentation spawns the confirmed projectile only after the content
   binding is resolved. Hit presentation reconciles to the authoritative result.
6. Unknown skill-to-animation/icon/effect mappings remain unavailable, not
   silently mapped by numeric order or English name.

This separation allows the Hunter to visibly hold equipment and attack smoothly
without trusting the browser to decide damage or rewards.

## Unresolved evidence

- Exact gear definition index to Hunter weapon skin.
- Exact base/sub-job integer branch selecting each weapon and advanced clip.
- Exact skill row to icon, effect-controller index, animation, projectile, and
  sound for mappings not independently confirmed.
- Exact `HunterData.skill` dictionary keys and live learned-state values.
- Exact native attack priority, proc checks, hit frame, projectile spawn frame,
  cooldown mutation point, and mana-orb use.
- Exact original damage/critical/effect formulas outside separately recovered
  native evidence.

Until those are recovered, the runtime may implement explicitly named rebuild
policies for test fixtures, but must not label them as original-game behavior.
