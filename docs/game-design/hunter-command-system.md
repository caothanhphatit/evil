# Hunter Lifecycle and Command System

Status: working specification
Package baseline: Evil Hunter Tycoon `1.411`
Last reviewed: 2026-07-26

## Purpose

Hunters are autonomous town residents rather than directly controlled player
avatars. The player influences their behavior through conversation commands,
town services, gifts, equipment and destination assignments. The authoritative
server owns command validation, movement state, purchases, service completion,
inventory mutations and lifecycle transitions.

This document incorporates user-provided gameplay descriptions and screenshots.
Labels visible in screenshots are treated as **User-raw visual evidence** until
the corresponding command identifiers and runtime handlers are recovered.

## Primary lifecycle

```text
Arrive at town
  -> enter active roster or waiting queue
  -> prepare in town
  -> travel to an assigned hunting ground
  -> fight monsters
  -> collect gold, materials and other loot
  -> return to town when instructed or when needs require it
  -> sell carried materials
  -> buy equipment or use town services
  -> recover and prepare
  -> repeat
```

Common reasons for returning to town include low HP, hunger, low Mood, fatigue,
full or operationally constrained inventory, explicit movement orders, service
needs, and death. Exact thresholds and decision priority remain unresolved.

## Hunter-owned state

Each Hunter owns an independent snapshot containing at least:

- base job and current advancement path;
- learned skills and skill levels;
- traits and growth/secret-technique allocations;
- personal statistics and current vitals;
- personal gold;
- carried materials and consumable items;
- gear item instances and equipped slots;
- fashion/costume and appearance composition;
- assigned pets and riding pets;
- current activity, destination and service state.

The player can provide equipment, fashion items, pets and other supported gifts
to a Hunter. Ownership transfer must be recorded as an authoritative transaction
between the player/town inventory and the Hunter inventory; the browser cannot
declare the resulting ownership or statistics.

## Death and resurrection

- A Hunter dies when HP reaches zero.
- A dead Hunter stops normal farming and town-service behavior.
- The Hunter can be resurrected through the resurrection feature/building.
- Resurrection cost, duration, location, automatic behavior and penalties must
  be versioned separately until recovered from package or runtime evidence.
- Death does not imply deletion, banishment or loss of Hunter ownership.

## Conversation command surface

The screenshots show five primary command categories:

1. Items
2. Move
3. Learn
4. Daily Life
5. Management

The command panel is contextual. A visible red `X` indicates that the command is
currently unavailable or its conditions are unmet; it does not prove that the
feature is absent.

## Items

The Items category controls the Hunter's personal economy and equipment.

| Command | Intended behavior | Authority and validation |
|---|---|---|
| Buy Equipment | Ask the Hunter to purchase eligible armor/accessories | Validate shop, stock, job/level restrictions, Hunter gold and resulting item instance |
| Buy Weapon | Ask the Hunter to purchase an eligible weapon | Validate weapon/job compatibility, stock, price and Hunter gold |
| Manage Equipment | Open or execute equipment management | Validate Hunter ownership, slot compatibility and item state |
| Equipment Storage / Preservation | Move or protect supported gear through the related town feature | Exact original semantics and cost remain unresolved |
| Sell Materials | Sell carried materials back to the town/market | Atomically remove Hunter material quantities and credit the correct wallet |

The screenshot exposes visible actions corresponding to buying equipment,
equipment management, an equipment-preservation/storage feature and selling
materials. The user description also separates weapon purchasing; the final
menu layout and unlock condition require further evidence.

## Move

The Move category displays available hunting destinations and sends the Hunter
to the selected area.

Visible screenshot destinations include:

- Colony;
- Land of the Dead;
- Demon World;
- Titan area or a currently unavailable destination;
- return/back action.

The displayed list is expected to depend on progression, difficulty, unlocked
regions and the Hunter's eligibility. A move command records player intent; the
server validates the destination and transitions the Hunter from town behavior
to travel/farming behavior.

## Learn

The Learn category contains:

| Command | Intended behavior |
|---|---|
| Skills | Learn or upgrade job-bound skills after level and resource requirements are met |
| Traits | Learn or upgrade eligible job traits after their conditions are met |
| Secret Techniques | Allocate or learn growth/secret-technique properties after unlock conditions are met |

The screenshot shows all three actions unavailable for the selected Hunter. The
server must calculate availability from the Hunter's level, job path, previous
nodes, currencies/resources and feature unlocks. The client only renders the
availability projection and sends the selected intent.

## Daily Life

The Daily Life category directs the Hunter to town services:

| Command | Need restored | Expected service domain |
|---|---|---|
| Eat | Satiety | Restaurant |
| Have Fun | Mood | Tavern |
| Rest | Stamina | Inn |
| Heal | HP | Infirmary |

Service use is not an immediate trusted client-side refill. The server validates
the service building, product, capacity, price, Hunter state and duration, then
applies the completion result authoritatively.

## Management

The Management category contains:

| Command | Intended behavior |
|---|---|
| Donate | Provide personal gold to the Hunter through the supported banking/donation flow |
| Banish | Permanently remove the Hunter from the player's roster after confirmation |

The user description identifies bank purchasing/support as another way to fund
Hunters. The exact distinction between bank purchase, donation and wallet
transfer must be resolved before implementing costs or monetization behavior.

Banishment must be a confirmed, idempotent server command. It must define what
happens to equipped gear, carried materials, pets, queue ordering and historical
records before it is enabled outside disposable fixtures.

## Autonomous behavior priorities

The server simulation should eventually express Hunter decisions as a state
machine rather than client timers:

```text
Dead
  -> Awaiting resurrection

In town
  -> Using service
  -> Shopping/equipping
  -> Selling materials
  -> Learning/progressing
  -> Preparing to travel

Outside town
  -> Travelling
  -> Searching for target
  -> Fighting
  -> Collecting loot
  -> Returning to town
```

Decision priority, thresholds, travel duration, sale selection and purchasing
AI remain unresolved. Until recovered, the rebuild should implement explicit,
versioned product decisions instead of presenting guesses as original behavior.

## Command contract guidance

Commands should use an idempotency key and include intent only. Examples:

```text
assign_hunting_ground(hunter_id, destination_id)
request_hunter_purchase(hunter_id, shop_id, product_id)
request_hunter_service(hunter_id, building_id, product_id)
learn_hunter_skill(hunter_id, skill_id)
allocate_hunter_growth(hunter_id, property_id)
sell_hunter_material(hunter_id, material_id, quantity)
donate_to_hunter(hunter_id, amount)
banish_hunter(hunter_id)
```

Every command validates ownership, current state, content release, unlock
conditions, balance/inventory, concurrency revision and replay/idempotency before
committing a transaction.

## Unresolved details

- Exact AI decision priority and return thresholds.
- Exact loot pickup, carrying capacity and automatic sale rules.
- Whether Hunters independently choose purchases without an explicit command.
- Equipment-preservation command semantics and costs.
- Complete destination list and unlock conditions.
- Skill, trait and secret-technique learning currencies and transaction order.
- Donation/bank conversion rules and monetization boundaries.
- Resurrection cost, duration and penalties.
- Banishment treatment of owned items, pets and history.
