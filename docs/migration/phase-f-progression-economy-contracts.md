# Phase F: Progression and Economy Contracts

This contract surface mirrors feature names and UI boundaries observed in the
legacy evidence set. It does **not** claim that the original catalog rows,
prices, rates, starter stats, or provider semantics were recovered.

## Authority and safety

The browser sends an intent only. The server may return `binding_blocked` with
the unresolved evidence bindings below. Until a binding is resolved from a
serialized content row or an authorized runtime trace, the server must not
invent an item grant, quest completion, currency amount, ad reward, product
price, or entitlement.

## Intent contract

| Intent | Evidence surface | Current result | Blockers |
| --- | --- | --- | --- |
| `open_hunter_progression(hunter_id)` | `HunterManager`, `HunterBorder`, `UserData` | `binding_blocked` | hunter catalog, starter stats, progression rules |
| `equip_hunter_item(hunter_id, item_id)` | `equip_*` assets, equipment UI | `binding_blocked` | equipment catalog, equipment rules |
| `claim_quest_reward(quest_id)` | `QuestPop`, `QuestList`, `UserQuestData` | `binding_blocked` | quest catalog, reward table |
| `open_shop(shop_id)` | `ShopPop`, `ShopTopMenuCtrl`, `ShopItemList` | `binding_blocked` | shop catalog, price table |
| `purchase_shop_item(shop_id, product_id)` | `ShopItemDetailPop`, `TokenShopPop` | `binding_blocked` | shop catalog, price table |
| `claim_mail(mail_id)` | `MailPopV3`, `MailListV3`, `UserMailData` | `binding_blocked` | mail schema, grant binding |
| `claim_rewarded_ad(placement)` | `AdsPop`, ad SDK classes | `binding_blocked` | placement, reward callback |
| `start_topup_purchase(product_id)` | `InAppPurchaser`, `AdminProductData` | `binding_blocked` | product catalog, receipt provider, entitlement rules |

These intents are intentionally accepted by the versioned protocol so the
original navigation can be migrated vertically. A blocked response is a
successful protocol response, not a failed payment or a zero-value grant.

## Resolution gates

Each blocker requires a pinned source hash and evidence reference. A catalog
may be promoted to runnable only after schema validation, server-side
authorization checks, idempotency tests, and a clean-account trace. Purchase
and rewarded-ad flows additionally require provider verification and replay,
refund, and revocation handling; development may use a fake provider but must
never treat a client success callback as proof of payment.

## Canonical content shape

`packages/content/progression-economy-v1.schema.json` is the source schema for
future recovered rows. Every field is a `boundValue` carrying its resolution
state, confidence, value, and pinned evidence locator. `null` is required for
an unresolved value; zero is a real value and must never be used as a
placeholder.

The expected record fields are:

| Domain | Identity and relationships | Values requiring evidence |
| --- | --- | --- |
| hunter roster/progression | hunter ID, archetype/job IDs, equipment slot IDs, unlock dependencies | base/derived stats, level/experience curves, promotion costs and effects |
| inventory/equipment | item ID, category, stack policy, eligible slots/jobs | stats, rarity, enhancement rules, sell/buy values, set effects |
| quests/missions | quest/mission ID, prerequisite IDs, objective type/target IDs | thresholds, time windows, rewards, repeat/reset policy |
| shop | shop/product IDs, category, entitlement target | price/currency, limits, rotation, discount and grant quantity |
| mail | template/mail IDs, sender/localization keys, attachment references | expiry, eligibility and attachment grants |
| rewarded ads | placement ID and provider mapping | cooldown, cap, eligibility and reward |
| topup/purchase | product ID, provider SKU and entitlement ID | localized price display, grant, refund and revocation semantics |

Runtime player state must reference canonical IDs and store quantities,
progress counters, claim timestamps, and ledger IDs only after the referenced
catalog is runnable. Display names, prices, rewards, and computed outcomes do
not come from the client.

## Typed-state boundary

Phase F keeps only the existing safe flow state (screen and boot completion).
No guessed progression, inventory, wallet, quest, mail, ad, or purchase state
is persisted by these intents. Once evidence is resolved, each domain gets a
durable ledger-backed state model and migration rather than extending the
fixture reward path.
