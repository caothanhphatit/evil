# Evil Hunter Migration Knowledge Transfer

Last updated: 2026-07-26  
Baseline commit: `41fae603e23f298cafb9a18b6dd45f1f491cb707`

## Mission

This repository is an educational rebuild and migration study of the supplied
Evil Hunter Tycoon `1.411` package. The implementation must be driven by
recoverable source evidence, serialized Unity data, controlled runtime captures,
and user-provided screenshots. Never fill an evidence gap with a plausible
mapping or silent fallback.

## Repository map

- `apps/web/`: Vite/TypeScript game client and town/building/Hunter UI.
- `apps/server/`: Rust server, session handling, simulation, products, Hunter
  roster, trading post, and building services.
- `infra/db/migrations/`: relational schema and seed migrations through `0016`.
- `packages/content/`: generated schemas and normalized runtime catalogs.
- `game-assets/normalized/`: normalized assets used by the rebuild.
- `reverse-engineering/evidence/`: machine-readable extracted evidence.
- `docs/migration/`: evidence boundaries, UI audits, and migration reports.
- `tools/`: deterministic extractors, validators, generators, and runtime tools.

Large package/native inputs are already tracked with Git LFS. After cloning:

```sh
git lfs install
git lfs pull
```

Do not commit `target/`, `node_modules/`, `dist/`, Python caches, credentials, or
new raw study inputs unless their inclusion and LFS treatment are deliberate.

## Current implementation

- Town projection, camera/depth handling, building placement, normalized base
  building versus skin data, and visible-world packaging are implemented.
- Building registries, conditions, product stock, crafting/service routes,
  trading post, blacksmith/gear shop, potion route separation, and related DB
  migrations exist. UI fidelity is still an iterative migration, not proof that
  every building matches the original behavior.
- A demo Hunter roster and modular Hunter appearance projection exist across DB,
  server, and web layers.
- Hunter Info has Status, Skills, Materials/Inventory, Growth, and Riding Pet
  projections. Missing per-Hunter evidence is intentionally shown as unavailable
  rather than synthesized.

The exact original Hunter Detail tab dispatch is:

1. Status
2. Skills
3. Inventory/Materials
4. Growth
5. Riding Pet

If the client order differs, correct it only while preserving the evidence
boundary documented below.

## Strong Hunter evidence

Read these before touching Hunter generation or Hunter Info:

- `docs/migration/hunter-info-data-audit-v1.md`
- `docs/migration/hunter-detail-scene-object-graph-v1.md`
- `docs/migration/hunter-info-binding-evidence-v1.md`
- `docs/migration/android-save-runtime-audit-v1.md`
- `docs/migration/hunter-generation-flow-evidence-v1.md`
- `reverse-engineering/evidence/hunter-info-serialized-bindings-v1.json`
- `reverse-engineering/evidence/hunter-save-serialization-v1.json`

Confirmed model boundaries:

- `UserData` is the large aggregate: 527 fields.
- `HunterData` is the primary Hunter snapshot candidate: 109 fields.
- `HunterLookData` is the appearance projection candidate: 11 fields.
- `SaveData` is a small wrapper and is not the complete player snapshot.
- Equipment slots: Gloves, Helmet, Necklace, Boots, Ring, Weapon, Armor, Belt.
- Scene assets expose 50 skill icons, 15 growth assets, 21 pet portraits, 21 pet
  actor thumbnails, 3 pet-skill icons, 6 pet-trait icons, and 69 job-trait icons.
- Only Fury to `skill_h1_01` and War Cry to `skill_h1_02` are currently confirmed
  exact skill bindings. Do not bind the other skill icons by array index.

Still unresolved without runtime evidence:

- The remaining skill row-to-icon and per-Hunter learned-state bindings.
- Runtime Inventory/Materials rows and quantities.
- Growth node IDs, costs, effects, and learned state.
- Riding Pet ownership, skill, trait, and gear values per Hunter.
- The serializer call graph and exact local save encoding.

## Runtime capture on Mac

Use a physical, authorized Android ARM64 test device where possible. The package
is ARM64-only; emulating it on a small x86 host is unnecessarily expensive.

- Guide: `docs/migration/hunter-info-runtime-capture-macos.md`
- Frida script: `tools/runtime/hunter-info-runtime-dump.js`

The current dumper waits for `libil2cpp.so`, attaches to the IL2CPP domain, and
uses exported reflection APIs to emit fields, types, offsets, methods, and tokens
for `HunterData`, `HunterLookData`, `UserData`, `SaveData`, and
`HunterDetailPop`. It deliberately does not read arbitrary managed objects or
claim save-value bindings.

When returning captured output to another agent, include package version,
package ID, device ABI, Frida client/server versions, UTC timestamp, exact user
action, and both before/after captures. A field-name resemblance is not proof of
a UI binding.

## Development commands

```sh
pnpm install
pnpm test:web
pnpm build:web
cargo test --manifest-path apps/server/Cargo.toml
pnpm test:assets
pnpm building:validate
python3 -m unittest tools.tests.test_scene_evidence
```

Run validation in proportion to the change. Lightweight mining-only changes
should at least pass `git diff --check`, relevant Python compilation/tests, and
`node --check` for changed JavaScript. Do not regenerate large catalogs unless
the source evidence or generator changed.

## Handoff checklist

Before reporting work complete:

1. State exactly what evidence supports each new mapping or behavior.
2. List unresolved fields explicitly; do not hide them behind default values.
3. Keep DB migrations, server projections, protocol, and FE models consistent.
4. Record newly generated evidence and the deterministic command that created it.
5. Report tests actually run and any tests skipped.
6. Check `git status`, large-file handling, and secrets before committing.

