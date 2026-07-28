# Monster World BE v1

## Scope

This vertical slice implements three server-authoritative hunting-region fixtures inside one village world instance. Town and all three hunting regions remain alive concurrently; camera focus never replaces the world or despawns another region. Every hunting region owns its own monster runtime and wooden density board. A board changes only its region's density I/II/III. Account/world difficulty remains a separate global modifier shared by the entire instance.

## Evidence boundary

The recovered API35/runtime inventory exposes monster-related schema keys (`MonsterCountType01..04`, `MonsterZoneRespawnCount01..04`, `MonsterZoneRespawnSecond01..04`, `MonsterMiddleBossCount01..04`, `MonsterRespawnTime01..04`) but does not prove their numeric values. The runtime now consumes exact ordinary monster catalog rows while keeping density counts, movement/combat timing, damage compatibility, and respawn timing explicitly labeled temporary tuning; they must not be treated as original balance.

Mined Spine bundles reused by the slice:

- `mon_a_01_1`: `apps/web/public/content/releases/visible-world-v1/actors/mon_a_01_1/`
- `mon_goldblin`: `apps/web/public/content/releases/visible-world-v1/actors/mon_goldblin/`

The three density boards are exact scene assets rather than redrawn UI. `sign_01`, `sign_02`, and `sign_03` have confirmed Unity transforms, circle colliders, and three source sprite states (`area_sign_*_0..2`) in `level1-scene-evidence-v2.json`. Their region bindings remain fixture bindings until original controller data confirms the semantic association.

## Runtime contract

`set_monster_region_density` is the world-object board command and carries both `region_id` and the next I/II/III level. It is accepted while the shared world is visible and never changes camera focus. Legacy `enter_monster_map` and `set_monster_density` remain protocol-v18 compatibility commands; their `map` wording means presentation focus, not world ownership. Unknown regions, density values outside I/II/III, dead/unknown monsters, and dead/unknown hunters are rejected. Density III exposes the observed cluster banner `Quái vật đang tập trung tại Tử Địa`. Exact numeric density counts remain explicit fixtures.

The fixture tick is deterministic: monsters move one lane unit per tick, acquire the first living active hunter, deal fixture damage every five ticks, take fixture damage every three ticks, emit one `material:32` drop on death, and respawn after eight ticks. These rules are scaffolding for migration tests, not claims about legacy rates.

Monster actors, HP, targets, respawn timers, ground drops, and camera focus are ephemeral session runtime and are deliberately excluded from `DurablePlayerAggregate` and PostgreSQL aggregate JSON. Only the three per-region density levels in `monster_field_config` are durable; reconnect reconstructs fresh deterministic runtime for all three regions. A drop becomes durable only when a future atomic, idempotent pickup/claim ledger settles it; death alone never persists a reward.

## Open migration work

- Resolve exact tier map IDs, monster families, density counts, spawn/rate/drop tables from API35 runtime capture.
- Replace fixture drop IDs and damage values with evidence-backed content.
- Replace the three explicit fixture map assets and centered portrait spawn anchors when exact map bindings and coordinates are recovered.
- Add command-id idempotency persistence for monster intents before production use.
