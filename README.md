# Evil Hunter Web Rebuild

Clean-room, server-authoritative web reconstruction workspace for academic study of Evil Hunter Tycoon 1.411.

The repository migrates the original game in ordered A-Z phases. The current primary runtime is an evidence-aware boot -> village -> hunter-roster flow using original assets, generated protocol v4 intents, PostgreSQL/Redis session ownership, and explicit blockers for legacy bindings that are not yet proven. The former hunter/monster combat is retained only as an isolated technical fixture and is not presented as the original UX.

## Stack

- Web client: TypeScript, Vite, PixiJS, Vitest
- Authoritative game server: Rust, Tokio, Axum, deterministic 10 TPS simulation
- Persistence: PostgreSQL 17
- Ephemeral state and coordination: Redis 7
- Local runtime: Docker Compose

## Run

```bash
cp .env.example .env
pnpm assets:index
pnpm assets:catalog:full
docker compose up --build
```

Open `http://localhost:5173`. Server health is available at `http://localhost:8080/health`; readiness also checks PostgreSQL and Redis at `/ready`.

PostgreSQL and Redis are exposed on host ports `15432` and `16379` to avoid colliding with common local installations; containers still use their standard internal ports.

The browser sends versioned screen/navigation intents only. The server resolves the session identity from an HttpOnly cookie, owns screen state and unresolved-binding decisions, and never grants guessed progression/economy outcomes. Full exported assets are mounted read-only in Docker; behavior binding remains a separate evidence gate.

## Repository layout

```text
apps/web/                  PixiJS client
apps/server/               Rust authoritative simulation and API
infra/db/migrations/       PostgreSQL schema
game-assets/source/        Byte-for-byte legacy Unity/XAPK study input
game-assets/extracted/     Web-consumable exports and joined Unity files
reverse-engineering/       Reports, class catalog, metadata and native evidence
docs/                      Architecture, ADRs, rules and migration guides
tools/                     Asset indexing and verification
```

## Asset completeness

- The original XAPK is preserved locally.
- All 415 Unity asset files are copied byte-for-byte and verified by `make verify-assets`.
- The full derivative catalog indexes 9,359 files: 8,980 sprites, 152 textures, 116 audio clips, 106 text assets, two fonts, and three metadata inventories.
- Raw Unity scene, prefab, animation clip/controller and Addressables data remain the source of truth until converted by the migration pipeline.

Run:

```bash
make verify-assets
make asset-index
make full-asset-catalog
pnpm assets:validate:original-flow
```

`game-assets/asset-index.json` records file size and SHA-256 for every exported web asset. `/full-assets/manifest.json` selects a checksum-pinned runtime catalog; see `docs/assets/full-export-runtime.md`. Catalog presence does not mean every asset is already behavior-bound.

## Working model

The client sends intent (`move`, `select_target`, `use_skill`) and never sends authoritative damage, rewards or inventory balances. The server validates commands, advances deterministic simulation, and emits snapshots. Rendering, interpolation, effects and audio remain client-side.

Start with [the documentation index](docs/README.md), then read the software architecture and vertical migration plan before adding a feature.

## Legal boundary

This workspace is for private academic analysis. Original assets remain third-party copyrighted material. Do not publish, redistribute or commercialize them without permission. New code should be a clean-room implementation and must not connect to the original production service.
