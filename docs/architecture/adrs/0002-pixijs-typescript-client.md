# ADR-0002: PixiJS TypeScript Client

- Status: Accepted
- Date: 2026-07-22

## Context

The game is primarily 2D with many animated sprites, effects, maps, and management UI. DOM rendering of the world would create excessive layout/reactivity cost, while a full game framework can obscure performance-sensitive ownership.

## Decision

Use strict TypeScript with PixiJS for world rendering. A lightweight component framework may render menus and HUD projections, but PixiJS exclusively owns per-frame entities and effects. Rendering, authoritative state, interpolation, and UI state are separate layers.

## Consequences

- Direct control over batching, atlases, culling, pooling, memory, and frame pacing.
- Existing Unity assets require a conversion pipeline and canonical animation/state format.
- More engine infrastructure must be written than with Phaser, but unnecessary gameplay abstractions are avoided.
- Browser feature and device-tier fallbacks must be explicit and tested.

## Rejected Alternatives

- DOM/canvas components per entity: rejected for frame-time and allocation risk.
- Phaser: viable for rapid prototypes, but less desirable for the long-term custom server-driven architecture.
- Unity WebGL: rejected due to payload/memory constraints and limited benefit when game logic is being independently rebuilt.
