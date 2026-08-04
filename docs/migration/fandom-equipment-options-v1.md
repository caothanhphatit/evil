# Fandom Equipment Options (v1)

Source: [Evil Hunter Tycoons Fandom - Equipment](https://evil-hunter-tycoons.fandom.com/wiki/Equipment), accessed 2026-07-31 through the public MediaWiki parse API (page ID 151).

This page is a useful legacy, human-readable index of equipment options. It is not a complete or versioned dump of the package and must not be treated as authoritative data.

## What it adds

The page groups options into Positive, Negative, and Unique categories and documents the expected equipment families, C-to-SSS tier labels, and quality multipliers (`0.8`, `0.9`, `1.0`, `1.1`, `1.2`). Its option list covers the familiar stat, monster-type damage, resource, proc-skill, critical, lifesteal, pure-damage, and transformation families.

This supports building an admin/catalog view around modifier families rather than treating every option as an anonymous number. The transformation entries also corroborate the package's Archangel/Demon Lord properties.

## Boundaries against package evidence

The decoded `gearProperty` worksheet for package `1.411` has 125 rows and includes package-only or later/special properties not shown on this page, including Virtue-set reduction and many post-legacy skill properties. The page has no package IDs, roll ranges, `uniqueOptionYn`, `gearSkillidx`, or Ancient/Primal generation-pool information.

Therefore:

- Use Fandom names and Positive/Negative/Unique grouping as community labels.
- Use `quicksheet-decoded-v1.json` for IDs, ranges, skill links, and unresolved rows.
- Keep the SSS tier/option-count claims versioned and unconfirmed.
- Keep the Ancient/Primal transformation acquisition rule unresolved until the gear creation writer or runtime capture confirms it.

Machine-readable record: `reverse-engineering/evidence/fandom-equipment-v1.json`.
