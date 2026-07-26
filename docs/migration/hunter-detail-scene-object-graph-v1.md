# HunterDetailPop scene object graph v1

Date: 2026-07-26

## Scope and evidence boundary

This audit maps the serialized `HunterDetailPop` object graph in the original
1.411 `level1` scene. It records object IDs, MonoScript IDs and persistent
`Button.onClick` method names. It does not infer runtime data that is absent
from the scene and does not treat a hierarchy label as a recovered formula.

Primary inputs:

| Source | Size | SHA-256 |
| --- | ---: | --- |
| `game-assets/extracted/joined_unity_files/level1` | 16,845,572 | `450848793be37124e178cc072adec3255af8b2bc7d0c1e4986e473ba7c1014d8` |
| `game-assets/extracted/exported/metadata/inventory.json` | 11,308,781 | `f0dfe98c3c89810e47dd861b97cf853fe6cbaa48b05a4a1e6dc2a4db4cc66c7c` |
| `game-assets/source/unity-assets/bin/Data/Managed/Metadata/global-metadata.dat` | 15,315,640 | `ebbadaf6d94d838037b33bd77d60d861d84cc309b1a148383b3800d5e294b63e` |

The existing normalized scene is
`reverse-engineering/evidence/level1-scene-hierarchy.json`. Object IDs in this
report are the `GameObject.path_id` values from `level1`; component IDs are the
component object `path_id` values.

## Root controller

```text
UIPopupCanvas[224]/HunterDetailPop[2358]
  Image component [75449]
  HunterDetailPop component [74554], MonoScript [982], Assembly-CSharp
  PopupFunction component [75379], MonoScript [940]
```

The root has 647 serialized descendants. The raw `HunterDetailPop` component
payload directly contains PPtr references to the five content groups:

| Raw payload offset | Referenced group |
| ---: | --- |
| 628 | `StatGroup[1810]` |
| 640 | `SkillGroup[1338]` |
| 652 | `InventoryGroup[1671]` |
| 664 | `GrowUpGroup[17370]` |
| 676 | `RidingPetGroup[16890]` |

This is direct serialized ownership evidence, not only name correlation.
The root also references the five tab `Image` components at offsets
3028, 3040, 3052, 3064 and 3076 in the same order.

## Five-tab dispatch

All tab buttons target component `HunterDetailPop[74554]`. Their persistent
calls use method `TabAction`, event mode `Int`, and the following serialized
integer argument:

| Tab | Button object | Image component | Button component | Persistent call |
| --- | ---: | ---: | ---: | --- |
| Status | `StatButton[1705]` | `76146` | `74209` | `TabAction(0)` |
| Skills | `PropertyButton[1678]` | `76410` | `75139` | `TabAction(1)` |
| Inventory | `InventoryButton[1191]` | `74174` | `76277` | `TabAction(2)` |
| Growth | `GrowUpButton[12264]` | `91940` | `91667` | `TabAction(3)` |
| Riding pet | `RidingPetButton[14473]` | `96917` | `83351` | `TabAction(4)` |

`PropertyButton` is therefore the serialized name of the skills tab. It must
not be interpreted as a generic stat/property page in the migrated UI.

## Common shell and equipment

Confirmed common-shell objects under `Background[2060]` include:

```text
Title[1732]
GoldGroup[521]/Text[1290]
ReviveGroup[1295]/Grid[677]
ExpGroup[2651]/{Bg[968], Bar[2642], Value[2013]}
GearGroup[2407]
BottomGroup[1023]
```

### Standard equipment slots

Each standard slot is a `Button` whose persistent target is
`HunterDetailPop[74554]` and whose method is `GearInventoryClick`.

| Slot object | GameObject ID | Button component ID |
| --- | ---: | ---: |
| Gloves | 411 | 75729 |
| Helmet | 454 | 75095 |
| Necklace | 1108 | 74398 |
| Boots | 2209 | 75374 |
| Ring | 2408 | 74618 |
| Weapon | 2492 | 73886 |
| Armor | 2594 | 75853 |
| Belt | 5492 | 102349 |

Every one of these eight slot trees contains an item icon and level. The
serialized trees also expose rating stars and some combination of rune,
engraving and potential indicators. Their presence proves display capacity;
it does not prove unlock rules, stat formulae or the detail-popup payload.

### Appearance and auxiliary slots

