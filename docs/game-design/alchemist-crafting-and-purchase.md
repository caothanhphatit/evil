# Alchemist Crafting And Hunter Purchase

## Evidence boundary

The package-backed building registry identifies `build_14` as the Alchemist's
Home and marks it with `potion-crafting` and `potion-display-and-sale`
capabilities. The five consumable families (`consumable:0..4`) have localized
names, exact level recipe inputs/outputs, and an eight-tier
`hunterPaysTownGoldByTier` price table. These rows come from
`packages/content/releases/evil-hunter-1.411/building-registry.json`, sourced
from `core-economy-tables-v1` and `serialized-building-tables-v1`.

Craft duration, exact building-level conditions, stack limits, and any native
craft queue timing remain unresolved. The rebuild therefore exposes the
resolved inputs, outputs, stock and prices, while rejecting missing or
unresolved values instead of inventing a fallback.

## Authoritative flow

1. The client sends `craft_shop_item` with the Alchemist instance, recipe ID,
   and quantity.
2. The server validates the recipe's `build_14` ownership, capability,
   quantity, capacity and material balance, then atomically consumes town
   materials and adds product stock.
3. The crafted stock is routed to Potion Shop `build_11`; the client sends
   `purchase_shop_item` with Hunter ID, `build_11`, and the recipe product ID.
4. The server validates stock, Hunter town state and gold, resolves the
   consumable's exact level price, then atomically decrements stock, debits
   Hunter gold, credits town gold, and records the owned product.

Potion Shop cards use the shared purchase detail flow. The player selects an
idle Hunter, sees the resolved effect and price, and can submit only while the
latest authoritative snapshot still reports stock and sufficient Hunter gold.

The purchase path uses the catalog price table because consumable product rows
do not carry a generic `salePrice`. Gear purchases continue to use their
resolved product sale price and shop route.

## Recovered Hunter automation

The original ARM64 client contains `HunterCtrl.ItemPotionBuy()` (token
`100686720`, module offset `0x341a7f0`, native size `0x20c`). Its recovered body
confirms that potion procurement is a Hunter-owned autonomous action: it
selects a pending product, records a local purchase result, invokes the common
pending-object helper, clears the pending object, and then calls three mutation
helpers before returning a Boolean success value. See
`reverse-engineering/evidence/original-native-hunter-auto-trade-decrypted-api35-v1.json`
and `docs/migration/original-native-hunter-auto-trade-evidence-v1.md`.

This is enough to preserve the state-machine boundary but not enough to claim
the original selection formula. The helper bodies have not yet bound:

- how `autoPotion01..03` and `potionCheck` choose a missing potion;
- the desired carry quantity or the meaning of "buy a little of each";
- the exact walk/speech transition and shop destination selector;
- wallet, stock, ownership and equip/consume mutation identities.

Accordingly, the rebuild supports a synchronous player-issued purchase of an
explicit catalog potion. Background auto-buy and a menu command that asks the
Hunter to choose products remain fail-closed until those helper identities or
typed before/after captures are recovered. The browser must not guess the
product list or quantity.

## Unresolved behavior

- Craft timers and asynchronous completion are not claimed.
- Stack limits and automatic potion use are not claimed.
- Native purchase helper identities and UI wording remain unresolved.
- Premium or protected items do not use this direct product path.
