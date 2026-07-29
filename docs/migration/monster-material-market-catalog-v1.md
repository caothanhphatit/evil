# Monster material and Trading Post catalog v1

## Scope

`packages/content/releases/evil-hunter-1.411/monster-material-market-catalog.json`
normalizes the complete material supply path that can currently be proven from
the supplied `1.411` package:

```text
monster drop slot -> Hunter carried loot -> Trading Post purchase
                  -> building cost or recovered recipe input
```

This is a content/economy contract. It does not define UI layout, combat timing,
or unrecovered crafting gates.

## Confirmed coverage

- All `179` unique material indices referenced by the `195` packaged monster
  rows have an exact material definition.
- The catalog preserves all `1,617` monster material slots in original array
  order, including quantity and raw percentage value.
- Every one of the `179` droppable materials has a recovered Trading Post unit
  price and is listable by the rebuild market.
- `5,797` exact recipe input slots connect droppable materials to recovered
  product and building IDs.
- `132` exact construction/upgrade material-cost slots connect droppable
  materials to building IDs, levels, quantities, and source row kind.

Material names, `price`, `rating`, `level`, `convert`, `compose`, `parentIndex`,
and `magic` come directly from
`reverse-engineering/evidence/core-economy-tables-v1.json`. Trading Post price
direction is the existing strongly-inferred binding documented in the building
registry: the material `price` is the gold the town pays the returning Hunter.

## Runtime integration

`sell_hunter_loot` now settles any catalog-backed material instead of accepting
only the old `material:1` fixture. The server:

1. groups the Hunter's carried material stacks by canonical item ID;
2. rejects unknown, non-material, unpriced, or overflowing lines before mutation;
3. caps each deterministic sale line to the quantity the remaining town wallet
   can fund, retaining unsold loot and request remainder;
4. credits town stock and Hunter gold atomically in the durable aggregate;
5. writes one deterministic trade-settlement row per material line;
6. retains command-id idempotency for retries.

The current web-rebuild runtime also auto-settles requested common materials for
Hunters continuously farming an ordinary region. This is explicit product
behavior while the original walk-to-Trading-Post trigger remains unresolved;
it is not claimed as a recovered original-game method.

The browser still cannot submit quantity, price, or settlement outcome as a
trusted value.

## Explicit unresolved data

The catalog records `2,810` recipes whose material inputs and building family
are recovered but whose exact building-level/progression conditions remain
unresolved. Those links keep `requiredEvidence`; no default level or condition
is generated. Native crafting duration, some product mutation rules, the
ordinary material runtime modifier chain, and unique-gear selection also remain
outside this slice.

## Regeneration and validation

```sh
python3 tools/generate-monster-material-market-catalog.py
python3 -m unittest tools.tests.test_monster_material_market_catalog
cargo test --manifest-path apps/server/Cargo.toml \
  hunter_sells_multiple_catalog_materials_in_one_authoritative_settlement
```

The generator embeds byte counts and SHA-256 hashes for the monster catalog,
core economy evidence, and building registry. The freshness test regenerates
the complete catalog in memory and compares it with the committed JSON.
