# Hunter Personality System

Status: working specification
Package baseline: Evil Hunter Tycoon `1.411`
Last reviewed: 2026-07-26

## Purpose

Each Hunter has one generated personality/Characteristic that modifies combat,
economy, needs, group behavior or cosmetic dialogue. The Rust server owns the
assignment roll and every gameplay effect. The browser only renders the selected
definition and server-calculated values.

This document distinguishes the supplied package from the publisher's public
[Hunter Personality](https://evilhunter.zendesk.com/hc/vi/articles/360039887431-T%C3%ADnh-C%C3%A1ch-Th%E1%BB%A3-S%C4%83n)
article. The public article may describe a different live release.

## Confirmed content boundary

- **Package-confirmed:** the `personality_global` QuickSheet asset contains 33
  rows with an exact integer `index`, localized name, Korean description
  template and integer `keepValue`.
- **Package-confirmed:** `HunterData.personality` stores a per-Hunter integer
  personality value.
- **Official-reference:** the public article describes 27 player-facing
  personalities and their qualitative effects.
- **Unresolved:** the native assignment method, generation pool, weights,
  exclusions, tutorial overrides, request-type overrides and rarity coupling.
- **Product decision (`web-rebuild-v1`):** all 33 packaged definitions are
  eligible and selected uniformly. Each definition has exact probability
  `1/33` (approximately `3.030303%`) before any future versioned override.

`keepValue` is an effect parameter, not a generation weight. It appears inside
the packaged effect-description template, for example Strong uses
`keepValue = 10` in an attack-power percentage statement. No recovered field or
method establishes a probability for any personality. Do not normalize these
values into percentages or use them as weighted-roll entries.

## Personality catalog

The table preserves the package index and raw `keepValue`. Effect wording is a
concise translation of the package description and is cross-checked against the
official article where that article lists the row.

| Index | Package Vietnamese name | English key | Raw `keepValue` | Effect surface | Official article |
|---:|---|---|---:|---|---|
| 0 | Mạnh Mẽ | Strong | 10 | Attack power higher by the parameterized percentage | Listed |
| 1 | Chạy Nhanh | Fast Runner | 10 | Movement speed higher by the parameterized percentage | Listed |
| 2 | Nhanh Nhẹn | Swift | 10 | Attack speed higher by the parameterized percentage | Listed |
| 3 | Yếu ớt | Fragile | 10 | Attack power lower by the parameterized percentage | Listed as Mong Manh |
| 4 | Chậm Chạp | Sluggish | 10 | Movement speed lower by the parameterized percentage | Not listed |
| 5 | Kém nhạy bén | Thickheaded | 10 | Attack speed lower by the parameterized percentage | Not listed |
| 6 | Vụng Về | Careless | 20 | Sells materials to town below the normal price by the parameterized percentage | Listed |
| 7 | Keo kiệt | Stingy | 20 | Sells materials to town above the normal price by the parameterized percentage | Listed |
| 8 | Nhà lãnh đạo | Charismatic | 5 | Raises party Mood by the parameterized number of stages at dungeon start; packaged note says non-stacking | Listed as Nhà Lãnh Đạo |
| 9 | Gánh Nặng | Dead Weight | 5 | Lowers party Mood by the parameterized number of stages at dungeon start; packaged note says non-stacking | Listed |
| 10 | Mắt thâm | Baggy Eyes | 20 | Stamina gauge depletes faster by the parameterized percentage | Listed as Mắt Híp |
| 11 | Năng Động | Energetic | 20 | Stamina gauge depletes slower by the parameterized percentage | Listed |
| 12 | Thừa Cân | Overweight | 20 | Satiety gauge depletes faster by the parameterized percentage | Listed |
| 13 | Gầy Còm | Skinny | 20 | Satiety gauge depletes slower by the parameterized percentage | Listed |
| 14 | Lạc Quan | Optimistic | 20 | Mood gauge depletes slower by the parameterized percentage | Listed |
| 15 | Bi Quan | Pessimistic | 20 | Mood gauge depletes faster by the parameterized percentage | Listed |
| 16 | Nhút Nhát | Coward | 60 | Flees after taking relatively little damage; exact threshold use is unresolved | Listed |
| 17 | Gan Dạ | Fearless | 0 | Never flees due to the normal fear/retreat condition | Listed |
| 18 | Nghiện thuốc | Addict | 0 | Emits special dialogue when consuming a potion | Listed as Bợm Nhậu |
| 19 | Sợ Bệnh Viện | Scared of Hospital | 0 | Plays fearful behavior when going to the infirmary | Listed |
| 20 | Anh Dũng | Heroic | 7 | Attack power, attack speed and movement speed are each higher by the parameterized percentage | Listed |
| 21 | Giàu Có | Rich | 120 | Monster-kill gold uses raw parameter `120`; exact formula is unresolved | Listed |
| 22 | Cờ Bạc | Gambler | 5 | Enhancement success chance higher by the parameterized percentage | Listed as Khéo Tay |
| 23 | Anh Hùng Thép | Man of Steel | 10 | Incoming damage reduced by the parameterized percentage | Listed |
| 24 | Thông Thái | Nimble | 3 | Evasion chance higher by the parameterized percentage | Listed |
| 25 | Lề mề | Laggard | 3 | Evasion chance lower by the parameterized percentage | Listed as Lười Biến |
| 26 | Nhạy Bén | Sharp | 3 | Critical-hit chance higher by the parameterized percentage | Listed |
| 27 | Ngốc Nghếch | Dull | 3 | Critical-hit chance lower by the parameterized percentage | Listed |
| 28 | Bình Thường | Ordinary | 0 | No special numeric effect | Listed |
| 29 | YOLO | YOLO | 0 | Package description says the Hunter behaves recklessly; exact behavior is unresolved | Not listed |
| 30 | Anh hùng bàn phím | Internet Troll | 0 | Package description says “almost invincible”; exact mechanics and availability are unresolved | Not listed |
| 31 | Dục vọng | Naughty | 0 | Dialogue/personality behavior only in the recovered description; exact triggers are unresolved | Not listed |
| 32 | Thô Lỗ | Rude | 0 | Rude dialogue behavior; exact speech routing is unresolved | Not listed |

The official article ends by stating that additional personalities exist beyond
its list. This supports the presence of package rows absent from that article,
but it does not confirm that every hidden row is obtainable in normal Hunter
generation.

## Authoritative data model

Definitions and Hunter assignment must remain separate:

```text
HunterPersonalityDefinition
  release_id
  personality_id
  source_index
  localized_names
  raw_description
  raw_keep_value
  effect_handler_id nullable
  availability_status
  evidence

Hunter
  personality_release_id
  personality_id
```

- Store the package row and raw parameter without interpreting it as a weight.
- Resolve effects through versioned server handlers, never through localized
  text parsing.
- Keep definitions with unresolved formulas readable but non-executable until a
  reviewed rule set exists.
- Keep personality separate from job traits: one is a generated Characteristic;
  the other is an unlockable job-tree progression system.

## Effect integration points

Once formulas are confirmed or adopted as explicit product rules, handlers may
contribute to these server-owned calculations:

- derived ATK, movement speed, attack speed, evasion and critical chance;
- incoming damage calculation;
- stamina, satiety and Mood drain;
- material sale price and monster-gold reward;
- enhancement success chance;
- retreat/fear decisions;
- dungeon-party Mood changes;
- infirmary, potion and speech/animation events.

Effects must be applied exactly once in a documented modifier phase. The client
must not independently apply a personality multiplier to projected statistics.

## Generation contract

The assignment function accepts an explicit ruleset/content release, request
route and deterministic RNG stream, then returns one eligible definition and
auditable roll evidence.

For `web-rebuild-v1`:

1. Use the ordered source-index domain `0..32`.
2. Draw one unbiased integer in that closed domain.
3. Resolve the definition by `(release_id, source_index)`.
4. Persist the chosen personality ID, RNG stream/version and roll audit data in
   the Hunter creation transaction.
5. Do not reroll based on job, gender/body, rarity, request type or generated
   statistics.

This uniform policy is an explicit rebuild rule, not a recovered claim about the
original game. Future event, tutorial or paid-request pools require separate
versioned rulesets and tests. Personalities with unresolved effects may still be
assigned, but their server handler must remain explicitly unavailable or limited
to confirmed cosmetic behavior; the implementation must not invent a combat or
economy formula.

## Original-generation evidence gap

Required evidence before claiming original generation behavior:

1. Eligible personality indices for each Hunter creation/request route.
2. Raw weight for every eligible row or proof of uniform selection.
3. Filter and reroll order, including tutorial and special-request overrides.
4. Relationship, if any, to job, gender/body, rarity and generated stat grade.
5. Confirmation whether special rows such as Internet Troll can be naturally
   generated or are development/event-only content.

Until those items are recovered, UI, telemetry and documentation must label the
uniform distribution as `web-rebuild-v1`, never as the original package rate.

## Sources

- `reverse-engineering/evidence/hunter-generation-tables-v1.json`
- `reverse-engineering/evidence/hunter-info-runtime-schema-android-api30-v1.json`
- `docs/migration/hunter-generation-flow-evidence-v1.md`
- [Official Hunter Personality article](https://evilhunter.zendesk.com/hc/vi/articles/360039887431-T%C3%ADnh-C%C3%A1ch-Th%E1%BB%A3-S%C4%83n)
