# Corrective Village and Actor Binding Report

## Scope

This is a read-only binding report for correcting the empty `map_new01` presentation. It uses the recovered `level1` hierarchy, Unity object inventory, raw Unity dependencies, exported files, and Spine metadata. It does not claim a recovered starter save, first-monster table, spawn table, or gameplay formula.

Confidence labels are `confirmed`, `strong`, `medium`, and `unresolved`.

## Main map binding

The original saved scene does **not** use `map_new01` as the complete village. The principal village is the active root `Background` (GameObject path ID `1017`) and its children. `map_new01` is the sprite assigned to the separate child `Background[1017]/background_19[13910]` at Unity position `(-10.406, 8.258, 499)`. Rendering it alone as a full-screen map discards the positive-X village composition.

The browser-ready base tiles below are all `confirmed` SpriteRenderer bindings from `level1` to `sharedassets1.assets`:

| GameObject path | Unity position `(x, y, z)` | Sprite path ID | Browser asset |
| --- | --- | ---: | --- |
| `Background[1017]/background_01[2127]` | `(4.30, 14.11, 499)` | 1548 | `/game-assets/sprites/background_01__1548.png` |
| `Background[1017]/background_02[506]` | `(9.42, 14.11, 499)` | 1515 | `/game-assets/sprites/background_02__1515.png` |
| `Background[1017]/background_05[1839]` | `(24.78, 15.39, 499)` | 1522 | `/game-assets/sprites/background_05__1522.png` |
| `Background[1017]/background_06[1965]` | `(29.90, 14.11, 499)` | 1547 | `/game-assets/sprites/background_06__1547.png` |
| `Background[1017]/background_07[2615]` | `(4.30, 8.99, 499)` | 1533 | `/game-assets/sprites/background_07__1533.png` |
| `Background[1017]/background_08[1714]` | `(9.42, 8.99, 499)` | 1530 | `/game-assets/sprites/background_08__1530.png` |
| `Background[1017]/background_11[2491]` | `(24.78, 7.71, 499)` | 1508 | `/game-assets/sprites/background_11__1508.png` |
| `Background[1017]/background_12[1172]` | `(29.90, 8.99, 499)` | 1519 | `/game-assets/sprites/background_12__1519.png` |
| `Background[1017]/background_13[1441]` | `(4.30, 3.87, 499)` | 1506 | `/game-assets/sprites/background_13__1506.png` |
| `Background[1017]/background_14[2548]` | `(9.42, 3.87, 499)` | 1541 | `/game-assets/sprites/background_14__1541.png` |
| `Background[1017]/background_15[1159]` | `(14.54, 3.87, 499)` | 1542 | `/game-assets/sprites/background_15__1542.png` |
| `Background[1017]/background_16[1802]` | `(19.66, 3.87, 499)` | 1517 | `/game-assets/sprites/background_16__1517.png` |
| `Background[1017]/background_17[1211]` | `(24.78, 3.87, 499)` | 1516 | `/game-assets/sprites/background_17__1516.png` |
| `Background[1017]/background_18[943]` | `(29.90, 3.87, 499)` | 1535 | `/game-assets/sprites/background_18__1535.png` |

The following foreground village pieces are also exact `confirmed` scene bindings, but the current exported PNG catalog does not contain their external bundle sprites. Their immutable sources are available and should be extracted reproducibly before claiming a complete village:

