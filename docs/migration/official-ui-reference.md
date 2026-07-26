# Official UI Reference

The visual rebuild is checked against the official Google Play listing for
`com.superplanet.evilhunter`. Promotional screenshots are reference evidence,
not runtime textures; the web build must use the recovered XAPK assets.

## Confirmed Layout

- Gameplay uses a portrait mobile viewport.
- Town and combat scenes fill the viewport behind the HUD rather than being
  contained inside a landscape canvas.
- Resource and mode indicators occupy the top edge.
- The persistent bottom actions are ordered `Construct`, `Dungeon`, `Hunters`,
  `Storage`, and `Shop`.
- The recovered icons map in that same order to `menu_ic_01__6756.png` through
  `menu_ic_05__6398.png`.

## Runtime Policy

- Desktop centers a 9:16 gameplay stage instead of stretching the mobile game.
- A wider scene is cropped by the portrait camera; empty bars must not appear
  inside the gameplay stage.
- Unimplemented branches are disabled without exposing migration diagnostics.
- The current checksum-pinned `map_new01` remains a labelled field candidate.
  It is not evidence of the original town layout.

## Remaining Town Gap

The official town reference contains dense building placement, paths, NPCs,
speech indicators, decoration and top HUD elements. The current evidence proves
many relevant GameObject and SpriteRenderer names but does not yet resolve a
complete building-to-sprite runtime release. Until that compiler exists, the
field candidate is preferable to the previously broken collage of unrelated
`Village_*` fragments. It must not be presented as a source-faithful town.
