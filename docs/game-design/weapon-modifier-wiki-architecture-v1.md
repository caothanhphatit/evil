# Weapon Modifier Wiki Architecture (v1)

The admin reference follows the same mental model as a PoE-style modifier
database: a base item and a modifier pool are separate catalogs.

## Separation of concerns

| Concern | Base weapon | Modifier pool/tier |
| --- | --- | --- |
| Identity | Class, family, implicit stats, visual | Modifier ID, localized text, family |
| Level | `baseLevelCap` and unlock level | `minimumItemLevel` per tier |
| Randomness | Does not choose the modifier | Weight, exclusive group, roll range |
| Effect of a higher base | Raises the item's possible level ceiling | Does not change weight or modifier identity |

Generation is therefore:

```text
itemLevel = serverRoll(base.unlockLevel, base.baseLevelCap)
eligibleTiers = modifierTiers where minimumItemLevel <= itemLevel
modifier = weightedPoolRoll(class, slot, exclusiveGroups)
tier = weightedOrDeterministicEligibleTier(modifier, itemLevel)
value = serverRoll(tier.minimumValue, tier.maximumValue)
```

The exact tier-selection policy remains a product decision. It must be
authoritative and deterministic for a given RNG seed; the client never sends a
tier or rolled value.

## Tier naming

The current rebuild cap (`level 800`) exposes `T1` through `T8`, mapped to the
existing eight evidence-backed level bands. `T9` is reserved for the first
release that raises the cap to `900`; it must not be populated with invented
original-game values before evidence or an explicitly reviewed rebuild design
exists.

## Wiki presentation

The modifier page should show one row/card per pool entry with:

- prefix/suffix, weight, family, and exclusive group;
- localized text and evidence state;
- a tier table (`T1`...`T8`) containing required item level and roll range;
- no difficulty column on the modifier itself.

Difficulty remains a progression/unlock filter for base weapons. It is not a
modifier tier selector.

## Storage compatibility

The first live release already contains evidence-backed rows under the legacy
physical column name `difficulty`. The admin API exposes those rows as
`tier`/`requiredItemLevel` so the public contract follows this architecture
without rewriting production content in place. A future content-release
migration may rename the physical column to `tier`; until then, both the
catalog generator and migration must preserve the compatibility mapping.
