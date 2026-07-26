# Hunter information UI reference v1

## Scope and provenance

This inventory is based only on the six screenshots in the user-provided Google Sheet. It records visible UI and the minimum state implied by that UI. It does not assign gameplay meaning to an unidentified icon and does not fill missing states with assumptions.

- Sheet: `18R7Fd0bYoNYXJn0wmPivi9HiLSlfr4e5UW6p9_EB_oI`, `gid=0`
- Export inspected: `/tmp/hunter-info-reference.xlsx`
- Export SHA-256: `e568d426df95037f200d025810469a7999d59764d357c6d07738be5aa5420024`
- All six embedded PNGs are `591x1280`.
- Sheet order is: roster, stats, skills, secret points, riding pet, materials.

The local extraction used during this audit is `/tmp/hunter-info-reference.YEdlgb/xl/media/`. The image names below are the names in the exported workbook, not semantic asset names from the game.

## Shared Hunter information shell

Screenshots 2, 3, 5, 6 and 1 show the same Hunter information shell with only the lower tab content changing.

Visible shared hierarchy, from top to bottom:

1. Framed modal over a dimmed but still visible Town/Hunters screen.
2. Header with a left square silhouette icon, centered text `Charismatic Ocós`, and a right lock icon.
3. Reincarnation area with five star positions and the text `Reincarnation`.
4. Hunter money panel at upper right: coin icon plus `6,684`.
5. Central full-body Hunter sprite surrounded by item/appearance slots.
6. Purple `EXP 262/322` progress bar.
7. Five icon-only tabs in one horizontal strip.
8. Scrollable tab content area.
9. A single red `Close` button centered at the bottom.

The title visibly combines two pieces of text (`Charismatic` and `Ocós`), but the screenshot alone does not prove the source fields or their order for every localization. The lock icon is visibly present; its exact mutation semantics and confirmation flow are not shown.

### Central Hunter and slot area

The common slot area visibly requires more than a portrait:

- One rendered Hunter body in current appearance/equipment.
- Twelve surrounding square positions in a roughly three-left/three-inner-left/three-inner-right/three-right arrangement. Some are occupied and some are empty/disabled.
- Occupied examples include an axe, torso armor, boot, glove/arm item, blue clothing, a pink costume-like icon, and two `6`-marked effect-like icons.
- Several outer positions have a small purple auxiliary button attached to the slot edge.
- One occupied upper slot has a small eye icon attached above it.
- Empty positions use distinct placeholder symbols rather than one generic blank tile.
- Some occupied tiles show a small gold star marker.

The exact equipment-slot names cannot be safely derived from these screenshots. Implementation must preserve stable slot identifiers and icon/state metadata, but must not label the unidentified positions until matched to mined UI/data definitions.

The screenshots do not show any slot detail popup, equip/unequip flow, costume visibility popup, or item comparison popup.

## Screenshot 1: materials tab

- Local image: `/tmp/hunter-info-reference.YEdlgb/xl/media/image1.png`
- SHA-256: `cd1ba106343ece13e0238d2ccfc86eaa426941348807d1fe868ef4d2ae000f09`
- Sheet position: bottom-right.
- Selected tab: fifth tab, briefcase icon.

Visible content:

- Section title `Material`.
- Four square material tiles in one row.
- Visible quantities: `18`, `16`, `8`, and `6`.
- The four visible icons resemble a board/plank, rolled white material, orange powder/ore, and a round brown material.
- A vertical scrollbar indicates the inventory can exceed the visible area.
- No per-item name, quality, price, action button, selected state, or detail popup is visible.

Minimum UI state indicated by the screenshot:

- Ordered material entries with stable item identity, icon, quantity, and visibility/order.
- Scroll position/content overflow.

Not evidenced:

- Whether tapping a material opens a child popup.
- Whether this tab includes non-material inventory categories.
- Stack limits, selling, consuming, sorting, or filtering.

## Screenshot 2: stats/status tab

- Local image: `/tmp/hunter-info-reference.YEdlgb/xl/media/image2.png`
- SHA-256: `8ca5f64b39886d2e71351549dc8f87954bc2030036c0044bc3071ca3119435b2`
- Sheet position: top-center.
- Selected tab: first tab, document icon.

