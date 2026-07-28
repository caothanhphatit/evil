# Hunter Flow BE v1

This is an explicit `web-rebuild-v1-fixture` slice, not a claim about the
original balance or RNG.

## Authoritative flow

- `assign_hunter_hunt` accepts only the pinned fixture zone `migration-zone-1`.
- Hunt progress advances from server-owned village ticks; the client cannot
  choose a progress amount.
- Ten server ticks produce one `material:1` fixture loot stack and move the
  Hunter to `returning`.
- `return_hunter_hunt` returns the Hunter to `idle` while retaining loot.
- `sell_hunter_loot` atomically debits town gold at the fixture unit price,
  credits the Hunter wallet, transfers the material into town stock, records a
  durable settlement, and clears the loot.
- `revive_hunter` restores a dead Hunter to max HP and idle state.
- `learn_hunter_skill` accepts all ten packaged basic-skill definitions, with
  exactly two definitions assigned to each base job. Study costs and
  progression gates remain unresolved; only the two H1 icon bindings are
  independently confirmed.
- `use_hunter_skill` accepts a learned basic skill and an optional
  monster target. The server validates ownership, job, target availability,
  range, and readiness, then emits the authoritative class attack sequence.
  Cooldown durations use the exact packaged base values
  (6, 8, 15, or 16 seconds). Exact effect formulas and skill-specific
  animation/effect bindings remain unresolved, so activation does not invent
  damage, buffs, healing, or debuffs; the client only requests intent and
  renders the resulting authoritative presentation.

Every value-changing command is server-side and command-idempotent. Reusing a
command ID with a different Hunter/action fingerprint is rejected.

## Persistence

Migration `0018_hunter_flow_v1.sql` adds JSONB `hunt_state` to
`player_hunter` and a durable `player_hunter_action_command` table. Hunt state,
loot, and idempotency keys therefore survive normalized PostgreSQL reloads.
