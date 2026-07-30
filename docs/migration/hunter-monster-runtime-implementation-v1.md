# Hunter And Monster Runtime Implementation v1

## Delivery tasks

1. **Hunter selection and command UI** - complete. Clicking a world Hunter opens
   the Info/Command bubble. `Di Chuyển` exposes `Thuộc Địa`, `Tử Địa`, and
   `Ma Giới` in the recovered ordinary-region order.
2. **Authoritative region assignment** - complete. The browser sends only the
   Hunter ID and region ID through `assign_hunter_hunt`; the Rust session owns
   the resulting hunt state. A player-issued ordinary-region assignment is the
   highest-priority Hunter task: it preempts an unfinished town task, refunds an
   unfinished paid service and its consumed stock, and publishes the
   `EnteringRegion` FSM in the accepted snapshot. Reassignment also normalizes
   an older persisted bridge checkpoint to the furthest reached entry waypoint;
   a position outside the assigned field, town floor and recovered bridge
   checkpoints is relocated to a clear town anchor before routing. Dead Hunters
   remain rejected. Migration `0029` keeps PostgreSQL's normalized Hunter
   action constraint aligned with the durable `entering_region` FSM state, so
   an accepted assignment cannot disconnect during checkpoint persistence.
3. **Shared-world navigation** - complete. Town and the three hunting regions
   remain in one actor roster and coordinate space. Hunter assignment changes
   AI destination, not world ownership or scene lifetime. Clearing a field
   assignment now preserves the Hunter's current position and walks through the
   town-arrival corridor instead of relocating directly to a town anchor.
   Returning low-HP Hunters with no owned potion continue to an obstacle-safe
   Infirmary interaction point when an affordable stocked treatment exists.
4. **Hunter and monster FSM runtime** - complete for the basic loop. Typed
   states cover entry, target acquisition, chase, attack, loot collection,
   patrol, death, and respawn.
5. **Catalog-backed ordinary monsters and rewards** - complete for base rows.
   Region and global-difficulty pools, HP, damage, armor, EXP, gold, and material
   arrays come from the normalized `1.411` catalog.
6. **Density reconciliation** - complete. Each wooden sign changes only its own
   region and adds or removes active monsters without rebuilding another region.
7. **Animation projection** - complete for verified clips. Hunter class attacks,
   Hunter walk/death, and `mon_a_01_1` walk/attack/death use exact Spine keys.
   Hunter presentation composes the confirmed first packaged weapon-family skin
   and explicitly binds its recovered slot/attachment (`sword`, `hammer`, `bow`,
   `wand`, or `spear`). This makes the demo actor visibly carry its class weapon;
   it is not claimed as an equipped gear-index mapping.
   Ranger (`H3`) and Sorcerer (`H4`) now retain a ranged attack envelope while
   the melee families continue to close distance. Native evidence confirms a
   job-dependent `mRange` branch, but the current scene-pixel values remain
   isolated rebuild tuning until the decoded native values are mapped.
   Ranger basic attacks also use the packaged `atk_ranger` projectile. The
   serialized scene binds sprite path ID `3599` under
   `ArrowFallenPasture/ArrowCtrl/Image`, so this is not a drawn fallback.
   Protocol v22 carries the authoritative target entity, attack sequence, and
   an explicit attack-effect key;
   the latter restarts the one-shot attack clip and emits one arrow per server
   hit. The exact native spawn/hit frame remains unresolved, so arrow travel is
   bounded to the confirmed 0.3333-second H3 basic-attack clip. H4 remains
   ranged but has no projectile until its exact wand-effect binding is mined.
8. **Verification** - server, web, content, build, and local interaction checks
   are required before handoff.
9. **Shared scene coordinates** - complete. Authoritative Hunter and monster
   positions use `scene_pixels_v1`, the same 3072 by 1536 coordinate space as
   the recovered Unity scene. They are never normalized into the town bounds.
10. **Movement presentation contract** - complete for the current 10 Hz
    runtime. Protocol v22 publishes a lightweight `world_frame` at the fixed
    simulation cadence instead of serializing the complete player aggregate on
    every tick. Full domain snapshots remain command/resync responses and no
    longer block the movement loop on a periodic timer. The browser advances
    walking actors locally between confirmations with bounded dead reckoning;
    idle, attack, death, teleport, and mode changes still reconcile immediately.
    Pixi consumes the smooth render timeline directly instead of applying a
    second target blend that previously pulled actors backward. Hunter facing
    follows the packaged skeleton's setup orientation.
11. **Ordinary-region entry routing** - partial. Hunters enter through the
    exact recovered bridge and density-sign scene anchors before moving into
    the corresponding safe field extent. The left and southern routes use
    `Village_Bridge_C`; the eastern route uses `Village_Bridge_B`. Each route
    now approaches from a tested safe-floor anchor before crossing the
    recovered bridge center and leaving from the sprite's field-side edge.
    Density signs remain interaction controls and are no longer treated as
    walkable waypoints.
    Original path-helper and navigation-polygon semantics remain unresolved.
    Bridge sprites render as walkable floor surfaces above the town ground but
    below actors, so the exact route no longer visually passes underneath the
    bridge artwork. The asset pipeline also preserves the original Unity sprite
    pivot after transparent texture trimming; bridge B/C are no longer shifted
    vertically relative to their recovered scene transforms.
