# Original Flow Content Contract (Phase C)

## Purpose

Phase C defines the canonical, evidence-aware content boundary for the original `boot -> village -> hunter roster -> field` flow. It does not implement the browser UI and it does not promote the existing Training Ground fixture into legacy content.

The generated release is `game-assets/manifests/releases/original-flow-v1.json`. Its source selection is `game-assets/manifests/original-flow-v1.selection.json`, and its structural contract is `packages/content/original-flow-v1.schema.json`.

## Evidence semantics

Every claim uses one of four confidence values:

- `confirmed`: directly resolves to the Unity inventory, asset index, or extracted scene hierarchy.
- `strongly-inferred`: multiple structural signals support the role, but a runtime or serialized reference is still missing.
- `tentative`: a filename or single weak signal identifies only a candidate.
- `unknown`: current evidence cannot supply the value.

Asset identity and screen binding are deliberately separate. For example, `background_01` is a confirmed source Sprite, but its role as the initial village background is only tentative. Likewise, the Hunter Spine bundle is confirmed, while starter skin composition and starter stats remain unknown.

## Canonical flow boundary

| Order | Flow | What is locked | What blocks runtime compatibility |
| ---: | --- | --- | --- |
| 1 | Boot | Intro/loading asset candidates and account/localization domain surface | Exact state order, localized copy, fonts, and reference trace |
| 2 | Village | `UICanvas`, `MainCanvas`, `WorldCanvas`, `MapManager`, `BuildGroup`, `BottomView` | Background binding, camera/map bounds, building anchors/interactions |
| 3 | Hunter roster | Hunter manager/group/border and atomic Hunter Spine source bundle | Starter composition/stats and resolved roster UI bindings |
| 4 | Field | World, Hunter, and Evil scene groups plus HP-frame candidate | First map/monster, combat values, drops, revival, audio/effect timing |

The release gate remains `runnable: false` while any required binding is unresolved. A basic build may consume the manifest for development diagnostics, but product code must not present it as a completed original-game flow.

## Server-authoritative visual projection

Protocol v5 exposes a bounded `snapshot.world` projection so the browser can render visible actors instead of inventing them locally. The server owns the active world mode, visual tick, entity list, normalized positions, facing, animation clip, and current selection. A 5 TPS `world_update` stream advances non-economic roaming while the active world is visible. The browser may send only `enter_field`, `navigate_back`, and `select_entity` intents.

This projection is explicitly scoped as `visual_roaming_only`. It uses source-confirmed Spine families and source-confirmed non-combat clips: Hunter `hunter_walk`, Npc `npc_stay`, and the passive Goldblin candidate `walk`. Goldblin is not claimed as the original first-field monster. Actor placement remains `unknown` and unresolved; normalized coordinates are temporary presentation anchors rather than recovered legacy spawn points. The visual tick and selection are session-only, while only boot completion and the current screen are durable.

The visual field projection does not authorize combat, HP, damage, drops, rewards, rates, or economy state. Those field gameplay bindings remain blocked until independently evidenced.

## Numeric and gameplay rules

The contract contains no guessed costs, stats, rates, damage, cadence, drop chances, gameplay coordinates, timers, or progression gates. Numeric gameplay values can be added only when marked `legacy-verified` or `observed` and linked to the evidence that supplied them. The protocol's normalized visual anchors are explicitly unresolved presentation metadata and cannot be consumed by simulation or collision logic. Unity `path_id` values are evidence identifiers, never gameplay IDs.

## Generation and validation

Run:

```bash
pnpm assets:generate:original-flow
pnpm assets:validate:original-flow
```

Generation resolves every selected asset against both `game-assets/asset-index.json` and the Unity inventory, resolves every scene object by path ID and exact name, pins source hashes, preserves flow order, and materializes all unknown bindings as `null`. Validation fails if the committed manifest is stale or any evidence reference no longer resolves.

## Promotion rules

A binding may move from `unresolved` to `resolved` only after the selection and generator support a concrete evidence record. Field actors require two independent correlation signals. Runtime promotion also requires all assets in an atomic bundle, especially Spine skeleton/atlas/pages. Missing evidence must fail the release gate; it must not trigger a fallback actor, fixture rate, or generic screen.
