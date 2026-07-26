# Hunter Data Model Review

## Decision

PostgreSQL is the durable source of truth for accounts, towns, Hunters, inventory and shop stock. Redis is limited to session lookup, player leases, fencing and other short-lived coordination. Core Hunter state is relational; JSONB is reserved for optional, versioned extensions that do not participate in integrity rules or economy queries.

## Problems in schema 0013

Migration 0013 correctly modeled the eight active slots, FIFO waiting queue and idempotent banishment commands, but a Hunter row only contained wallet and service gauges. It could not preserve identity, class, rarity, progression, combat stats, presentation state, traits or skills. Consequently, a reconnect could restore roster membership while losing the data required by the Hunter HUD and actor renderer.

## Schema 0014

The model separates immutable content definitions from player-owned state:

- `hunter_content_release` pins all definitions to an evidence/version boundary.
- `hunter_class_definition`, `hunter_rarity_definition`, `hunter_trait_definition` and `hunter_skill_definition` hold reusable content.
- `player_profile` records account kind and deterministic seed metadata.
- `player_hunter` owns identity, progression, gauges, combat stats and current presentation state.
- `player_hunter_trait` and `player_hunter_skill` own unlock/equip state and reference release-pinned definitions.

This prevents base definitions from being copied into every account, keeps foreign-key validation available, and permits content releases to coexist without silently changing existing Hunters.

## Runtime write strategy

The server loads one aggregate when a player lease is acquired and simulates frequent Hunter actions in memory. It persists at bounded checkpoints and meaningful economy boundaries rather than issuing one SQL write per animation tick. A state revision plus lease fence rejects stale writers. Economy mutations that must be durable together remain in one PostgreSQL transaction.

Redis must not become the only copy of Hunter inventory, farm rewards, shop stock or progression. Losing Redis may disconnect a session, but it must not lose durable game state.

## Demo fixture

Migration 0014 seeds the public disposable profile `Hunter Lab` with exactly eight active Hunters. The seed is deterministic (`hunter-lab:20260724`, version 1) and intentionally covers all five current visual families and rarity labels, with distinct traits and recovered skill-animation bindings.

The fixture does not claim decoded original balance. H1-H5 class labels are strongly inferred, rarity mechanics remain unresolved, trait effects are unresolved, and skill rows currently prove visual/animation bindings only.
