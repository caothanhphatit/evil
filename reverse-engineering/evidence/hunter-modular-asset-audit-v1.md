# Hunter Modular Asset Audit v1

## Scope

This audit uses only the packaged Evil Hunter Tycoon 1.411 asset payload:

- `apps/web/public/content/releases/evil-hunter-1.411/hunter-assets/spine/hunter.json`
- `apps/web/public/content/releases/evil-hunter-1.411/hunter-assets/catalog.json`
- `apps/web/public/content/releases/evil-hunter-1.411/hunter-assets/portraits/`

It confirms asset existence, exact naming ranges, attachment-slot compatibility, and safe Spine composition ordering. It does not bind visual families to decoded gameplay jobs, item IDs, rarity, Characteristic, or generation probabilities.

## Confirmed inventory

The Spine 4.2.43 skeleton contains 1,937 skins and 70 animations. The modular Hunter-related pools are:

| Component | Exact confirmed pool | Count |
| --- | --- | ---: |
| Female standard appearance | `hunter_f_01` through `hunter_f_120`, continuous | 120 |
| Male standard appearance | `hunter_m_01` through `hunter_m_120`, continuous | 120 |
| Female darkload appearance | `hunter_f_darkload01` through `hunter_f_darkload40`, continuous | 40 |
| Male darkload appearance | `hunter_m_darkload01` through `hunter_m_darkload40`, continuous | 40 |
| Other Hunter body skins | `hunter_devil_01` through `hunter_devil_06` | 6 |
| Common costumes | `costum_h0_01` through `costum_h0_222`, plus `costum_h0_devil_01` through `costum_h0_devil_06` | 228 |
| H1 costumes | 12 numbered base skins plus 16 named/gender/advanced variants | 28 |
| H2 costumes | 12 numbered base skins plus 16 named/gender/advanced variants | 28 |
| H3 costumes | 12 numbered base skins plus 18 named/gender/advanced variants | 30 |
| H4 costumes | 12 numbered base skins plus 16 named/gender/advanced variants | 28 |
| H5 costumes | 12 numbered base skins plus 16 named/gender/advanced variants | 28 |
| Hats | `hat_01` through `hat_46`, continuous, plus `hat_empty` | 47 |
| Weapons | All `weapon_h*` skins described below | 1,059 |

The 326 `hunter*` skins therefore split exactly into 320 gendered appearances plus 6 `hunter_devil_*` skins. The 320 gendered appearances are not all named `hunter_[fm]_<number>`: indices above the standard 120 use the separate `darkload` Spine family.

## Appearance attachment contract

Standard and darkload appearance skins populate the same body/look slots:

```text
body_01
chic_slot
eye_01
hair
hair_b
hand_L
hand_R
head
leg_L
leg_R
lip_slot
```

Some appearances additionally populate `hair_b_deco`. The component carries body, face, eyes, lips, hair, hands, and legs together; the asset does not expose these as independently selectable generation pools.

The darkload skins replace the body/face attachments with `*_darkload` regions while reusing the numbered hair regions. This proves darkload is a distinct appearance family, not a missing numeric continuation in Spine.

## Costume contract

The normal H1-H5 costumes populate the clothing slots:

```text
cos_body
cos_hand_L
cos_hand_R
cos_pents_L
cos_pents_R
```

The H0/common costumes can additionally populate `back`, wing/deco slots, `hat`, and `hat2`. Therefore a later explicit hat skin must override an H0 costume's hat slots when both are equipped.

The `h0` and `h1`-`h5` name families are confirmed visual families. Their original gameplay eligibility, unlock rules, sex restrictions, and costume content-ID bindings are unresolved.

## Weapon families and slots

| Visual family | Skins | Distinct roots | Attachment slots |
| --- | ---: | ---: | --- |
| `weapon_h1_*` | 185 | 47 | `weapon_01` |
| `weapon_h1a_*` | 185 | 47 | `s_weapon` |
| `weapon_h2_*` | 185 | 47 | `weapon_02` |
| `weapon_h2a_*` | 1 | 1 | `s_weapon_shield`, `s_weapon_shield_h5` |
| `weapon_h3_*` | 185 | 47 | `weapon_03`, `weapon_03_effect` |
| `weapon_h4_*` | 185 | 47 | `weapon_04` |
| `weapon_h5_*` | 116 | 29 | `weapon_05` |
| `weapon_h5a_*` | 17 | 5 | `s_weapon_shield_h5` |

For H1/H1a/H2/H3/H4, 46 ordinary roots each have four skins: the root and `_1`, `_2`, `_3`; each family also has one `*_devil` skin. H5 has 29 four-variant roots. H5a has four `cos18`-`cos21` roots with `_0`-`_3` variants and one shield skin.

The suffix variants and letter groups are confirmed names only. The assets do not prove that they represent rarity, enhancement, difficulty, gear level, or a specific item definition.

## Aggregate and effect helpers

