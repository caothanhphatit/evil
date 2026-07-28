# Field Hunting Runtime V1

## Purpose

This document defines the server-authoritative boundary for ordinary field
hunting. It replaces the current `web-rebuild-v1-fixture` tick behavior only
after the remaining native control-flow evidence has been recovered. It does
not define unverified formulas or rates.

## Evidence Boundary

Package and runtime evidence currently confirms:

- `evil` contains 195 monster rows with source index, area, type,
  `createLevel`, HP, armor, damage, EXP, gold, material indices, material
  counts, material percentage values, race, and localized names.
- Areas `0`, `1`, and `2` each contain three ordinary monster types for global
  difficulty values `0..4`. Rows using `createLevel == 5` include special or
  advanced content and are not part of the ordinary three-field mapping yet.
- `dropUniqueGear` contains 61 rows with drop range, cuts, gear types, gear
  indices, and gear percentage values.
- The ordinary material loop visits source-array slots in ascending order,
  scales each raw percentage value by `10`, and rolls independently in
  `[1, 10_000]`. The raw percentage denominator is therefore `1,000`; the
  effective integer threshold uses denominator `10,000` and succeeds
  inclusively when `effectiveThreshold >= roll`.
- `HunterCtrl` and `EvilCtrl` both use queued finite-state-machine actions.
  Movement, target selection, attack setup/action/end, death, rewards, and
  material rewards are separate native methods.
- `HunterCtrl.Reward` delegates durable EXP and gold changes to `PlusExp` and
  `PlusGold`. `RewardMetrial` is a separate material/gear reward boundary.
- Monster actors and their current targets, HP, positions, action queues, and
  respawn clocks are runtime state. Only player-owned value and field density
  configuration require durable persistence.

Still unresolved:

- the complete modifier chain applied to base material percentage values;
- the unique-gear row selection and `dropRange`/`dropCut` evaluation order;
- exact ordinary-field target priority, leash radius, movement speed, attack
  range, wind-up, recovery, and respawn timing values;
- the mapping from every monster catalog row to its actor asset/prefab;
- modifiers applied to base EXP and gold before `PlusExp` and `PlusGold`.

Monster source index `34` packages 13 material indices, 13 counts, and 14 raw
percentage values. The recovered primary loop is bounded by the indices array,
so its trailing percentage value is preserved as evidence but is not consumed
by that loop.

## Authoritative Model

The browser sends intent and renders confirmed projections. The Rust world
simulation owns every gameplay transition and reward outcome.

Town and the three ordinary hunting areas belong to one village world
instance. They are regions of one simulation, not separate map instances. A
camera/viewport focus change may select a region for presentation, but it must
not despawn the other regions, replace their actor roster, or create another
world owner.

```text
content release
  -> world-region spawn planner
  -> perception and target selection
  -> navigation and action queue
  -> combat resolution
  -> death event
  -> reward resolver
  -> transactional hunter/economy mutation
  -> world delta for presentation
```

No client command may contain trusted damage, death, EXP, gold, material,
equipment, or drop results.

## Content Definitions

The canonical monster definition key is the packaged source `index`. For the
ordinary three hunting regions, lookup is additionally indexed by:

```text
(area, type, createLevel) -> monster source index
```

This tuple is unique for areas `0..2` and difficulty values `0..4`. The source
index remains the primary key so special rows and future duplicate tuple cases
cannot collide.

Definitions are immutable within a content release and contain only recovered
catalog values. Spawn positions, region anchors, active entity IDs, current HP,
and targets are runtime state and must not be stored in the content catalog.

The runtime instance owns all four regions concurrently:

```text
VillageWorldInstance
  - TownRegion
  - HuntingRegion(area=0)
  - HuntingRegion(area=1)
  - HuntingRegion(area=2)
```

`currentRegion` is an ephemeral camera/interaction focus only. Density remains
configured independently per hunting region, while global difficulty applies
across the account/world according to the recovered product rule.