12. **Town obstacle ownership** - complete for rebuild building footprints.
    The server derives collision rectangles from authoritative building grid
    placements and routes Hunters around them. The browser no longer snaps
    Hunter Y positions around visual building bounds, which previously caused
    visible oscillation. Integration tests now trace all three field routes and
    reject every Hunter position that enters a building footprint plus actor
    clearance. Static original-scene navigation polygons remain an evidence
    gap.
13. **Monster patrol cadence** - implemented at the recovered FSM boundary.
    Monsters alternate explicit `Idle` and `Patrolling` states, use `stay` only
    while stationary and `walk` only while advancing, and complete one bounded
    waypoint before the next idle. `EvilCtrl.FsmMoveEnd` confirms the original
    waypoint-end transition, but not its numeric radius or pause duration.
14. **Town Hunter movement** - implemented as an explicit rebuild fixture.
    Unassigned Hunters follow deterministic per-Hunter waypoint permutations,
    keep a destination until they reach it, and use staggered per-journey pause
    durations. This avoids the former shared global cadence that made the town
    roster march in rows and reverse direction together. Movement still uses
    the server obstacle solver and remains inside the confirmed safe town floor.
    No native town-roam waypoint table has been recovered, so the route order
    and pause distribution are presentation tuning rather than a claim about
    original cadence.
15. **Reconnect continuity** - complete for active Hunter agents. PostgreSQL
    checkpoints preserve each Hunter's scene position, facing, FSM action,
    animation, combat target, recovery/respawn timers, presentation sequences,
    active temporary skill state, region-entry stage, and town-roam journey and
    pause state. Re-entering the game restores that state before the welcome
    snapshot is projected. Monster
    actors and ground drops remain reconstructable ephemeral state; a persisted
    target is retained only when the regenerated monster ID is live, while an
    interrupted loot action falls back to target acquisition at the same Hunter
    position because its referenced drop no longer exists.
16. **Loot pickup completion** - an initiated pickup finishes its short
    authoritative recovery before a newly aggroed monster can retake the
    Hunter's FSM. Gold remains wallet-only, material inventory receives only
    `material:*` rows, and pickup text includes the collected quantity. The
    exact original pickup cadence and ground-gold sprite remain unresolved.

## Evidence-backed behavior

- The native Hunter and Evil controllers use queued FSM action boundaries.
- The three ordinary region order and Vietnamese labels are recovered in
  `original-hunter-movement-animation-revival-evidence-v1.md`.
- Ordinary monster definitions and material base rolls are recovered in
  `ordinary-hunting-monster-map.json` and `monster-runtime-catalog.json`.
- The material loop rolls each source slot independently in `1..=10_000` and
  succeeds inclusively when `rawPercent * 10 >= roll`.
- Verified Hunter and monster animation names come from the packaged Spine
  skeletons. Goldblin has no attack clip and is therefore not used as an attack
  actor until its gameplay/prefab binding is resolved.

## Temporary runtime tuning

Native evidence does not yet resolve ordinary-field movement speed, perception
radius, attack range, recovery time, respawn time, the complete damage modifier
chain, or the exact revival coordinate. The implementation isolates those
values as named constants in `monster_world.rs` and does not claim them as
original balance.

Current temporary choices are:

- density counts `3 / 6 / 9` per ordinary region;
- fixed integer movement steps and attack ranges;
- melee attack range `42` scene pixels and H3/H4 ranged attack range `150`
  scene pixels;
- Hunter movement averages exactly `7.5` scene pixels per simulation tick by
  alternating deterministic `7 / 8` pixel steps. This is the requested `1.5x`
  product adjustment from the previous temporary `5` pixel step, not an
  original-game value;
- Hunter and monster attack recovery ticks;
- monster patrol radius `64` scene pixels and a 2.5-second idle (the arrival
  frame plus a `24`-tick countdown at the authoritative 10 Hz simulation);
  both are product tuning,
  not recovered original values;
- monster and Hunter respawn ticks;
- compatibility scaling for catalog monster damage against demo Hunter stats;
- the operational world constructor currently starts at global difficulty `0`
  and has no difficulty-transition command, so only the nine difficulty-zero
  rows are live; the other 36 exact rows remain catalog-backed but unreachable
  until the difficulty gate is recovered or deliberately implemented;
- active spawn type order uses `spawn_index % 3` across each three-row pool.
  The packaged row mapping is exact, but the original type-selection/random
  order is unresolved, so this deterministic order is rebuild fixture policy;
- the town revival anchor;
- town-roam waypoint anchors, deterministic per-Hunter route permutation and
  staggered pause cadence (temporary fixture only; native waypoint semantics
  remain unresolved);
- ordinary-field extents extending outward from the exact recovered
  `sign_01`, `sign_02`, and `sign_03` transforms; the extents are isolated from
  the town building zone and remain temporary until navigation polygons are
  recovered;
- demo Hunter required-EXP growth after level-up while the fixture class to
  original job-column binding remains unresolved.

## Remaining gaps

- Exact monster source-index to prefab/Spine-family mapping.
- Unique gear pool selection and modifier order.
- Full EXP/gold/drop modifier chain.
- Original navigation/pathfinding helpers and numeric tuning.
- Original revival point and revive-building routing.
- Ground-drop sprite/icon projection; reward ownership and collection already
  run on the server, but the exact presentation binding remains unresolved.
- Crash recovery is bounded by the current world-checkpoint interval. Graceful
  disconnect persists the latest runtime state, while abrupt process loss may
  restore the last completed checkpoint rather than the last rendered frame.
