# Full Source Asset Inventory (Phase A/B)

This report is the inventory baseline for the complete, source-faithful migration. It describes what is present in the extracted Evil Hunter Tycoon 1.411 evidence set and what must still be reconstructed before a browser release can claim compatibility. It is not a claim that all listed objects are already runnable in the web client.

## Integrity baseline

| Evidence | Count/size | Meaning |
| --- | ---: | --- |
| `game-assets/asset-index.json` exported files | 9,359 | Immutable exported files with SHA-256 hashes |
| Exported bytes | 190,429,626 | PNG, WAV, TTF, text, and metadata only |
| Unity joined files | 6 files / about 136 MiB | `level1`, `globalgamemanagers`, shared assets/resources |
| Addressables bundles | 7 | Localization and mono-script bundles |
| Exported sprites | 8,980 / 31,295,890 bytes | Includes UI, map, actors, effects, icons and animation frames |
| Exported textures | 152 / 41,514,176 bytes | Unity `Texture2D` exports; one duplicate name exists in inventory |
| Exported audio | 116 / 78,647,326 bytes | WAV `AudioClip` exports |
| Exported fonts | 2 / 16,659,944 bytes | Reproducibly recovered embedded TTF payloads |
| Exported text assets | 106 / 11,002,132 bytes | Spine JSON/atlas and other Unity text payloads |
| Exported metadata | 3 / 11,310,158 bytes | Inventory, type counts, extraction errors |

The object inventory contains 100,779 serialized Unity objects. The largest classes are `GameObject` (26,853), `RectTransform` (19,411), `CanvasRenderer` (16,814), `Sprite` (8,980), `Transform` (7,442), `SpriteRenderer` (6,015), `Animator` (5,967), `MonoScript` (4,438), `AnimationClip` (650), `AnimatorController` (526), and `ParticleSystem` (486). This is the evidence that the original scene/UI hierarchy is substantially larger than the current Slice 1 fixture.

The initial extractor recorded four errors, all now reconciled without hiding their history. Font path IDs 197 and 198 were reproducibly recovered from `m_FontData` as `DefaultFont2__197.ttf` and `NotoSansThai-Bold__198.ttf`. Texture2D path IDs 18 and 23 were inspected and verified as 0x0 `Font Texture` placeholders with no image payload, so they are `excluded-with-reason`. The outcomes remain machine-readable in `game-assets/manifests/full-source-inventory.json`; there are no unresolved extraction failures.

## Migration categories

### Maps and world presentation

Confirmed named source objects include `level1`, `map_new01`, `map_shading`, `devil_cloister_map`, `Pasture`, `background_01` through `background_18`, `back_anim_*`, `cloud_*`, dungeon thumbnails (`dg_dif_*`, `dc_thumb_*`), and adventure/raid map objects. `level1` is available as a joined Unity scene payload, but its hierarchy, sorting groups, colliders, navigation regions, spawn markers and runtime references have not been reconstructed. Map migration therefore requires a scene-schema extraction pass before any map is declared compatible.

### Hunters, monsters, NPCs and pets

The export contains 53 Spine skeleton text assets (paired `.json`/`.atlas` names), including `hunter`, `Chief`, `Npc`, `npc_animal`, `pet`, `Phoenix`, `gold_pet`, `devilcastle_pet`, 38 `mon_*` actors, and 7 `chaos_*` actors. Texture objects include the same actor families. Spine bundles must remain atomic: skeleton JSON, atlas text and all referenced atlas pages are one dependency unit. The hunter skeleton exposes modular skins and 70 animations; this is the source-faithful path for the player actor. No starter skin composition or numeric monster table is recovered by filenames alone.

### UI, icons and menus

The 8,980 exported sprites include named UI families such as `Button *`, `btn_*`, `setting_ic_*`, `equip_*`, `item_*`, `main_cont_*`, `pattern_ic_*`, `boss_select_*`, `boost_btn_*`, `dg_dif_*`, `point_chaged`, profile/chat/inventory/build/raid controls, and localization/intro screens. Serialized UI evidence includes 19,411 `RectTransform`, 16,814 `CanvasRenderer`, 16 `Canvas`, and 1,520 name matches for button-like objects. Web conversion must preserve anchors, pivots, nine-slice borders, text metrics, modal flow and locale keys; exporting loose PNGs is not sufficient.

### Animation, effects and particles

