# Hunter System

Status: working specification
Package baseline: Evil Hunter Tycoon `1.411`
Last reviewed: 2026-07-26

## Purpose

Hunters are player-owned character aggregates that visit and live in the town,
fight monsters, carry personal resources and equipment, learn job-bound skills,
and use town services. The Rust server owns Hunter generation, progression,
inventory mutations, skill state, simulation outcomes, RNG, and persistence.

Detailed autonomous behavior and the conversation command taxonomy are specified
in [Hunter lifecycle and command system](hunter-command-system.md). Generated
Characteristics and their effect boundaries are specified in
[Hunter personality system](hunter-personality-system.md).

## Evidence boundaries

This specification combines:

- **Package-confirmed** QuickSheet and runtime-schema evidence under
  `reverse-engineering/evidence/`.
- **Official-reference** behavior from the publisher's Vietnamese help article,
  [Thợ Săn](https://evilhunter.zendesk.com/hc/vi/articles/360039882831-Th%E1%BB%A3-S%C4%83n).
- Future **User-raw** tables, which must be reviewed before becoming normative.

Official-reference rates must be versioned separately from the supplied `1.411`
package when package evidence does not independently confirm the same RNG table.

## Aggregate ownership

```text
Account 1--1 Player
Player 1--N Hunter
Hunter N--1 JobDefinition
JobDefinition 1--N SkillDefinition
Hunter 1--N HunterSkillSnapshot
Hunter 1--1 HunterInventory
HunterInventory 1--N ItemInstance
Hunter 1--N MaterialStack
Hunter 1--N StatSnapshot
```

- One account owns one durable player aggregate.
- One player owns multiple active or waiting Hunters.
- Each Hunter has one current base job and one valid advancement path.
- Each Hunter has one generated personality/Characteristic, separate from Job
  Traits and advancement-tree state.
- Skill definitions are shared content selected through the Hunter's job path.
- Learned level, cooldown state, unlock state, items, materials, equipment,
  appearance, traits, growth and derived-stat snapshots belong to the Hunter.
- Shared definitions must not be copied into every Hunter row.

## Jobs and skills

The official reference lists five base jobs:

1. Berserker
2. Ranger
3. Paladin
4. Sorcerer
5. Dark Knight

Package evidence contains 10 basic skills, two for each base job, and 40
class-change skills keyed by `job`, `subJob`, `thirdJob`, and `fourthJob`.
QuickSheet rows also expose level limits, cooldowns, effect parameter arrays,
study requirements, study costs and localized text.

A Hunter may only learn a skill that is eligible for its current job path. The
server must validate this relationship on every learn, upgrade, equip, reset or
job-change command. Per-Hunter learned state is a snapshot referencing the
shared skill definition.

Exact combat execution order, rounding, stacking and most skill icon/VFX
bindings remain unresolved. Fury to `skill_h1_01` and War Cry to `skill_h1_02`
are the only currently confirmed exact basic-skill icon bindings.

## Hunter vitals and combat statistics

The official reference describes:

- **HP**: reduced by monster attacks; restored by the infirmary. A Hunter at
  zero HP dies and requires resurrection at the resurrection sanctuary.
- **Satiety**: consumed while attacking. A hungry Hunter returns to town and can
  recover at the restaurant.
- **Mood**: reduced when attacked and restored at the tavern. The article states
  that other Hunter statistics change with Mood, but the exact formula remains
  unresolved.
- **Stamina**: decreases over time. A tired Hunter returns to town and recovers
  at the inn.
- **ATK**: contributes to damage dealt.
- **DEF**: reduces damage received.
- **CRIT**: chance to deal `1.5x` critical damage. The official article states a
  `50%` equipment-derived maximum.
- **ATK SPD**: controls attack action time. The official article states an
  equipment-derived minimum of `0.25`.
- **Evasion**: chance to avoid a monster attack. The official article states a
  `40%` equipment-derived maximum.

Caps and formulas are official-reference rules until confirmed against package
or runtime evidence for the selected content release.

## Generated stat quality

The official reference states that, when a Hunter arrives:

- Each generated statistic has a `33%` chance to be a high statistic.
- Each generated statistic has a `6%` chance to be the highest statistic.
- High statistics are displayed in blue.
- Highest statistics are displayed in orange.

The wording does not prove whether the `6%` is included within or independent of
the `33%`, nor does it define the roll order, stat ranges, tutorial overrides or
RNG seeding. The server must not implement those missing details by assumption.

## Rarity score

The official reference assigns points per generated statistic:

- Normal statistic: `0` points
- High statistic: `1` point
- Highest statistic: `2` points

Hunter rarity is based on the total score:

| Score | Rarity |
|---:|---|
| Other / below 2 | Normal |
| 2-5 | Rare |
| 6-9 | Superior |
| 10-13 | Heroic |
| 14-27 | Legendary |

The precise generated-stat set contributing to the score must be confirmed
before implementing the rarity calculator.

## Arrival and waiting roster

- Hunters visit the town until the available Hunter slots are filled.
- Waiting Hunters enter in queue order when an active slot becomes available.
- The official article states that the waiting queue is not retained when the
  game is deleted or restored from cloud save.
- Request-Hunter rates are stated to match the corresponding waiting-slot rate,
  except for the separately corrected Random Hunter Request composition below.

## Official rarity rates

These tables transcribe the images in the official Vietnamese article.

### Waiting slot

| Rarity | Rate |
|---|---:|
| Legendary | 0.05% |
| Heroic | 0.55% |
| Superior | 3.00% |
| Rare | 37.00% |
| Normal | 59.40% |

### Advanced Hunter Request

| Rarity | Rate |
|---|---:|
| Legendary | 0.50% |
| Heroic | 3.00% |
| Superior | 17.00% |
| Rare | 30.00% |
| Normal | 49.50% |

### Divine Hunter Request

| Rarity | Rate |
|---|---:|
| Legendary | 2.00% |
| Heroic | 9.00% |
| Superior | 30.00% |
| Rare | 59.00% |

### Superior Hunter Request

| Rarity | Rate |
|---|---:|
| Legendary | 5.00% |
| Heroic | 25.00% |
| Superior | 70.00% |

### Heroic Hunter Request

| Rarity | Rate |
|---|---:|
| Legendary | 15.00% |
| Heroic | 85.00% |

### Legendary Hunter Request

| Rarity | Rate |
|---|---:|
| Legendary | 100.00% |

### Random Hunter Request composition

The official article explicitly corrects the in-game display and marks the
following table as authoritative:

| Selected request table | Rate |
|---|---:|
| Hunter Request | 65.00% |
| Advanced Hunter Request | 31.00% |
| Divine Hunter Request | 4.00% |

The article notes that the displayed values `70% / 22% / 8%` are incorrect.

## Inventory and snapshots

Each Hunter owns independent mutable state:

- material stacks;
- item and consumable stacks;
- gear item instances and equipment slots;
- learned skills and levels;
- traits and growth allocation;
- appearance composition;
- riding-pet assignment;
- base, current and derived statistics.

Definitions are shared content. Hunter snapshots contain identifiers and
mutable values only. Gear must be represented as an item instance because its
quality, enhancement, options, runes, lock state and potential can differ from
another instance of the same definition.

## Server-authoritative commands

At minimum, the server validates and persists:

- recruit, queue, promote and banish Hunter;
- learn, upgrade, equip and reset skill;
- equip, unequip, enhance, upgrade, identify and modify gear;
- add, reserve, consume, sell or transfer Hunter materials/items;
- spend service costs and apply service completion;
- generate Hunter statistics and rarity using a versioned RNG rule set.

The browser submits intent only. It must never submit trusted rarity, rolled
statistics, costs, rewards, item options or skill outcomes.

## Unresolved implementation rules

- Exact Hunter stat ranges and RNG algorithm.
- Whether highest-stat probability is nested within the high-stat probability.
- Which generated statistics contribute to the 0-27 rarity score.
- Tutorial, pity, advertisement and paid-request overrides.
- Derived-stat calculation and rounding order.
- Skill combat handlers, stacking rules and most icon/VFX bindings.
- Exact local/cloud persistence treatment of the waiting queue.

These rules must remain explicit unresolved states until package evidence,
controlled runtime capture, or a versioned product decision resolves them.
