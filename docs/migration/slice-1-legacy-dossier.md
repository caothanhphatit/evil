# Slice 1 Legacy Dossier

## Purpose

This dossier defines the evidence that may be used for the first vertical migration slice. It separates verified legacy facts from implementation choices. The machine-readable companion is `reverse-engineering/evidence/slice1-asset-candidates.json`.

## Evidence Baseline

- `game-assets/asset-index.json` inventories 9,359 exported files by path, byte count, and SHA-256.
- `game-assets/extracted/exported/metadata/inventory.json` retains Unity source file, `path_id`, object type, and object name.
- `reverse-engineering/evidence/monoscripts.csv` identifies relevant `Assembly-CSharp` types, but contains no method bodies or serialized field values.
- `game-assets/extracted/joined_unity_files/level1` is the raw scene source. The current export does not reconstruct its hierarchy, component references, collision, spawn points, or sorting rules.
- Spine skeletons are JSON marked version `4.2.43`. Their `.json`, `.atlas`, and `.png` files are atomic bundles; renaming the atlas page requires a deterministic import rewrite.

## Recommended Actor Pair

### Hunter

Use the original hunter Spine bundle:

| Unity object | Exported path | Confidence |
| --- | --- | --- |
| `sharedassets1.assets:166` `Texture2D hunter` | `game-assets/extracted/exported/textures/hunter__166.png` | High |
| `sharedassets1.assets:245` `TextAsset hunter.json` | `game-assets/extracted/exported/text/hunter.json__245.bin` | High |
| `sharedassets1.assets:258` `TextAsset hunter.atlas` | `game-assets/extracted/exported/text/hunter.atlas__258.bin` | High |
| `sharedassets1.assets:1581` `GameObject Hunter` | not exported as a prefab | High identity, unavailable serialization |

The skeleton exposes 70 named animations. Slice 1 should limit its state projection to `hunter_stay`, `hunter_stay_back`, `hunter_walk`, `hunter_walk_back`, `h1_hit`, `h1_hit_back`, `hunter_damage`, `hunter_damage_back`, `hunter_dying`, and `hunter_die`.

The bundle contains 1,937 modular skin entries for bodies, faces, hair, costumes, jobs, and equipment. `All_h1` and `weapon_h1a_a_01` are concrete skin names; `weapon/h1_a_01` is a concrete atlas region attached as `weapon_01/sword`. This verifies a warrior/sword visual path, but not the original starter appearance or its database IDs. The web importer must support composed Spine skins. It must not invent a “default hunter” by selecting the first body, face, or hair entry.

### Monster

Use Goldblin as the primary Slice 1 monster:

| Unity object | Exported path | Confidence |
| --- | --- | --- |
| `sharedassets1.assets:115` `Texture2D mon_goldblin` | `game-assets/extracted/exported/textures/mon_goldblin__115.png` | High |
| `sharedassets1.assets:332` `TextAsset mon_goldblin.json` | `game-assets/extracted/exported/text/mon_goldblin.json__332.bin` | High |
| `sharedassets1.assets:338` `TextAsset mon_goldblin.atlas` | `game-assets/extracted/exported/text/mon_goldblin.atlas__338.bin` | High |

Goldblin has explicit `stay`, `stay_b`, `walk`, `walk_b`, `die`, and `die2` animations and skins `default` and `lv1`. It is sufficient to validate rendering, facing, movement, skin selection, and death. It does **not** expose a named attack or hit animation. The first compatible scenario must therefore treat Goldblin as a passive/training target until its original combat presentation is observed; it must not fabricate an attack clip.

`mon_a_01_1` remains a technical fallback because it exposes `atk`, `atk_b`, `dying`, and `die`, but choosing it changes the target actor and must be recorded as a deliberate fixture substitution. No recovered table binds either bundle to a numerical content ID, map, HP, attack, cadence, XP, or drop table. Those values must remain test fixtures until runtime/table evidence is recovered.

## Equipment And Drop Path

The visual equipment candidate is `weapon/h1_a_01` inside the hunter atlas, selected by the `weapon_h1a_a_01` skin and attached through `weapon_01/sword`. This is medium confidence for a visible sword and zero confidence for its stats or rarity.

The legacy code catalog proves the architecture includes:

| MonoScript index | Class | Evidence supplied |
| ---: | --- | --- |
| 3397 | `AdminGearData` | gear content table exists |
| 2334 | `AdminItemData` | item content table exists |
| 206 | `AdminDropUniqueGearData` | unique gear drop table exists |
| 2632 | `DropGearData` | runtime drop model exists |
| 2671 | `DropGearTouchCtrl` | dropped gear interaction exists |
| 4174 | `GearData` | gear runtime data exists |

The values and relationships of those classes are not available. `item_unique_01`, `ic_hunter_gear_0`, `coin 1`, and the `uniquedrop` audio clip are presentation candidates only. They must not be asserted as the drop from Goldblin or `mon_a_01_1`.

For the first end-to-end implementation, create a clearly namespaced migration fixture such as `fixture.slice1.training_sword`. It may grant a deterministic server-owned stat improvement, but it must not use a legacy item ID, name, rarity, rate, or balance claim. Replace the fixture only after an evidence record binds all of those fields.

## Combat Animation, Effect, And Audio