There are 650 `AnimationClip`, 526 `AnimatorController`, 486 `ParticleSystem`, 486 `ParticleSystemRenderer`, 103 `SortingGroup`, and 73 `Material` objects. Named effects include skill clips (`yetiking_*`, `lavadra_*`, `darkness_prince_skill_*`), weapon effects, damage/coin/drop/revive/level-up effects, portals, stage gems, fires, clouds and background animation. Animator state graphs and event bindings are not yet exported into a canonical browser schema. Gameplay-triggering events must be reimplemented as server rules; animation events can only schedule presentation cues.

### Audio

All 116 `AudioClip` objects have WAV exports and hashes. The set includes nine BGM tracks (`bgm_*`, `darknessprince_bgm`), combat/skill SFX, boss/raid ambience, drop/coin/revive-adjacent cues and named skill sounds. Source-to-event bindings, loop points, mixer groups, volume and spatial mode are not recovered from the flat WAV export. Audio migration needs a cue registry and browser variants (with autoplay-safe start and a user mute/volume policy) before release.

### Fonts, localization and text

Two Unity `Font` objects are exported as TTF files by `tools/extract-unity-fonts.py`. Their binary presence is verified, but browser layout metrics, fallback behavior, licensing, and glyph coverage still require validation. Six localization Addressables bundles are present for shared data, locale metadata, English, Japanese, Simplified Chinese and Traditional Chinese. Text migration remains blocked on extracting locale tables and completing font/glyph coverage checks; hard-coded replacement UI is not acceptable when a source key exists.

### Content data and serialized behavior

The inventory contains 4,438 `MonoScript`, 1,613 `MonoBehaviour`, 30 `Mesh`, 121 `MeshRenderer`, 83 `MeshFilter`, 105 `CircleCollider2D`, 32 `BoxCollider2D`, one `CapsuleCollider2D`, one `NavMeshSettings`, and one `RenderSettings`. These records establish the presence of gameplay/content components but do not expose trustworthy formulas, table rows or references. The C# catalog and runtime observations must be used to define canonical server content schemas; Unity `path_id` values are not game content IDs.

## Browser conversion requirements

1. Extract `level1` and all scene/prefab references into a versioned declarative scene schema (transforms, sorting, colliders, navigation, spawns, interactions and asset IDs).
2. Convert all Spine pairs without flattening bones, slots, skins, draw order, events or premultiplied alpha. Validate atlas page references and Spine runtime version.
3. Convert Unity Animator controllers/clips into a canonical state graph with loop/speed/transition/event metadata; keep authoritative hit timing on the server.
4. Port materials, shaders and particles to PixiJS filters/meshes or record an explicit device-tier equivalent. Preserve blend/tint/mask/dissolve intent.
5. Build a UI schema preserving anchors, pivots, slicing, font metrics, locale keys and modal/navigation state. Generate responsive layouts only after the source layout is captured.
6. Create an audio cue manifest with source hashes, event confidence, loop points and browser codecs. Do not bind a sound to a skill by filename alone.
7. Extract localization tables and font binaries, then run glyph coverage and fallback-chain checks for all six supported locale bundles.
8. Validate every released dependency through a content-addressed manifest; missing required assets fail the release rather than falling back silently.

## Current migration status

- **Copied/evidence-complete:** 9,359 exported files and joined Unity inputs are hashed and indexed; the full raw source set remains immutable. The two empty Texture2D placeholders are explicitly excluded rather than represented as fake images.
- **Runtime-addressable:** all 9,359 exports are available in Docker through `/game-assets/` and listed by the versioned `/full-assets/manifest.json` catalog. Addressability is not behavior binding.
- **Behavior-runnable:** only assets promoted by a validated content release are bound to scenes, UI, animation states, audio cues, or gameplay. The full catalog deliberately defaults every entry to `unbound-evidence`.
- **Unresolved:** scene hierarchy, complete UI reconstruction, Animator graphs, material/shader equivalents, localization/font extraction, audio event bindings, and content table values.
- **Do not claim:** 100% gameplay, 100% runnable assets, recovered rates/drops, or pixel-identical UX until the above evidence and validation gates pass.

## Evidence references

- `game-assets/asset-index.json`
- `game-assets/extracted/exported/metadata/inventory.json`
- `game-assets/extracted/exported/metadata/type_counts.json`
- `game-assets/extracted/exported/metadata/errors.json`
- `reverse-engineering/evidence/monoscripts.csv`
- `reverse-engineering/REPORT.md`
- `docs/assets/asset-migration-spec.md`
