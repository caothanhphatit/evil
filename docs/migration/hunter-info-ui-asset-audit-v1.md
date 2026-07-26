# Hunter information UI and asset audit v1

Date: 2026-07-24

## Scope and evidence

This audit covers the six reference screenshots embedded in Google Sheet
`18R7Fd0bYoNYXJn0wmPivi9HiLSlfr4e5UW6p9_EB_oI`, the recovered Unity
`level1` scene, the immutable sprite export, the packaged browser assets, and
the current web Hunter roster implementation. It does not assign gameplay
meaning from a filename alone.

The sheet exports as an XLSX containing exactly six 591 x 1280 PNG images.
Five images show the same Hunter detail popup with a different selected tab;
the sixth image shows the eight-Hunter town roster that opens the popup.

Primary evidence:

- `game-assets/extracted/joined_unity_files/level1`
- `reverse-engineering/evidence/level1-scene-hierarchy.json`
- `game-assets/extracted/exported/sprites/`
- `apps/web/public/content/releases/evil-hunter-1.411/hunter-assets/catalog.json`
- `apps/web/src/main.ts`
- `apps/web/src/ui/hunter-roster.ts`
- `apps/web/src/styles.css`

## What each screenshot requires

### Screenshot 1: riding pet

The fourth tab is selected. The stable popup header, equipment stage and EXP
bar remain visible. The content states that no riding pet is mounted and
offers `Move to Ranch`.

The recovered scene confirms `RidingPetButton` and `RidingPetGroup`. The group
contains `NoRideBorder`, `NoRideText`, `GoPastureBtn`, `RidingSelectBtn`, three
gear buttons, a pet grade aura, rank frames, a pet skill group and a trait
group. Therefore the empty state in the screenshot is only one state of this
tab, not its complete data contract.

Required data is mounted-pet identity or an explicit empty state, ranch
eligibility, pet grade/rank, three pet gear slots, pet skills and pet traits.
The screenshot proves the empty state and navigation label. It does not prove
pet stat formulae, unlock rules or Ranch mutation behavior.

### Screenshot 2: growth / secret points

The middle tab is selected. It displays `Total Secret Points 0` and a grid of
15 growth nodes, each at `0/100` in this captured state.

The recovered scene confirms `GrowUpButton`, `GrowUpGroup`, `TpText`, and a
dedicated main border. The immutable export contains 15 `growth_ic_00..14`
icons plus growth panel, point, line and button sprites. Required data is the
Hunter's available secret points and one progress/max pair per growth node.

The screenshot and scene do not establish node IDs, localized names, effects,
cost curves, prerequisites, reset rules or the authoritative mutation. Those
must not be fabricated from the icon order.

### Screenshot 3: skills

The second tab is selected. It groups skills by `Basic Skill`, `2nd Skill` and
`3rd Skill`. Two basic skills are active at level 1; later-job skills are
visually locked and name their class-change requirement.

The recovered scene confirms that this is `SkillGroup`, reached from
`PropertyButton` in the serialized hierarchy. It includes first, second,
sub-job, third-job, heroic and reincarnation skill groups, plus class/job text,
levels and dimmed lock states. The export contains 50 `skill_h1..h5_01..10`
icons and many sub-job/heroic skill sprites.

Required data is skill definition ID, localized name and description, icon,
level, group/tier, equipped/active state, unlock state and an evidence-backed
unlock reason. The current protocol has ID, display name, icon, animation,
level, equipped slot and ready state, but does not carry description, tier,
unlock state or requirement.

### Screenshot 4: carried materials

The fifth tab is selected. It displays a `Material` section with item icons
and per-stack quantities. The captured Hunter carries four material stacks.

The recovered scene confirms `InventoryButton`, `InventoryGroup` and its
scrollable `MainBorder`. Required data is per-Hunter stack ownership, material
definition ID, localized name, icon and quantity. The item export and the
browser material catalog provide icons, but the current server projection only
exposes town-wide and pooled `hunter_quantity` values. It cannot truthfully
render this screenshot for an individual Hunter yet.

### Screenshot 5: status

The first tab is selected. It displays rarity, level, class, DPS, four needs
(HP, satiety, mood and stamina), six combat values (ATK, DEF, CRIT, ATK SPD,
Evasion and the displayed DPS summary), and Awakening progress.

The recovered `StatGroup` confirms `LevelJob`, `Rating`, `Dps`, `DpsValue`,
HP/hunger/feel/tire graphs, attack/armor/critical/attack-speed/dodge borders and
values, awakening stone group, and rank-up indicators. The nine
`h_detail_ic_01..09` sprites visually match HP, satiety, mood, stamina, attack,
defense, critical, attack speed and evasion in that order. This mapping is
supported by both the screenshot and the scene's named stat rows.

The current protocol is sufficient for level, rarity name, class name, HP,
satiety, mood, stamina, attack and defense. It is missing DPS, critical chance,
attack speed, evasion, awakening current/max, and the rank-up markers visible
in the recovered scene.

### Screenshot 6: town roster

The roster shows exactly eight cards in a two-column grid. Each card has a
per-instance rendered Hunter, name, rarity marker, level, class and current
activity/state, plus an `Info` button and another action button. This is not a
split-pane desktop list/detail view.

The current web screen instead uses a large two-column panel with active and
waiting lists on the left and a permanently embedded generic detail view on
the right. It therefore differs structurally from the reference before any
spacing or styling issue is considered.