| GameObject path | Unity position | Sprite name | Immutable Unity source |
| --- | --- | --- | --- |
| `Background[1017]/Village_Ground[20214]` | `(18.01, 10.53, 489)` | `skin_botton_00` | `game-assets/source/unity-assets/bin/Data/fc6112344ba24e846ab93206222bc5ce`, path ID 1 |
| `Background[1017]/Village_Gate[13190]` | `(13.06, 14.26, 493)` | `skin_gate_00` | `game-assets/source/unity-assets/bin/Data/69d52ffa47a4c624da3b4f6b9e3ba220`, path ID 1 |
| `Background[1017]/Village_Wall_A[5824]` | `(18.73, 8.55, 486)` | `skin_wallA_00` | `game-assets/source/unity-assets/bin/Data/b13a1ee42a18e194cbf1f8a074eb05e2`, path ID 1 |
| `Background[1017]/Village_Wall_B[18300]` | `(14.33, 9.53, 488)` | `skin_wallB_00` | `game-assets/source/unity-assets/bin/Data/44f0ab482bf1a8b4c8839576a308a16b`, path ID 1 |
| `Background[1017]/Village_Wall_C[23009]` | `(22.43, 9.915, 488)` | `skin_wallC_00` | `game-assets/source/unity-assets/bin/Data/509a760d786690d468890bf130819324`, path ID 1 |
| `Background[1017]/Village_Wall_D[5719]` | `(13.93, 11.42, 490)` | `skin_wallD_00` | `game-assets/source/unity-assets/bin/Data/119508b99aa4a4a49b5966c5738f62f1`, path ID 1 |
| `Background[1017]/Village_Wall_E[16348]` | `(19.515, 12.115, 492)` | `skin_wallE_00` | `game-assets/source/unity-assets/bin/Data/e64a839e6c603d34e9a89b00ffbbae53`, path ID 1 |
| `Background[1017]/Village_Bridge_A[12748]` | `(14.86, 12.49, 491)` | `skin_bridgeA_00` | `game-assets/source/unity-assets/bin/Data/bf7d60d575a7bb24fa3ba6753c8abaff`, path ID 1 |
| `Background[1017]/Village_Bridge_B[3759]` | `(22.17, 9.43, 487)` | `skin_bridgeB_00` | `game-assets/source/unity-assets/bin/Data/ff81645f1f305f74ca58acd9196ddc6f`, path ID 1 |
| `Background[1017]/Village_Bridge_C[2831]` | `(15.30, 8.67, 487)` | `skin_bridgeC_00` | `game-assets/source/unity-assets/bin/Data/c1bc3dd9f6b3ac342a7f8771421dbb0d`, path ID 1 |

World entities are dynamically parented under the active roots `Group[94]/HunterGroup[239]` and `Group[94]/EvilGroup[38]`; both are empty in the serialized scene. Therefore the hierarchy proves render ownership but not initial entity positions.

## Visible actor bindings

| Role | Binding and exact sources | Evidence | Confidence |
| --- | --- | --- | --- |
| Hunter | Spine family `hunter`: `/game-assets/text/hunter.json__245.bin`, `/game-assets/text/hunter.atlas__258.bin`, `/game-assets/textures/hunter__166.png` | 70 animations; 1,937 skins; dynamic world root `Group[94]/HunterGroup[239]`; `HunterManager[160]` | Confirmed family; unresolved starter skin and spawn |
| Generic NPC | Spine family `Npc`: `/game-assets/text/Npc.json__287.bin`, `/game-assets/text/Npc.atlas__314.bin`, `/game-assets/textures/Npc__74.png` | Animations `npc_stay`, `npc_talk`, `npc_walk` and back variants; skins `npc_01`, `npc_02`, `npc_builder`, `npc_trader`, `npc_witch`, etc.; `NpcManager[1532]` | Strong family; unresolved saved-scene skin/position |
| Chief/player avatar | Spine family `Chief`: `/game-assets/text/Chief.json__234.bin`, `/game-assets/text/Chief.atlas__268.bin`, `/game-assets/textures/Chief__175.png` | `stay`, `talk`, `walk` animations and composable body/head/costume/weapon skins | Strong identity; unresolved village placement and starter composition |
| Animal NPC | Spine family `npc_animal`: `/game-assets/text/npc_animal.json__294.bin`, `/game-assets/text/npc_animal.atlas__326.bin`, `/game-assets/textures/npc_animal__177.png` | Animations `hunter_stay`, `hunter_stay (cry)`, `hunter_walk`; skins `1`, `2`, `3` | Strong animal family; individual species/placement unresolved |
| Pet | Spine family `pet`: `/game-assets/text/pet.json__272.bin`, `/game-assets/text/pet.atlas__248.bin`, `/game-assets/textures/pet__181.png` | Walk/touch animations for `pet00` through `pet10`; `PetManager[1194]` | Strong family; exact owned/default pet unresolved |
| Mole NPC pair | `Background[1017]/MoleNpc[10774]/Npc01[20992]` and `Npc02[16774]`; exported frames include `/game-assets/sprites/img_mole_npc_1_0__382.png` through numbered variants and `/game-assets/sprites/img_mole_npc_2_0__856.png` through numbered variants | Exact active scene nodes, SpriteRenderers, Animators, and exported frame assets | Confirmed visible village decoration/NPC |
| Rift village NPC | `Background[1017]/RiftViligeNpc[20623]` at `(20.28, 6.98, 492)`; initial sprite `/game-assets/sprites/img_devilmotion_01_0__8345.png` | Exact active scene node, SpriteRenderer, Animator, collider and behavior components | Confirmed visible village NPC |
| Farm NPC 1 | `Background[1017]/SheepShip[15069]` at `(20.839, 12.731, 492)`; sprite `img_farm_npc_1_0` | Exact active scene binding; external bundle `2ed56cd26b560684c8009cb9d7e5cf41`, path ID 1 | Confirmed scene actor; PNG extraction pending |
| Farm NPC 2 | `Background[1017]/SheepDog[14028]` at `(21.96, 13.25, 492)`; sprite `img_farm_npc_2_0` | Exact active scene binding; external bundle `c22dc728206506d458a90434973e7b51`, path ID 3 | Confirmed scene actor; PNG extraction pending |
| Fallen-pasture NPC | `Background[1017]/AirShip[19693]` at `(19.624, 13.71, 492)`; sprite `fallen_pasture_npc_0` | Exact active scene binding; external bundle `91499e849527b97488223e41557f71c5`, path ID 1 | Confirmed scene actor; PNG extraction pending |