Visible content:

- Rarity label `RARE`.
- `Lv.24 Berserker`.
- `DPS 245.77`.
- Four current/max status bars:
  - `HP 775/7283`
  - `Satiety 100/140`
  - `Mood 57/120`
  - `Stamina 96/100`
- Five combat stats:
  - `ATK 639`
  - `DEF 444`
  - `CRIT 7%`
  - `ATK SPD 2.60`
  - `Evasion 3%`
- `Awakening 0/4` with a dedicated icon.
- Each status bar has its own icon and color treatment.
- ATK and DEF values use distinct highlight colors; the remaining values are white.

Minimum UI state indicated by the screenshot:

- Hunter rarity display, level, class display, DPS, four current/max resources, five combat values, and Awakening current/max.
- Display formatting is part of the contract: integer, decimal, percentage, and current/max are not interchangeable.

Not evidenced:

- Formulas for DPS or derived stats.
- Base-versus-bonus breakdown.
- Tap behavior on a stat or Awakening icon.
- Additional stats below the visible viewport.

## Screenshot 3: skills tab

- Local image: `/tmp/hunter-info-reference.YEdlgb/xl/media/image3.png`
- SHA-256: `b3b8eaf9b7724b67785a15924b23ab47ec91b3067fc51879f11e34d7feda5160`
- Sheet position: top-right.
- Selected tab: second tab, Hunter silhouette icon.

Visible content:

- `Basic Skill` section with two active skill cards:
  - `Fury`, `Lv.1`, description visibly begins `Attacks quickly for a certain time and increases Attack Speed.`
  - `War Cry`, `Lv.1`, description visibly begins `Charge to enemy and Stun it.`
- Two columns headed `2nd Skill` and `3rd Skill`.
- Four dark/locked-looking class-change skill cards are visible:
  - `Dual Weapon`, `Lv.0`, available after changing class to `Duelist`.
  - `Battle Shout`, `Lv.0`, available after changing class to `Barbarian`.
  - `Death Coil`, `Lv.0`, with a class-change requirement whose class label is too low-contrast to transcribe confidently from this screenshot.
  - `Aura Blade`, `Lv.0`, available after changing class to `SwordSaint`.
- Every card has a dedicated skill icon.
- A vertical scrollbar indicates more skill content than is visible.

Minimum UI state indicated by the screenshot:

- Skill identity, localized name, icon, level, localized description, grouping/column, current availability, and an unlock requirement rendered as class-change text.
- Locked and available cards have materially different visual states.

Not evidenced:

- Skill upgrade action/cost.
- Selection/equip behavior.
- Cooldowns, damage coefficients, passive/active markers, or maximum level.
- Skill detail child popup.

## Screenshot 4: Hunter roster

- Local image: `/tmp/hunter-info-reference.YEdlgb/xl/media/image4.png`
- SHA-256: `1c6e671ee1f134ec8554a39757d0cf8fdf41f0634f864fbf0ad63b1f571eb346`
- Sheet position: top-left.

Visible hierarchy:

1. Town remains visible above and behind the roster.
2. Top-right HUD displays `8/8` beside a person icon.
3. Roster header row has `Place the Hunting Grounds`, `Sort Hunters`, and an `X` close button.
4. Eight Hunter cards are displayed as a four-column by two-row grid.
5. Each card has name plus a colored one-letter rank marker, level plus class, full-body sprite, current activity/status text, an `Info` button, and a separate green square icon button.

Visible card examples:

| Name | Marker | Level | Class | Activity/status |
| --- | --- | ---: | --- | --- |
| Ocós | R | 24 | Berserker | Fun |
| Sharon | N | 15 | Ranger | Heal |
| Steen | N | 13 | Paladin | Dead |
| Rak | S | 12 | Sorcerer | Sell Material |
| Eluin | R | 10 | DarkKnight | Sell Material |
| Reika | S | 17 | Berserker | Dead |
| Holmes | H | 17 | DarkKnight | Dead |
| Eileen | N | 4 | Berserker | Fun |

