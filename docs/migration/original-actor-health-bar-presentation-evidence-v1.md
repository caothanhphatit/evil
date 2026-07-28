# Original Actor Health-Bar Presentation Evidence V1

## Scope

This record covers the ordinary world `Hunter` and `Evil` prefabs from Evil
Hunter Tycoon `1.411`. It identifies the serialized HP hierarchy, exact sprite
geometry, and the live-decrypted `InitHunterHpBar` / `InitEvilHpBar` behavior.
It does not claim boss bars, shields, PvP bars, or a complete death lifecycle.

Machine-readable evidence is stored at
`reverse-engineering/evidence/original-actor-health-bar-presentation-v1.json`.

## Serialized Prefabs

Both actor controllers expose a `HpBar` `GameObject` and `HpBarRender`
`SpriteRenderer`. The serialized references point to the colored fill child,
not the parent object named `HpBar`. This distinction matters because native
code scales only the fill around its left pivot.

The common source sprites are:

| Sprite | Source | Size | Pivot | Use |
| --- | --- | ---: | --- | --- |
| `hp_in` | `sharedassets0.assets:5625` | 18 x 2 | `(0, 0.5)` | dark inner background and colored fill |
| `hp_bg` | `sharedassets0.assets:7393` | 20 x 4 | `(0.5, 0.5)` | outer frame |
| `hp_lv_bg_9` | `sharedassets0.assets:3211` | 7 x 4 | `(0.5, 0.5)` | Hunter level badge |

All three sprites use `100` pixels per Unity unit. The parent `HpBar` object is
at local Unity Y `-0.06`, which projects to six pixels below the actor origin in
the Pixi Y-down coordinate system.

Monster layout:

```text
inner/fill x = -0.09 units (-9 px)
frame x      =  0.00 units
```

Hunter layout:

```text
inner/fill x = -0.065 units (-6.5 px)
frame x      =  0.025 units (2.5 px)
level frame  = -0.10 units (-10 px)
```

The Hunter shift is therefore not a guessed alignment. Its prefab reserves the
left side for the level badge. The current web slice preserves the HP fill and
frame geometry but does not render the badge until its font/text update path is
migrated.

## Native Initializers

An authorized Android API 35 ARM64 capture resolved these methods:

| Method | Token | Live module offset |
| --- | --- | --- |
| `EvilCtrl.InitEvilHpBar()` | `0x06002FCF` | `0x2F20414` |
| `HunterCtrl.InitHunterHpBar()` | `0x06005C11` | `0x33F79BC` |
| `HunterCtrl.HpFullSetting(string)` | `0x06005BC2` | `0x3435ED4` |

Both initializers divide current HP by maximum HP, write the result to the
fill object's local X scale, and preserve the other scale component. They apply
the same color thresholds:

| HP ratio | RGB |
| --- | --- |
| `>= 0.50` | `(102, 231, 32)` |
| `>= 0.20` and `< 0.50` | `(231, 102, 32)` |
| `< 0.20` | `(231, 48, 32)` |

At zero HP the initializer selects red and sets the fill scale to zero. The
serialized bar group itself is active. The rebuild therefore keeps an empty
frame for a dead projected actor. A later original death step may disable the
actor or status group, but that call path has not been proven and is not
invented here.

## Rebuild Contract

Protocol v21 introduced nullable `current_hp` and `maximum_hp` fields to each
`WorldEntityProjection`:

- monster values come directly from authoritative ephemeral `MonsterState`;
- Hunter values come from the durable authoritative Hunter aggregate;
- non-combat NPC projections use `null` and render no bar;
- the browser only calculates the visual ratio and never supplies HP outcomes.

Protocol v22 retains this HP contract and adds attack-effect presentation data;
HP authority and rendering semantics are unchanged.

The client uses the exact packaged `hp_in` and `hp_bg` sprites. Defensive ratio
clamping protects rendering from malformed packets, but the server remains
responsible for the invariant `0 <= current_hp <= maximum_hp`.

## Reproduction

Static prefab and sprite inspection uses the ordered `sharedassets0` splits and
the joined `sharedassets1.assets`. Runtime prefixes were captured with:

```sh
~/.local/share/evil-frida-venv/bin/python \
  tools/runtime/capture-il2cpp-native-methods.py \
  --adb /Users/trana/Library/Android/sdk/platform-tools/adb \
  --launch \
  --method HunterCtrl:InitHunterHpBar:0 \
  --method HunterCtrl:HpFullSetting:1 \
  --method EvilCtrl:InitEvilHpBar:0 \
  --code-bytes 4096 \
  --timeout 20 \
  --action 'Cold launch tutorial town; HP bar initializer prefixes only.' \
  --output /tmp/original-actor-health-bar-native-v1.json
```

Raw native prefixes are intentionally not committed. Their tokens, live module
offsets, bounded-prefix hashes, and reviewed semantics are retained in the
machine-readable evidence.

## Unresolved

- Hunter level-badge font and runtime text refresh.
- `ShieldBar` scale, color, and visibility behavior.
- Any post-death operation that disables the complete actor or `StatusGroup`.
- Boss, raid, guild, adventure, and PvP health-bar variants.
