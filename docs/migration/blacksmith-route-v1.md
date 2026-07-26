# Blacksmith Route Decode

This is the route-specific contract for `build_10` and the display shops
`build_7` (Weapon Shop) and `build_8` (Armor Shop). It is based on the
serialized `GearCreatePop` hierarchy and screenshots 0477-0480.

## Popup geometry

`GearCreatePop` is a `450 x 950` panel using `popup_bg_9`. The source popup has
a 3-column by 2-row item grid, four visible category tabs (weapon, armor,
gloves, boots), a Regular dropdown, craftable-only checkbox, upgrade hint, and
Upgrade/Close buttons. The selected item detail uses `box_gear_9`,
`item_grade_01`, five `ic_star` sprites, `alchemist_make_bg_9`, and required
material rows. Create and Close use `btn_green_01_9` and `btn_red_01_9`.

## Data routing

Only `recipe:{weapon|armor|helmet|gloves|boots}:{index}:rating:{rating}` rows
belong to this route. The normalized catalog currently contains 2,755 rows:
1,575 weapons, 215 armor, 535 helmets, 215 gloves, and 215 boots. A card is
formed by joining the recipe output (`gear:*`) to the authoritative gear table
for title, description, materials, and buy price. Unknown joins are omitted;
raw IDs and building thumbnails must not be rendered.

The icon field intentionally remains unresolved until the atlas region to
`gear:*` mapping is proven. The adapter uses `null` rather than a fabricated
fallback, preventing the broken images currently seen in the generic popup.

Weapon Shop and Armor Shop are display/sale routes (`BuildingPop`), not
crafting routes. They consume stock produced by Blacksmith and must never open
`GearCreatePop` directly.

The normalized source also contains helmet recipes, but the supplied original
runtime captures show no helmet tab in `GearCreatePop`. Those rows remain
server-side evidence and are intentionally hidden from the web route until a
runtime capture proves their visible navigation binding.

## Authoritative runtime flow

The server resolves the producer and destination shop from capability rows:
weapon recipes route to `weapon-display-and-sale`; armor, helmet, glove, and
boot recipes route to `armor-display-and-sale`. Crafting validates the
Blacksmith instance, tier (`rating < building level`), quantity, and material
costs before atomically adding stock to the matching display shop. A display
shop also rejects products above its own unlocked tier.

The captured shop list marks crafted gear as `On Display`; it does not expose a
player-operated Sell button. Hunter purchase settlement therefore remains
fail-closed until a concrete visiting hunter, wallet debit, owned gear instance,
and equipment handoff are bound. Adding town gold without those bindings would
mint currency. The item detail actions Lock and Dismantle likewise require the
unresolved owned-gear instance/stat model and are not represented as working
actions yet.