The one-letter marker is recorded exactly as displayed. Its full enum labels must come from mined data/localization rather than expansion from this screenshot.

Minimum UI state indicated by the screenshot:

- Active occupancy/current capacity.
- Per-Hunter stable identity distinct from display name, display name, rank marker, level, class, composed full-body look, current activity/status, and action availability.
- Sort state/action and hunting-ground placement action.

Not evidenced:

- Sort choices or child popup.
- Meaning/action of the green square card button.
- Waiting-queue roster, banish flow, arrival animation, or empty-capacity state.
- Whether `8/8` includes only active Town Hunters or any queued Hunters.

## Screenshot 5: secret points tab

- Local image: `/tmp/hunter-info-reference.YEdlgb/xl/media/image5.png`
- SHA-256: `744ff721ee6899a337e06097bbd005e63e2e418b0686f266c1cc52fdcd5a5730`
- Sheet position: bottom-left.
- Selected tab: third tab, scroll/document-like icon.

Visible content:

- `Total Secret Points 0` in a highlighted pill at the upper right.
- Fifteen icon nodes in a `6 + 6 + 3` grid.
- Every visible node displays `0/100`.
- Every node has a distinct monochrome icon.
- There are no visible node names, descriptions, allocation buttons, prerequisites, connecting lines, or reset action.

Minimum UI state indicated by the screenshot:

- Total available Secret Points.
- Ordered definitions for at least fifteen nodes, each with stable identity, icon, current points, maximum points, and display order.

Not evidenced:

- Node names/effects.
- Whether nodes are clickable or how points are spent/refunded.
- Unlock conditions, dependencies, costs, or server-side formulas.
- A node detail child popup.

This tab cannot be implemented faithfully from the screenshot alone beyond the grid shell and point counters. Node definitions and behavior require mined definitions/localization.

## Screenshot 6: riding pet tab

- Local image: `/tmp/hunter-info-reference.YEdlgb/xl/media/image6.png`
- SHA-256: `208be3f33b7e5e59e3084ca4fe1af95fd2c838c819049898d8325a746ec1112c`
- Sheet position: bottom-center.
- Selected tab: fourth tab, horse-head icon.

Visible empty state:

- Message: `No riding pets are being mounted.`
- Gold/brown outlined action button: `Move to Ranch`.
- A vertical scrollbar remains visible at the right edge of the content area.

Minimum UI state indicated by the screenshot:

- Whether a riding pet is currently mounted.
- Empty-state localized text.
- Availability of a navigation action to the Ranch.

Not evidenced:

- Mounted-pet presentation.
- Pet stats, bonuses, identity, icon, level, rarity, or equipment.
- Ranch selection/mount/unmount flow.
- What happens if Ranch is unavailable or locked.

Only this empty state is safe to reproduce from the reference set.

## Cross-screen requirements and evidence gaps

The six screenshots establish one roster screen and one reusable Hunter information shell with five tabs. They do not establish complete behavior for all tabs.

Safe UI/data coverage from screenshots:

- Roster card structure and currently visible activity strings.
- Common Hunter header, look/equipment presentation, money, EXP, tab strip, scrolling content, and close action.
- Full visible stats/status field set.
- Visible skill cards, groupings, levels, descriptions, and class-change locks.
- Material item grid shape and quantities.
- Secret-points grid shape and counters, but not node semantics.
- Riding-pet empty state only.

Required evidence before claiming full migration:

- Semantic identifiers for all central equipment/appearance positions and the purple auxiliary controls.
- Equipment detail/equip/visibility behavior.
- Complete skill definitions, unlock rules, upgrade behavior, and localization.
- All fifteen Secret node definitions, icons, effects, point-allocation rules, and any child popup.
- Mounted riding-pet state and Ranch navigation result.
- Sort Hunter options, the green card action, and waiting/arrival/banish states.
- Exact localization keys rather than hardcoded screenshot strings.

No fallback values or invented labels should be introduced for these gaps. A partially migrated tab should remain unavailable or explicitly evidence-limited until its authoritative definitions are recovered.
