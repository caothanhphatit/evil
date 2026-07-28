# Original Hunter Weapon And Attack Presentation Evidence v1

## Scope

This report covers only the packaged Hunter/monster Spine data, the normalized
`1.411` gear catalog, the API 35 IL2CPP type schema, and the bounded native AI
methods already committed to the repository. It does not infer an equipment
row-to-skin mapping or name obfuscated native helpers.

Machine-readable evidence is generated at
`reverse-engineering/evidence/hunter-weapon-attack-presentation-v1.json` by:

```sh
python3 tools/extract-hunter-weapon-attack-presentation.py
python3 -m unittest tools.tests.test_hunter_weapon_attack_presentation
```

## How a Hunter carries a weapon

The Hunter actor is one Spine 4.2 skeleton with 56 slots, 1,937 skins, and 70
animations. A visible weapon is not an independent Pixi world sprite. It is a
Spine skin attachment composed into a weapon slot whose bone is already part of
the Hunter rig.

The exact packaged base families are:

| Visual family | Skin prefix | Spine slot | Attachment key | Skin count |
| --- | --- | --- | --- | ---: |
| `h1` | `weapon_h1_` | `weapon_01` | `sword` | 185 |
| `h1a` | `weapon_h1a_` | `s_weapon` | `s_weapon` | 185 |
| `h2` | `weapon_h2_` | `weapon_02` | `hammer` | 185 |
| `h3` | `weapon_h3_` | `weapon_03` | `bow` | 185 |
| `h4` | `weapon_h4_` | `weapon_04` | `wand` | 185 |
| `h5` | `weapon_h5_` | `weapon_05` | `spear` | 116 |

`h3` additionally owns the `weapon_03_effect` slot. The rig also contains
secondary/shield slots used by advanced variants. These are evidence that the
renderer must use Spine skin composition; they are not permission to assign an
advanced weapon to a Hunter whose sub-job state is unresolved.

The gear catalog independently contains 315 weapon definitions: 63 rows for
each numeric job `0..4`. Their first rows are respectively sword, hammer, bow,
staff, and spear families. The catalog does not contain the exact Spine skin
name, so `GearDefinition.index -> weapon_h*_...` remains unresolved.

## Attack animation contract

The packaged basic attack clips are explicit directional pairs:

| Family | Front clip | Back clip | Packaged duration |
| --- | --- | --- | ---: |
| `h1` | `h1_hit` | `h1_hit_back` | 0.3333 s |
| `h1a` | `h1_a_hit` | `h1_a_hit_back` | 0.3333 s |
| `h2` | `h2_hit` | `h2_hit_back` | 0.3333 s |
| `h3` | `h3_hit` | `h3_hit_back` | 0.3333 s |
| `h4` | `h4_hit` | `h4_hit_back` | 0.3333 s |
| `h5` | `h5_hit` | `h5_hit_back` | 0.3333 s |

Within each basic clip, the family weapon slot starts opaque and unrelated base
weapon slots start transparent. For example, `h3_hit` exposes `weapon_03` and
hides sword, hammer, wand, spear, and secondary-sword slots. The attack clip
therefore owns pose and visibility while the selected weapon skin supplies the
actual attachment image.

`HunterCtrl.HuntingAttackSetting()` establishes the target/range attack state.
`HunterCtrl.HuntingAttackAction()` then owns the combat action and mutates:

- `AttackAniTime` at offset `428`;
- `mNowAnimation` at offset `496`;
- `mAttackCheck` at offset `500` through the attack boundary;
- `mTargetEvil` at offset `888`;
- `TargetAttackCount` at offset `896`.

The exact native integer-to-clip branch and exact damage-impact frame are still
unresolved. Do not use the end of a 0.3333-second clip as a damage formula fact.

## Monster facing during chase and attack

The confirmed `mon_a_01_1` actor has separate presentation clips:

| State | Front | Back | Duration |
| --- | --- | --- | ---: |
| Walk | `walk` | `walk_b` | 1.3333 s |
| Attack | `atk` | `atk_b` | 0.8333 s |

The `_b` clips replace body, head, hand, and leg attachments with their `_b`
variants. The weapon remains the same `weapon` attachment and is animated by
the rig. It must not be mirrored or repositioned as a separate sprite.

Native `EvilCtrl.FixedUpdate()` reads `mTargetUnit`, `mTransform`, and
`mCharacter`, compares target-relative transform values, and uses two stored
Quaternion fields (`PBOCIECIFIP` at `168`, `CJGMDHPGAPL` at `184`) in separate
presentation branches before attack dispatch. This confirms that facing is
derived from the live target relationship and applied through the character
transform. It does not yet prove which world-axis/component selects `atk` versus
`atk_b`, nor the semantic names of the two Quaternion values.

## Clean implementation boundary

The rebuild should keep these concerns separate:

1. Server combat state owns target acquisition, chase, attack start, attack
   recovery, damage, death, and the authoritative target ID.
2. The world projection carries a stable attack state, target ID, and a small
   directional presentation value. Facing should be locked when entering an
   attack and only recalculated when the target changes or the attack ends.
3. The browser composes the Hunter's base appearance skin plus exactly one
   resolved weapon skin on the existing Spine skeleton.
4. The browser selects the packaged family clip and its `_back` partner; it
   never creates a second weapon sprite or restarts the same clip every render
   frame.
5. Until the original axis rule is recovered, the chosen front/back comparator
   must be an explicitly named rebuild presentation policy with a test. It must
   not be described as the original formula.

This model also avoids visible attack-direction jitter: target-relative facing
is sampled at the attack transition, not continuously flipped while both actors
interpolate around the range boundary.

The current rebuild uses an explicitly named Y-down presentation policy for the
confirmed `mon_a_01_1` front/back clips: a target above the monster selects
`walk_b`/`atk_b`, otherwise it selects `walk`/`atk`. This is tested product
presentation behavior, not a claim that the unresolved native comparator has
been recovered.

## Unresolved evidence

- Exact gear index to Hunter Spine weapon skin.
- Exact `HunterData` job/sub-job integer branches selecting `h1a` and advanced
  attack variants.
- Exact `EvilCtrl` transform component selecting front/back presentation.
- Exact horizontal mirror/rotation helper semantics for all monster families.
- Exact hit frame, projectile spawn frame, and damage-application ordering.
- Whether every packaged monster family has explicit `_b` clips or some use a
  transform mirror instead.

The failed read-only Frida attempt to capture `HunterCtrl.RefreshAnimation()`
and the obfuscated Evil attack helper timed out while injecting into the active
API 35 process. No new native claim is made from that failed attempt.
