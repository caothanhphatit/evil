# Localization Evidence v1

The four recovered locale Addressables bundles contain one directly decodable table: `IOS_DisplayNameTable`. Its shared data has three IDs and all four locale tables cover the same IDs:

| ID | English key/value | Locales |
| ---: | --- | ---: |
| 637734477824 | `Evil Hunter Tycoon` | 4/4 |
| 1725179092992 | `The accessed data is secure and used only for Ad targeting.` | 4/4 |
| 137974709956370432 | `Android Display Name` | 4/4 |

The machine-readable artifact is `reverse-engineering/evidence/localization-evidence-v1.json`. It pins every source bundle by byte length and SHA-256 and preserves the localized values without normalizing or translating them.

This is only display-name/consent evidence, not the game's complete string-table corpus. Font glyph coverage, fallback chains, rich-text behavior, and runtime locale selection remain unresolved. `Admin*Data` class names in the IL2CPP/MonoScript catalog are type evidence only; no economy, roster, building, quest, equipment, or progression rows are promoted from them without serialized rows or observed values.