The 11 production-shaped aggregate class helpers currently catalogued are:

```text
All_h1
All_h1_duallist
All_h2
All_h2_executor
All_h2_templer
All_h3
All_h3_mistic
All_h4
All_h4_darkload
All_h5
All_h5_concentrate
```

Their names and compatible slots are confirmed, but their binding to original job/subjob IDs remains unresolved.

Separate conditional visual helpers exist and must not be treated as persistent appearance components:

- Attack effects: `atk_effect_h1`, `atk_effect_h1a`, `atk_effect_h2`, `atk_effect_h2b`, `atk_effect_h3`, `atk_effect_h4`, `atk_effect_h4_darkload`, `atk_effect_h5`.
- Advanced effects: `h1_whirlwind`, `h2_darkweapon`, `h2_executor`, `h2_silverweapon`, `h3_minstrel`, `h3_mysticarrow`, `h4_darkload_aura`, `h5_concentrate`, `h5_piercingthrust`.
- Emotions: 18 `emotion_*` skins.
- Optional extras: 30 `z_cos_wing_*` skins and 34 `z_vehicle_*` skins.
- Reset helpers include `hat_empty`, `back_empty`, `hair_empty`, `hair_b_empty`, and `hair_b_deco_empty`.

## Safe Spine composition order

Asset slot collisions establish this safe persistent composition order:

```text
aggregate class helper
-> per-instance appearance
-> costume
-> explicit hat or hat_empty
-> weapon
```

In Spine, later skins override attachments with the same slot/attachment key. This ordering is necessary because aggregate helpers contain default body/clothing/hat/weapon attachments, appearance must replace the default body/look, and an explicit hat must replace any hat embedded by a common H0 costume. Attack effects, emotions, wings, and vehicles should be applied as separate state-dependent overlays after the persistent loadout rather than stored as identity components.

## Portrait audit and the 121-160 distinction

The packaged portraits are complete and continuous:

| Portrait family | Exact range | Count |
| --- | --- | ---: |
| Female | `hunter_f_01` through `hunter_f_160` | 160 |
| Male | `hunter_m_01` through `hunter_m_160` | 160 |

Consequences:

1. Portraits `hunter_f_121` and `hunter_m_141` are valid packaged portrait assets. They are not invalid portrait IDs.
2. Spine skins named `hunter_f_121` or `hunter_m_141` do not exist. Using portrait stem as the Spine skin name is invalid for portrait indices 121-160.
3. The asset ordering strongly suggests, but does not directly serialize, this pairing:

```text
portrait 001-120 -> hunter_[fm]_01 .. hunter_[fm]_120
portrait 121-160 -> hunter_[fm]_darkload01 .. hunter_[fm]_darkload40
```

The second line must remain `strongly_inferred` until a serialized body-index mapping or runtime trace confirms it. It should not be silently replaced with a different standard appearance.

Exact verified portrait paths relevant to the current demo are:

```text
/content/releases/evil-hunter-1.411/hunter-assets/portraits/hunter_f_121__5341.png
/content/releases/evil-hunter-1.411/hunter-assets/portraits/hunter_m_141__3522.png
/content/releases/evil-hunter-1.411/hunter-assets/portraits/hunter_f_111__5928.png
/content/releases/evil-hunter-1.411/hunter-assets/portraits/hunter_m_117__3163.png
```

The draft migration `0015_hunter_visual_composition.sql` currently references nonexistent replacement files `hunter_f_111__4744.png` and `hunter_m_117__2866.png`. It must not be deployed in that form. More importantly, replacing portraits 121/141 merely to fit the standard Spine naming range discards the likely darkload pairing rather than resolving it.

## Catalog gaps blocking a reliable generator

`hunter-assets/catalog.json` currently publishes portraits, 11 aggregate helpers, all weapon skins, and animations. It does not publish structured pools for:

- standard and darkload appearances by sex;
- portrait-to-appearance candidates and confidence;
- common and H1-H5 costumes;
- hats and empty/reset helpers;
- attack/advanced effect overlays;
- wings and vehicles;
- attachment-slot contracts.

`tools/generate-hunter-assets.mjs` should derive these arrays directly from `hunter.json`, preserve exact names, and attach evidence/confidence. Gameplay bindings and RNG weights must be supplied later from decoded data; they must not be inferred from array position alone.

## Implementation guardrails

- Store portrait asset and Spine appearance skin as separate fields/components.
- Validate every composed skin name against the skeleton catalog before persistence or projection.
- Do not derive appearance skin by copying the portrait stem for indices above 120.
- Do not bind weapon suffixes, costume variants, or aggregate helpers to rarity/class/subjob semantics without decoded rows.
- Do not persist attack effects or emotions as identity/loadout components.
- Keep the original per-instance component snapshot normalized as class helper, appearance, costume, hat, and weapon; optional wing/vehicle slots can be added independently when their runtime rules are recovered.
