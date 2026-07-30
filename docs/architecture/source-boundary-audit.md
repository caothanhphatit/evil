# Source Boundary Audit

Date: 2026-07-30

## Decision

The repository follows the intended authoritative-server topology, but several
implementation files still cross too many internal responsibilities. Service
extraction is not justified; the correct direction remains a modular monolith
with enforceable dependency direction and smaller bounded-context modules.

## Current findings

| Priority | Area | Finding | Target boundary |
| --- | --- | --- | --- |
| P0 | Authority | No browser-to-persistence or simulation-to-API dependency was found. Server authority remains intact. | Preserve with automated dependency checks. |
| P1 | Web composition | `apps/web/src/main.ts` is now a one-line composition root. `game-application.ts` coordinates the client and controllers; shell, entry, world, Hunter, building, trade, and combat concerns are separate modules. | Keep composition in `game-application.ts`; move new feature behavior into the owning controller rather than growing the root. |
| P2 | World simulation | The former 3,653-line world module is now a 469-line state/config facade over fixed-tick runtime, control, skills, navigation, combat, rewards, spawn, presentation, and colocated scenario tests. The tick runtime remains slightly above its 700-line target. | Preserve tick ordering; split only when a coherent AI/runtime policy can move without scattering mutable world access. |
| P2 | Persistence | The former 2,658-line module is now a 108-line repository port/facade over memory and PostgreSQL adapters plus aggregate, building, and Hunter codecs. The Hunter codec remains above its 700-line target because it maps the full captured nullable runtime schema. | Keep transaction orchestration in the PostgreSQL adapter and split the Hunter codec only along stable runtime aggregate boundaries. |
| P1 | Buildings | `buildings/mod.rs` combines identifiers, catalog model, validation, town aggregate, repository traits, and PostgreSQL SQL. | Domain model separated from repository port and PostgreSQL adapter. |
| P1 | Content | `building_registry.rs` combines serialized schema, evidence validation, release verification, and runtime content mapping. | Schema, validator, loader, and runtime mapper modules. |
| P2 | Original flow | The facade is reduced from 9,629 to under 2,000 lines, but durable contracts and restore normalization still share the orchestration module. | Move contracts and restore/upgrade policies into dedicated modules; facade target 500-700 lines. |
| P2 | Test ownership | Large test modules make ownership and failure diagnosis slower. | Colocate scenario tests with each domain module and retain only cross-domain contract tests centrally. |

## Dependency direction

```text
Web entry/composition -> UI controllers -> game presentation -> generated contracts
                                |                    |
                                +------ client intents+

API/gateway -> application facade -> domain policies -> immutable content
                       |                 |
                       +-> repository ports <- PostgreSQL/memory adapters
```

- Game/rendering modules must not import UI controllers.
- Simulation/domain modules must not import API, coordination, or persistence adapters.
- Generated protocol contracts are boundary DTOs, not domain models.
- PostgreSQL and Redis implementations depend on ports/domain types; domain code
  does not depend on those implementations.

## Fitness function

`pnpm architecture:validate` enforces dependency direction and ratchet ceilings
for current hotspot files. A ceiling prevents new growth while the lower target
documents the intended end state. Lower ceilings whenever a module is split;
never raise one merely to merge a feature.

The fitness function also ratchets wildcard parent imports in the refactored
server packages. New modules must declare explicit dependencies; existing
`use super::*` debt is removed incrementally and the ceiling lowered with each
cleanup.

## Refactor sequence

1. Extract web boot/session and building-screen controllers from `main.ts`.
2. Split building domain model from SQL adapters.
3. Split registry schema/validation/runtime mapping.
4. Move remaining original-flow contracts and restore policies, then lower its
   architecture ceiling to 700 lines.

Each step must preserve generated protocol compatibility, deterministic tests,
economy idempotency, and the Docker development contract.

## Current web size checkpoints

- `apps/web/src/main.ts`: composition-only entrypoint.
- `apps/web/src/app/game-application.ts`: below the 600-line ceiling.
- `apps/web/src/game/visible-world.ts`: below the 600-line ceiling; actor sizing/facing lives in `game/actor-presentation.ts`.
- `pnpm architecture:validate`: passing with no debt targets.