## Shared Hunter detail shell

The recovered `HunterDetailPop` root is a full-screen overlay. Its central
`Background` is 560 x 1050 Unity layout units. The following are invariant
across all five detail screenshots:

- title and Hunter name;
- reincarnation stars and personal gold;
- central composed Hunter render;
- equipment/loadout stage;
- EXP current/max bar;
- five-tab strip;
- scrollable tab content area;
- close action.

The equipment stage is explicitly represented by `GearGroup`. Confirmed slots
include weapon, helmet, armor, gloves, boots, ring, necklace and belt. The same
group also contains costume, weapon costume, seal costume, wing costume,
fairy, ramble pet, runes, engraving, potential, item level and visibility
toggles. A minimal screenshot clone must not silently claim these optional
systems are supported merely because their sprite slots exist.

Recommended reusable FE composition:

```text
HunterRosterScreen
  HunterRosterCard[]
  HunterInfoModal
    HunterInfoHeader
    HunterLoadoutStage
      HunterPaperDoll
      HunterEquipmentSlot[]
    HunterExperienceBar
    HunterInfoTabBar
    HunterStatusTab | HunterSkillsTab | HunterGrowthTab
      | HunterRidingPetTab | HunterMaterialsTab
    HunterInfoActions
```

`HunterInfoModal` should own selection, close behavior and the fixed shell.
Each tab should receive a typed, server-projected view model. Missing server
fields should produce an explicit unavailable state for that tab or row; a
tab must never synthesize values or borrow another Hunter's pooled data.

## Asset coverage

The immutable export already contains the main source sprites required to
draw the reference UI:

| Family | Confirmed exported coverage | Current browser Hunter catalog |
| --- | ---: | --- |
| stat icons | 9 `h_detail_ic_*` | not packaged |
| tab frames/lines | `cha_tab_on`, `cha_tab_off`, two line sprites | not packaged |
| needs gauges/stars/panel pieces | 10 selected `character_*` sprites | partially packaged |
| equipment frames/dummies | 22 selected `equip_*` sprites | not packaged except unrelated `ic_hunter_gear_*` icons |
| equipment/item boxes | `box_gear_9`, `box_item_in_hunter` | not packaged |
| EXP gauge | 3 sprites | not packaged |
| base class skills | 50 icons | packaged |
| job traits | 69 icons | packaged, semantic bindings unresolved |
| growth nodes | 15 icons plus panel pieces | not packaged |
| riding-pet HUD | at least 44 selected `rp_*` sprites | only a small Hunter/skill subset packaged |
| material icons | separate material catalog | packaged separately |

Important reusable source assets include:

- `popup_bg_9__1928.png` (60 x 120, nine-slice source)
- `cha_tab_on_9__3380.png` / `cha_tab_off_9__6739.png` (16 x 16)
- `stat_frame_9__2231.png` and `popup_stat_box_9__5933.png`
- `equip_bg_9__2684.png`, `box_gear_9__2514.png`,
  `box_item_in_hunter__3566.png`
- `exp_gauge_back_9__4014.png`, `exp_gauge_in_9__6967.png`
- `h_detail_stone_box__2654.png`
- `growth_top_bg__3232.png`, `growth_ic_00..14`
- `rp_hunter_empty__6924.png`, `rp_trait_bg__5576.png` and the recovered
  riding-pet frames/buttons.

The generator `tools/generate-hunter-assets.mjs` currently omits most of these
families. Runtime code should not point directly at `game-assets/extracted`;
the generator and manifest need an evidence-preserving `hunter-info-hud`
package before implementation.

## Current FE gaps

`apps/web/src/main.ts` owns the DOM, networking reactions, shop UI and Hunter
roster rendering in one module. `renderHunterDetail()` renders gauges, a small
stat grid and a skill list simultaneously rather than the recovered fixed
shell plus mutually exclusive tabs. `HunterView` in
`apps/web/src/ui/hunter-roster.ts` is also a flattened compatibility parser
that accepts many possible field names, which is useful during migration but
is not an appropriate authoritative contract for the completed popup.

The new feature should use generated protocol types directly at its boundary,
then project them into a discriminated Hunter-info view model. Suggested
module split:

```text
apps/web/src/ui/hunter-info/
  model.ts
  project.ts
  modal.ts
  loadout-stage.ts
  tab-bar.ts
  status-tab.ts
  skills-tab.ts
  growth-tab.ts
  riding-pet-tab.ts
  materials-tab.ts
```

Shared primitives should cover nine-slice panel, icon slot, value row, gauge,
scroll viewport and source-style button. They should not encode gameplay
rules or infer content IDs from asset filenames.

## Implementable versus blocked

The fixed shell can be built from confirmed layout and sprite evidence once
the HUD assets are packaged. Status can be implemented partially with current
data, but must omit or mark unresolved DPS, critical, attack speed, evasion and
awakening until the server provides them. Skills can render the currently
known skills, but exact grouping, descriptions and lock requirements remain
blocked. Materials are blocked by the lack of per-Hunter inventory. Growth is
blocked beyond its visual shell and captured empty values because node
semantics and progression rules are unresolved. Riding pet can implement the
explicit no-mounted-pet state only; populated pet state and Ranch behavior are
not yet backed by complete data.

No semantic fallback should be used for any blocked field.