## Runtime Components

### Hunter agent

The Hunter agent is split into independently testable components:

- `HunterPerception`: obtains eligible monsters in the current field.
- `HunterTargetPolicy`: selects or rejects a target using recovered rules.
- `HunterActionQueue`: executes recovered FSM action codes in order.
- `HunterNavigation`: requests a path and advances along it on fixed ticks.
- `HunterCombat`: validates range and cadence and emits attack facts.
- `HunterLootIntent`: moves to or claims server-owned drops when allowed.
- `HunterTownRoutine`: returns, sells materials, buys services, and resumes the
  assigned hunt through explicit domain commands.

The expected high-level lifecycle is:

```text
Town -> EnterField -> AcquireTarget -> Navigate -> Attack -> Recover
  -> MonsterDead -> ClaimDrops -> ContinueHunt
  -> ReturnTown -> Sell/Service -> EnterField
```

This lifecycle is product-confirmed, but transitions must not receive guessed
numeric thresholds while native values remain unresolved.

### Monster agent

The monster agent is similarly separated:

- `MonsterPerception`: observes eligible Hunters within recovered bounds.
- `MonsterTargetPolicy`: selects, retains, or clears a Hunter target.
- `MonsterActionQueue`: executes the recovered `EvilCtrl` FSM sequence.
- `MonsterNavigation`: handles roam, chase, correction, and return-to-anchor.
- `MonsterCombat`: validates range/cadence and emits attack facts.
- `MonsterLifecycle`: owns spawn, death, reward emission, and respawn.

The target state graph is:

```text
Spawn -> Idle/Roam -> AcquireTarget -> Chase -> Attack -> Recover
  -> Chase | ReturnToAnchor | Dead -> Respawn
```

This graph is an implementation boundary, not a claim that every transition or
timer has already been recovered.

## Reward Resolution

Monster death emits a fact containing the monster source index, content
release, field, global difficulty, killer Hunter, encounter RNG stream, and a
stable operation key. The reward resolver then:

1. loads EXP and gold base values from the exact monster definition;
2. applies only recovered modifiers in their recovered order;
3. evaluates material entries in source-array order using
   `baseThreshold = rawPercent * 10` and an independent integer roll in
   `[1, 10_000]` for each entry; an entry succeeds when the recovered effective
   threshold is greater than or equal to the roll;
4. evaluates unique gear through the recovered `dropUniqueGear` algorithm;
5. commits EXP, wallet, inventory, and reward ledger changes atomically;
6. publishes presentation drops and progression deltas after commit.

Material and gear rewards must use independent, server-owned deterministic RNG
streams so retries remain idempotent and adding a cosmetic roll cannot change
economy outcomes.

## Persistence

Durable PostgreSQL state:

- Hunter progression, HP/death state when required by the aggregate;
- Hunter inventory/equipment/materials and player wallet;
- field density setting per map;
- command and reward ledgers with idempotency keys;
- content release and global difficulty pinned to the player/session.

Ephemeral world state:

- monster instances across all three regions, HP, targets, paths, action
  queues, and respawn clocks;
- active Hunter positions, targets, paths, and combat timers;
- unclaimed presentation drops, provided their value mutation is already
  represented by a durable reward operation or recoverable pending event.

## Migration Rule

Do not extend the existing modulo-tick fixture. Replace it vertically in this
order:

1. publish the normalized monster and unique-drop catalogs;
2. introduce typed action queues and actor state without changing rewards;
3. port Hunter perception/navigation and verify deterministic movement tests;
4. port monster perception/navigation and verify chase/leash tests;
5. port attack cadence and death facts;
6. port EXP/gold, then material and unique-gear resolution as each algorithm is
   confirmed;
7. remove fixture stats, modulo-tick damage, and hard-coded material drops;
8. run server, protocol, web projection, persistence, and replay tests.

Every intermediate release must label unresolved behavior explicitly and must
not silently substitute plausible constants.