| Object | ID | Persistent method |
| --- | ---: | --- |
| `Costume` | 1679 | `ShowCostumePop` |
| `WeaponCostume` | 4906 | `ShowWeaponCostumePop` |
| `SealCostume` | 9296 | `ShowSealPop` |
| `Fairy` | 14595 | `ShowFairyPop` |
| `RamblePet` | 23059 | `ShowRamblePetPop` |
| `WingCostume` | 20154 | `ShowWingCostumePop` |

Visibility buttons call `OnClickCostumeHide` for hat, costume, costume hat,
weapon costume, seal costume, fairy, ramble pet and wing costume. The shell
also contains distinct rendered-image nodes for body, hat, costume, wing,
seal, fairy and ramble pet. This supports a composed per-Hunter look rather
than one portrait or class-only skin.

`ShowGearPropertyButton[17612]` targets the root controller method
`OnClickAllGearProperty`.

## Status tab

`StatGroup[1810]` has explicit rows for:

- `Rating[2160]`, `LevelJob[1630]`, `Dps[2439]`, `DpsValue[439]`;
- `HpGraph[2570]`, `HungryGraph[1868]`, `FeelGraph[1611]`, `TireGraph[2141]`;
- `DamageBorder[1326]`, `ArmorBorder[982]`, `CriticalBorder[575]`,
  `AttackSpeedBorder[1102]`, `DodgeBorder[468]`;
- `StoneGroup[7291]` and rank-up indicator groups for needs and combat stats.

The status rows are display nodes owned by `HunterDetailPop`; there are no
separate per-row gameplay controllers in this scene. Exact calculation and
formatting methods are not recoverable from the protected MonoBehaviour
payload in this pass.

## Skills tab

`SkillGroup[1338]` owns a masked, scrollable `ScrollContents[4004]` and the
following fixed card families:

| Family | Objects |
| --- | --- |
| Basic | `FirstSkillGroup[1843]`, `SecondSkillGroup[1719]` |
| Sub-job | `SubJobSkillGroup1[19960]`, `SubJobSkillGroup2[12758]`, `SubJobSkillGroup3[3295]` |
| Third job | `ThirdJobSkillGroup1[3382]`, `ThirdJobSkillGroup2[7026]`, `ThirdJobSkillGroup3[22050]` |
| Heroic | `HeroicSkillGroup[14877]`, `HeroicSkillGroup (1)[18129]` |
| Reincarnation | `ReviveStoneSkin[1967]`, `ReviveDoubleAttack[1171]`, `ReviveQuicken[2643]`, `ReviveCurse[1450]`, `ReviveWisdom[20367]`, `RevivePenetrate[9516]`, `ReviveSixSense[7216]`, `ReviveVitality[4000]`, `ReviveRick[4015]`, `ReviveFindItem[21543]` |

Each ordinary skill card calls root method `SkillClick`. Each card contains
separate `Icon`, `LvText`, `Title` and `Desc` nodes. Job-change cards also
contain `JobText` and a dim node. This is direct support for identity, icon,
level, description, tier and locked-job presentation.

Additional calls:

- `SkillTreeButton[22121]` -> `SkillTreeClick`;
- reincarnation cards -> `RevivePropertyClick`;
- `ReviveReset[21958]` -> `OnClickReviveReset`.

The scene does not serialize the skill-definition IDs or the mapping from
`skill_hX_*` filenames to these cards. Binding those icons by filename order
would still be fabricated.

## Inventory/materials tab

The entire serialized graph is:

```text
InventoryGroup[1671] (inactive by default)
  MainBorder[1692] (Image + ScrollRect + Mask)
    Grid[1748] (VerticalLayoutGroup + ContentSizeFitter)
      Title[1850]  LocalizeTextSetter key: storageinfotap_1
      Grid[481]    GridLayoutGroup + ContentSizeFitter; no serialized children
      Grid[2049]   GridLayoutGroup + ContentSizeFitter; no serialized children
      Title[434]   LocalizeTextSetter key: storageinfotap_2
  Text[2336] (inactive) LocalizeTextSetter key: hunterdetailpop_8
```

The two item grids are empty in the scene and therefore populated at runtime.
No `DetailInventoryList` or material-row controller instance is attached
inside `HunterDetailPop`. `DetailInventoryList` exists as MonoScript ID 295,
but this scene does not prove that it is the runtime prefab used by either
grid. The exact per-Hunter inventory enumeration method, category meaning of
`storageinfotap_1/2`, item prefab and tap behavior remain unresolved.