`Background[1017]/Bear[11350]` is not recommended as a literal bear binding yet. Although the GameObject and an animation/controller are named `Bear`, its initial sprite is `build_Adventure_0` from external bundle `8879c88468d6e574e83c0fb4e7e92e49` path ID 4. That identity conflict must be visually inspected first.

## Monster binding boundary

No serialized child exists under `Group[94]/EvilGroup[38]`, and no current table or scene reference binds a specific monster family to the first field. Consequently there is no high-confidence original first-monster binding.

The strongest **render-capable candidate** remains Spine family `mon_a_01_1` because it has `stay`, `walk`, `atk`, `dying`, and `die` in both directions and level skins `lv1` through `lv5`:

- `/game-assets/text/mon_a_01_1.json__289.bin`
- `/game-assets/text/mon_a_01_1.atlas__333.bin`
- `/game-assets/textures/mon_a_01_1__171.png`

This is `medium` confidence for technical completeness and `unresolved` confidence for original first-field identity. It may be shown only as an explicitly unverified field candidate, not as a recovered legacy binding. `mon_goldblin` is weaker for active combat because it has no verified attack animation.

## Recommended first visible roster

For the corrective village build, render the following in this order:

1. The 14 confirmed background tiles at their recovered Unity positions, then the extracted `Village_Ground`, walls, gate, and bridges using recovered Z ordering.
2. The confirmed Mole NPC pair and Rift village NPC at their recovered scene positions.
3. The two farm NPCs and fallen-pasture NPC after their external sprites/animation frames are reproducibly exported.
4. Two or three `hunter` Spine instances under the Hunter world layer, using neutral `hunter_stay`/`hunter_walk`; clearly tag their composition and spawn positions as migration fixtures until starter-save evidence is recovered.
5. One generic `Npc` instance and one `npc_animal` or `pet` instance for visible village life, with the same fixture labeling for skin/placement.
6. Keep monsters in the connected field side (`background_19`/`map_new01`) rather than inside the village. If `mon_a_01_1` is used temporarily, retain the unverified-binding marker in content data and UI diagnostics.

Do not flatten `map_new01` into the village background, invent a first monster ID, or present `All_h1` as a recovered starter skin. The map composition is now strongly evidenced; the runtime entity roster and exact starter bindings are not.

## Web packaging guardrails

The visible-world packager resolves the ten ground, gate, wall, and bridge
positions directly from `level1-scene-evidence-v2.json`. It requires the exact
serialized `Background` parent transform, identity rotation, and unit scale;
unsupported hierarchy changes fail packaging instead of falling back to copied
coordinates. Trimmed sprite anchors remain sourced from the immutable Unity
Sprite rect, pivot, and `textureRectOffset` metadata.

Town buildings are dynamic `BuildGroup` instances and therefore do not have
recoverable per-building positions in the saved `level1` scene. The browser
renders only authoritative building instances whose visual IDs are present in
the checksum-pinned visible-world release. Rendering no longer waits for the
separate 39 MB building UI/economy registry, because that registry does not add
placement evidence and previously left the town visually empty during load or
after a registry failure. Exact original saved-town building placement remains
unresolved.
