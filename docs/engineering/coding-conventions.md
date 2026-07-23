# Coding Conventions

## Shared Conventions

- Use English for identifiers, source comments, protocol fields, logs, and documentation.
- Prefer domain names over technical shorthand: `reward_ledger`, not `rl`.
- Keep functions small around one level of abstraction; extract policy from orchestration.
- Comments explain constraints, invariants, provenance, or non-obvious trade-offs, not syntax.
- Use UTC at boundaries and explicit duration/time types internally.
- Use integer or fixed-point types for currency and deterministic game math; never binary floating point for valuable outcomes.
- Return typed errors with stable categories; do not parse error strings.
- Inject clocks, RNG, IDs, and external providers for deterministic tests.
- Do not log secrets, access tokens, receipts, personal data, or full untrusted payloads.

## Rust Server

### Structure

Organize by domain first, then layer:

```text
domain/<name>/model.rs
domain/<name>/commands.rs
domain/<name>/service.rs
domain/<name>/repository.rs
transport/
infrastructure/
```

Domain modules do not depend on HTTP/WebSocket or database row types. Infrastructure implements domain traits. Keep public surfaces minimal with `pub(crate)` by default.

### Style And Safety

- `rustfmt` and `clippy` warnings are enforced in CI.
- Production code forbids `unwrap`, `expect`, unchecked indexing, and ignored `Result` except where an invariant is statically obvious and documented.
- Avoid `unsafe`; any necessary block requires a safety comment, isolated module, focused tests, and review.
- Newtypes represent player IDs, item IDs, currency amounts, ticks, and content versions.
- Use exhaustive enums for states and commands; unknown wire values fail at the transport boundary.
- Do not hold blocking locks or database transactions across `.await`.
- Place explicit limits on channels, queues, task creation, and request bodies.

### Concurrency And Simulation

- Each active zone has one logical owner to avoid shared mutable simulation state.
- Fixed-step simulation receives commands through bounded queues.
- Persistence effects leave the simulation as typed domain events.
- Ordering and retry semantics are documented for every consumer.
- Test deterministic output across repeated runs and server restarts.

## TypeScript Client

### Type Discipline

- Enable strict TypeScript, including unchecked indexed access and exact optional properties where supported.
- Avoid `any`; use `unknown` at trust boundaries and validate before narrowing.
- Prefer discriminated unions for protocol and UI/game states.
- Generated protocol/content types are not manually edited.
- Use branded types for entity IDs, ticks, asset IDs, and content versions where confusion is possible.

### Client Boundaries

- PixiJS owns the world render loop and display objects.
- The UI framework owns menus/HUD but consumes stable projections; it does not subscribe every entity to reactive rendering.
- Network decoding, authoritative state, interpolation, presentation state, and UI state are separate modules.
- Game logic that changes outcomes does not live in components or Pixi display objects.
- Explicitly destroy display objects, textures, listeners, workers, and subscriptions when ownership ends.

### Performance

- No per-frame allocation in hot entity loops unless measured and accepted.
- Pool short-lived effects and reuse typed buffers.
- Batch sprites by atlas/material and cull outside interest/view.
- Profile before micro-optimizing; preserve benchmark captures for budget changes.
- Avoid JSON for frequent state snapshots; use generated binary codecs.

## SQL And Persistence

- Use `snake_case`, singular table names if repository convention chooses it consistently, and explicit constraints.
- Every foreign key used for integrity is declared, not implied only in code.
- Valuable mutations include actor, reason, correlation/idempotency key, before/after or signed delta, and timestamp.
- Queries are parameterized; dynamic identifiers use allowlists.
- Review query plans for hot paths and include realistic-volume integration tests.

## API And Schema Naming

- Commands use imperative verbs: `equip_item`, `claim_quest_reward`.
- Events use past tense: `item_equipped`, `quest_reward_claimed`.
- State messages use nouns: `zone_snapshot`, `inventory_state`.
- Units are explicit in names where ambiguity exists: `cooldown_ms`, `radius_px`, `tick_index`.
- Version at the envelope/content-release level rather than suffixing every field.

## Tests

Name tests as behavior: `duplicate_reward_command_returns_original_result`. Arrange/act/assert should be clear without decorative comments. A bug fix first captures the failing case. Golden files are reviewed, versioned, deterministic, and updated only with an explained behavior change.