Safe migration consequence: the tab may bind only explicit per-Hunter stacks.
Town storage, trading-post stock or pooled material counts are not equivalent
to either runtime-generated grid.

## Growth tab

```text
GrowUpGroup[17370] (inactive by default)
  MainBorder[15841] (Image + ScrollRect + Mask)
    Grid[6312] (ContentSizeFitter + GridLayoutGroup; no serialized children)
  Image[4956]
    TpText[19452]
```

The growth-node grid is also runtime-generated. The scene confirms the total
point text and grid target but does not contain the fifteen node instances,
node IDs, effects, prerequisites or costs. `HunterGrowUpPop` (MonoScript ID
`HunterGrowUpPop` (MonoScript ID
3428) and `HunterGrowUpPropertyPop` (MonoScript ID 2805) exist in the binary
catalog, but neither is attached to this inline tab graph; their exact role
cannot be substituted for the missing node binding without further evidence.

## Riding-pet tab

`RidingPetGroup[16890]` contains two explicit states:

```text
NoRideBorder[17888]
  NoRideText[6392]
  GoPastureBtn[14667]

MainBorder[15519] (Image + ScrollRect + Mask)
  Border[3932]/BgBorder[3287]
    RidingSelectBtn[17749]
    PetGradeAura[10155]/RidingPetTb[5055]
    RankObj[5534]
    RatingGroup[11509]
    TitleGroup[5134]/Name[9585]
    SkillGroup[21496]/{IconBg, TextGroup/Title, TextGroup/Desc}
    GearBorder[2966]/{GearButton_0[12949], GearButton_1[19025], GearButton_2[16885]}
  LayoutGroup[10111]/TraitGroup[23181]/{IconBg, TextGroup/Title, TextGroup/Desc}
```

Persistent calls, all targeting `HunterDetailPop[74554]`:

| Object | Button component | Method |
| --- | ---: | --- |
| `GoPastureBtn[14667]` | 94056 | `OnClickGoPasture` |
| `RidingSelectBtn[17749]` | 101811 | `OnClickOutOfRide` |
| `GearButton_0[12949]` | 85665 | `OnClickRidingPetGearButton` |
| `GearButton_1[19025]` | 97549 | `OnClickRidingPetGearButton` |
| `GearButton_2[16885]` | 98213 | `OnClickRidingPetGearButton` |

Each of the three gear buttons also owns a
`RidingPetGearUIFormFactor` component (MonoScript ID 4099): component IDs
95682, 95496 and 90880 respectively. This is the strongest scene evidence
for a dedicated three-slot pet-gear binding.

The populated state clearly supports pet identity, grade/rank presentation,
one displayed skill, one displayed trait and three gear slots. The scene does
not recover the pet stat formulas, selection list, mount mutation protocol or
gear-slot enum names.

## Recovered controller method surface

The following method names are preserved in serialized `Button.onClick`
payloads and target the original `HunterDetailPop[74554]` controller:

```text
TabAction
GearInventoryClick
ShowCostumePop
ShowWeaponCostumePop
ShowSealPop
ShowFairyPop
ShowRamblePetPop
ShowWingCostumePop
OnClickCostumeHide
OnClickAllGearProperty
SkillClick
SkillTreeClick
RevivePropertyClick
OnClickReviveReset
OnClickGoPasture
OnClickOutOfRide
OnClickRidingPetGearButton
```

Only `TabAction` survives as a clean method string in the protected IL2CPP
metadata. The remaining names are nevertheless direct serialized event
evidence. Native method bodies and field names remain unavailable because the
metadata v39 identifiers are selectively poisoned.

## Migration decisions supported by this audit

1. Keep one fixed `HunterDetailPop` shell and switch exactly five content
   groups with an enum matching serialized arguments `0..4`.
2. Treat equipment, optional appearance systems and pet gear as distinct
   slots; do not collapse them into a generic image array without stable IDs.
3. Skills require explicit group, title, description, level and lock/job text.
4. Inventory and growth rows require runtime data sources; their scene grids
   cannot seed definitions or ownership.
5. Riding-pet empty state is complete enough to reproduce. Populated state
   should remain unavailable until pet identity, skill, trait and three gear
   slots are projected explicitly.
6. Do not map material categories, growth nodes, skill icons or equipment
   effects from filename order alone.
