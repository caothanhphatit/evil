# DTNGamer Community Corroboration (v1)

Source: [DTNGamer](https://dtngamer.com/evil-hunter-tycoon-thuoc-tinh-do-va-quai-thong-kho/), published 2021-01-02, modified 2021-01-03, accessed 2026-07-31.

This is a third-party historical reference. It is useful for hypotheses and terminology, but it does not override package, serialized-data, or runtime evidence.

## Additional claims

- Monster race groupings: Beast (Lycan, Hound), Undead (Skeleton, Witch, Reaper, Warrior, Bearer), Humanoid (Orc, Ogre, Cyclops, Human), Demon (Succubus, Vampire, Demon, article's “Ác Mông”), and Boss.
- Archangel and Demon Lord transformations are described as high-end weapon modifiers, allegedly obtained from Ancient and Primal weapon crafting.
- The article reports 5 seconds, ATK/DEF +30%, and movement speed +300% (3x) for the two transformations.
- A separate five-Virtue system is described for Torment 1 five-star gear upgraded with Angel Tears from Fallen Angel Gabriel, with thresholds 2/4/6. The reported Virtues are Devotion, Justice, Mercy, Glory, and Honor.

## Package comparison

The v1.411 quicksheet contains the authoritative transformation definitions:

- `gearProperty` 48: Archangel proc, random value `[3, 10]`, skill 1.
- `gearProperty` 49: Demon Lord proc, random value `[3, 10]`, skill 2.
- `gearSkill` 1: 5 seconds, DEF +30%, movement speed +200%.
- `gearSkill` 2: 5 seconds, ATK +30%, movement speed +200%.

The +300% article value conflicts with the package's +200% text. Keep the package value for the current rebuild, and leave the final multiplier semantics unresolved until runtime evidence is recovered. The normal decoded gear rows do not directly bind properties 48/49, so the Ancient/Primal crafting pool remains an unresolved generation rule; do not enable it solely from the article.

Virtues should be represented as a separate set/progression subsystem rather than ordinary prefix/suffix modifiers. Their exact formulas and availability remain community-corroborated only.

Machine-readable record: `reverse-engineering/evidence/dtngamer-evil-hunter-tycoon-v1.json`.
