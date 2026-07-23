# Game Analysis Guide

## Purpose And Boundary

The analysis phase reconstructs observable behavior, content relationships, and presentation from legally available local artifacts and controlled gameplay observation. It does not connect to, impersonate, attack, or bypass the original production service. Do not collect other users' data or reuse credentials and secrets.

The output is evidence and specifications for an independent implementation, not a claim that decompiled output is maintainable source code.

## Evidence Inventory

Record every source in an immutable evidence manifest:

| Evidence | Capture |
| --- | --- |
| XAPK/APK and splits | original filename, SHA-256, package/version, extraction tool/version |
| Unity assets | bundle/path, object type, original ID, checksum |
| IL2CPP metadata | assembly/type/member names, source offset, confidence |
| Native strings | binary, offset, encoding, surrounding symbols |
| Serialized content | source object, field layout, inferred type, confidence |
| Runtime observation | build version, device, scenario, timestamp, video/screenshots |
| Network observation | only traffic from an owned test account; redact tokens and identifiers |

Never modify evidence in place. Derived files live separately and record the source checksum and transformation command.

## Analysis Workflow

### 1. Inventory

- Enumerate APK splits, Unity version, architectures, assemblies, native libraries, scenes, asset bundles, resources, localization, audio, fonts, shaders, and third-party SDKs.
- Classify artifacts as game-owned, engine, SDK, platform integration, or unknown.
- Create coverage counts so omissions are visible.

### 2. Build a domain catalog

Map recovered names and serialized objects into domains: account, village, hunter, combat, monster, item, equipment, building, quest, dungeon, raid, PvP, guild, shop, ads, localization, audio, and tutorial.

For every feature, capture:

- entry and exit conditions;
- state and state transitions;
- commands available to the player;
- formulas, tables, caps, timers, RNG points, and server-time dependencies;
- assets, animations, audio, text, and UI screens used;
- persistence behavior and known failure cases;
- evidence references and confidence: confirmed, strongly inferred, tentative, or unknown.

### 3. Reconstruct data schemas

- Extract field names/types/defaults from serialized Unity objects and metadata.
- Correlate IDs across prefabs, sprites, animation clips, localization, and `Admin*Data` records.
- Normalize into canonical content definitions without discarding raw values.
- Write validators for duplicate IDs, dangling references, invalid ranges, missing localization, and missing assets.
- Preserve unknown fields verbatim in raw evidence until understood.

### 4. Reconstruct behavior

Use a behavior matrix rather than guessing method bodies:

| Inputs | Initial state | Observed result | Repetitions | Candidate rule | Confidence |
| --- | --- | --- | --- | --- | --- |

Vary one input at a time. Repeat RNG-dependent experiments sufficiently. Test boundaries such as zero, cap, cap+1, death, disconnect, clock change, full inventory, and duplicate command.

Prefer golden traces: timestamped command/state/event sequences that the new deterministic simulator can replay.

### 5. Separate client and server responsibility

Classify each original behavior as presentation, local prediction, authoritative rule, persistence, or external integration. In the rebuild, valuable outcomes always move to the authoritative server even if the original client computed them.

### 6. Produce a feature dossier

Each dossier contains:

1. player-facing description;
2. state model and sequence diagram;
3. content schema and assets;
4. authoritative rules and equations;
5. protocol commands/events;
6. persistence and transactions;
7. edge cases and abuse cases;
8. golden tests and visual references;
9. known gaps and confidence.

## Recommended Analysis Order

1. Boot, loading, scene/map, camera, localization, and save identity.
2. One hunter, one monster, movement, targeting, attack, damage, death, and revival.
3. Drop, inventory, equipment, currency, and progression transaction.
4. Building production and village automation.
5. Quest/tutorial flow and unlock gates.
6. Dungeon/raid variants, bosses, and timed content.
7. Shop, ads, purchase verification, and mail.
8. Guild, rank, PvP, events, and operations.

This order supplies complete vertical slices rather than isolated reverse-engineering notes.

## Accuracy Rules

- Never convert an inference into a fact without evidence.
- Preserve exact numeric precision and rounding order; label approximations.
- Distinguish visual timing from authoritative timing.
- Use server time in the rebuild for cooldowns and offline progress.
- Do not reproduce bugs accidentally; record them and decide intentionally whether compatibility requires them.
- Unknown behavior must fail safely and remain visible in the gap register.

## Deliverables

- evidence manifest and checksums;
- asset/content catalogs and dependency graph;
- domain and feature dossiers;
- formula/rate workbook or canonical content files;
- golden simulation traces;
- UI/animation/audio reference captures;
- compatibility matrix and gap register;
- risk register for legal, technical, security, and performance concerns.

## Completion Criteria

A feature is analysis-complete when its player-visible flow, authoritative state transitions, data dependencies, asset dependencies, edge cases, and validation examples are documented well enough for a different engineer to implement and test without inspecting the original package again.
