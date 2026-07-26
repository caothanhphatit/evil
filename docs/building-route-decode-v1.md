# Building Route Decode v1

This document is the migration scope for building UI. It separates recovered
evidence from runtime assumptions so later frontend/backend work does not route
different buildings through one generic popup.

## Decoded popup templates

| Popup | Source size | Role | Evidence |
| --- | ---: | --- | --- |
| `BuildingPop` | 560x900 | Multi-mode building management, product list, hunter tab, upgrade | Serialized scene |
| `GearCreatePop` | 450x950 | Weapon, armor, and accessory crafting | Serialized scene |
| `ConsumCreatePop` | 450x830 | Alchemist potion crafting | Serialized scene |
| `ProductCreatePop` | 450x860 | Inn/Infirmary/Restaurant/Tavern product quantity and conversion | Serialized scene plus user captures |
| `RequestPop` | 480x820 | Bounty request, not Trading Post | Serialized scene |
| `TradeWagonExchangePop` | 450x710 | Trade Wagon exchange | Serialized scene |

The important correction is that `ProductCreatePop`, not `ConsumCreatePop`, is
the quantity/material popup shown for Linen Bandage and Cake. Its recovered
hierarchy contains the exact product frame, minus/plus buttons, quantity-step
buttons, material grid, Produce button, and Close button. Its sprite bindings
include `popup_bg_9`, `store_bg_pd_g_9`, `store_btn_pd_m_9`,
`store_pd_minus_9`, `store_pd_plus_9`, `alchemist_make_bg_9`,
`box_menu_front_9`, `btn_green_01_9`, and `btn_red_01_9`.

## Priority routes

| Building | ID | Route | Popup chain | Decode state |
| --- | --- | --- | --- | --- |
| Trading Post | `build_3` | Purchase requests from hunter loot | `BuildingPop` | Layout/data decoded; native dispatch strongly inferred and confirmed by capture |
| Blacksmith | `build_10` | Weapon and armor crafting | `GearCreatePop` | Template and 2,755 recipe/rating rows decoded; native dispatch unresolved |
| Potion Shop | `build_11` | Display stock and sell potions | `BuildingPop` | Capability/template split confirmed; native dispatch unresolved |
| Alchemist's Home | `build_14` | Craft potions | `ConsumCreatePop` | Template and 40 consumable recipe-level rows decoded; native dispatch unresolved |
| Inn | `build_9` | Produce/use room products | `BuildingPop` -> `ProductCreatePop` | Seven products decoded; original runtime capture still needed |
| Infirmary | `build_12` | Produce bandages and treat hunters | `BuildingPop` -> `ProductCreatePop` | Seven products, conversions, capacity, and runtime capture decoded |
| Restaurant | `build_13` | Produce meals and feed hunters | `BuildingPop` -> `ProductCreatePop` | Seven products, conversions, capacity, and runtime capture decoded |
| Tavern | `build_19` | Produce drinks and serve hunters | `BuildingPop` -> `ProductCreatePop` | Seven products decoded; original runtime capture still needed |

## Route contracts

### Trading Post

`BuildingPop` supplies `TextTab`, `ratingTab`, `MoneyChange`,
`CreatePossible`, `RequestStateButton`, `GridBorder`, and `GridSecondBorder`.
The data adapter must expose town stock, hunter stock, requested quantity, unit
price, town gold, request state, and upgrade level. `RequestPop` must never be
used here because that popup is the bounty-kill request dialog.

### Blacksmith

`GearCreatePop` supplies the gear frame, gear type, main properties, sub
properties, required-material rows, Create, and Close. The route consumes the
recovered weapon/armor recipe by item type and rating. The outer product catalog
must not use a building thumbnail as an item icon.

### Potion Shop and Alchemist

These are separate routes. `build_11` owns potion display stock and hunter
sales through `BuildingPop`. `build_14` owns potion recipes through
`ConsumCreatePop`, including required materials, cooldown, effect, quantity,
Create, and Close.

### Inn, Infirmary, Restaurant, and Tavern

`BuildingPop` owns the Production/Hunters tabs, capacity, unlocked product rows,
effects, duration, fee, and upgrade hint. Selecting Produce opens
`ProductCreatePop`, which owns quantity changes, material conversion selection,
available quantity, Produce, and Close. Product unlock level and capacity are
server-authoritative.

## Remaining blockers before migration

- Decode or capture the native building-to-popup dispatch for every priority route.
- Capture the original Inn and Tavern screens; their product tables are decoded but their active tab states are not visually confirmed.
- Bind all recipe/material/item sprites by source ID instead of generic fallbacks.
- Decode button event semantics for stock sale/use and hunter assignment tabs.
- Add route-specific screenshot fixtures and pixel-diff thresholds before replacing the current UI.

The machine-readable source of truth is
`reverse-engineering/evidence/building-route-manifest-v1.json`. Regenerate it
with `python3 tools/generate-building-route-manifest.py`.