- Basic warrior animation: `h1_hit` / `h1_hit_back` in the hunter Spine skeleton, high confidence.
- Goldblin response: `die` or `die2`, high confidence that the clips exist but unknown trigger distinction.
- Technical fallback response: `dying` then `die` in `mon_a_01_1`, high confidence if that explicitly documented fallback is selected.
- Hunter response: `hunter_damage`, `hunter_dying`, then `hunter_die`, high confidence.
- `RoundSlashEffect_Anim` (`sharedassets1.assets:643`) and `RoundSlashEffect` controller (`sharedassets1.assets:1255`) form a named effect pair, medium confidence. It belongs to the hunter `h5_hit_roundslash` family and is outside the basic h1 scenario.
- `spearAttack` (`sharedassets1.assets:965`), `true_death` (`:932`), and `uniquedrop` (`:956`) have clear broad intent but no recovered event binding. Do not attach `spearAttack` to Goldblin or `h1_hit` based on the filename alone.
- No verified basic sword swing or ordinary monster-hit clip has been identified. The golden scenario should run without an attack SFX until observed evidence supplies one. Silence is preferable to a false mapping.

## Code Architecture Evidence

The legacy type inventory supports these boundaries without exposing their implementation:

- `HunterCtrl`, `HunterManager`, `HunterPatternCtrl`, `HunterData`, and `AdminHunterData` show separate control, coordination, runtime state, and content data concerns.
- `DamageManager`, `DamageCtrl`, and `DamageEffectCtrl` show that authoritative damage semantics and presentation were separate concepts.
- `AdminGearData`, `AdminItemData`, `AdminDropUniqueGearData`, `DropGearData`, and `GearData` show separate content definitions and owned/runtime instances.
- `ReviveBuildingCtrl`, `AdminReviveBuildingData`, and revive-property types show that revival is a wider village system, not merely a client timer.

These names justify domain boundaries only. They do not justify copying class names, field layouts, formulas, or state transitions into the rewrite.

## Golden Scenario

Record one deterministic server trace and one synchronized visual capture:

1. Account enters a minimal training area with one hunter and one Goldblin instance.
2. Hunter projects `hunter_stay`, receives a server target, then moves using `hunter_walk` or `hunter_walk_back` based on facing.
3. On server-confirmed attack start, client projects `h1_hit` or `h1_hit_back`; the client never submits damage.
4. Goldblin remains a passive target and projects `die` or `die2` when authoritative HP reaches zero. Record the chosen death clip as a fixture until the original trigger distinction is observed.
5. Server commits a deterministic fixture drop once, identified by encounter and reward-ledger idempotency keys.
6. Client presents the fixture drop without claiming it is a recovered legacy item.
7. Equip command references the owned instance. Server recomputes hunter stats and persists the change transactionally.
8. Reconnect after equip restores inventory, equipped instance, derived stat, encounter completion, and no duplicate reward.
9. The same seed and command trace produces byte-equivalent authoritative events on repeated runs.

The golden fixture values test the new architecture; they are not compatibility claims. Keep them in a fixture namespace and out of production content IDs.

## No-Guess Constraints

- Do not infer content IDs or table rows from Unity `path_id`; they are unrelated identifier spaces.
- Do not assign Goldblin or `mon_a_01_1` a zone, level, stat, reward, or rate from filename order.
- Do not bind `item_unique_01`, `ic_hunter_gear_0`, `coin 1`, or `uniquedrop` to the selected monster without a serialized reference or runtime observation.
- Do not select the first hunter skin entries as the original starter composition.
- Do not approximate legacy damage, defense, critical, cadence, pathfinding, revival, XP, or drop formulas and label them migrated.
- Do not use `spearAttack`, `roundSlash`, or their effects for the basic h1 attack merely because they are clearly named attack assets.
- Do not flatten Spine actors into loose sprites for the compatibility path. Preserve bones, slots, draw order, skins, events, animation timings, and premultiplied-alpha behavior.
- Do not rename or transcode source-of-truth assets. Derived web assets must retain provenance and checksums back to all source files.
- Do not claim the village map is migrated until scene hierarchy, renderer ordering, colliders, navigation bounds, and spawn coordinates are reconstructed or independently observed.

## Missing Evidence And Next Extraction Work

1. Reconstruct `level1` hierarchy and component references with a Unity-version-compatible extractor; publish object IDs for ground, structures, colliders, sorting groups, and spawn markers.
2. Dump serialized values for `AdminHunterData`, `AdminGearData`, `AdminItemData`, `AdminDropUniqueGearData`, and the monster/field data owners at runtime.
3. Capture an authorized clean-account session showing starter hunter composition, first field monster, ordinary attack audio/effect, first drop, equip delta, death, and revival.
4. Correlate runtime object/content IDs with asset names through logs or instrumentation; require two independent signals for balance/formula claims.
5. Export Unity Animator controller state/clip bindings for effects that are not embedded in Spine JSON.

## Slice 1 Acceptance Evidence

- Source hashes for every consumed asset match `slice1-asset-candidates.json`.
- Spine runtime demonstrates version-compatible loading, all selected states, facing, skin composition, and premultiplied alpha.
- Every content/rule value is tagged `legacy-verified`, `observed`, or `migration-fixture`; untagged values fail validation.
- Replay, reward idempotency, equip transaction, reconnect, and visual capture tests pass in Docker.
- Compatibility reporting counts only verified bindings. Fixture behavior is reported separately and never contributes to legacy-formula coverage.
