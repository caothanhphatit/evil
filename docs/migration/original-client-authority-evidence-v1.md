# Original Client Authority Evidence v1

Date: 2026-07-26
Package: Evil Hunter Tycoon `1.411` (`26071501`)

## Scope and claim boundary

This report classifies only evidence already present in the supplied package,
the decoded Unity assets, the protected IL2CPP metadata, and the authorized
Android reflection captures. It did not contact the original backend, inspect
credentials, or read raw account values.

`Client-resident` below means that the package contains state or a calculation
boundary capable of participating in that system. It does not mean the method
body or the final authority rule has been recovered. `Remote evidence` means a
route, JSON response boundary, cloud operation, or shared-service model is
present. A route name alone does not prove exactly what the server validates.

## Exact packaged content inventory

The `sharedassets1.assets` QuickSheet block contains the following directly
relevant objects. Each object header records the same packaged spreadsheet
identity and an exact serialized row count.

| Worksheet | Path ID | Rows | Relevance |
|---|---:|---:|---|
| `evil` | 12577 | 195 | Monster definitions; row schema and values are not yet decoded in committed evidence |
| `dropUniqueGear` | 12573 | 61 | Unique-drop definitions; selection formula is not recovered |
| `exp` | 12579 | 100 | Level/difficulty experience definitions, decoded exactly |
| `skill` | 12636 | 10 | Basic skill levels, cooldowns, effect arrays, and study costs, decoded exactly |
| `subJobSkill` | 12637 | 40 | Class-change skill cooldown/effect/study definitions, decoded exactly |
| `inAppReward` | 12600 | 7 | Product reward definitions |
| `innerCastleReward` | 12601 | 20 | Inner Castle reward definitions |
| `riftReward` | 12630 | 30 | Rift reward definitions |
| `riftRaidReward` | 12629 | 20 | Rift Raid reward definitions |
| `honorReward` | 12595 | 14 | Honor reward definitions |
| `fallenRanchReward` | 12580 | 3 | Fallen Ranch reward definitions |
| `moleTimeReward` | 12608 | 20 | Timed reward definitions |

The decoded evidence already records raw and canonical decoded SHA-256 hashes
for `exp`, `skill`, and `subJobSkill`. Definition rows prove content shipped in
the client; they do not prove owned state, outcome selection, or transaction
authority.

## System classification

| System | Client-resident evidence | Remote/server evidence | Supported conclusion |
|---|---|---|---|
| Damage formulas | `HunterData` stores obscured `damage` and `armor`. `GameManager.GetGearDamage` (token `100690456`) and `RandDamage` (`100690719`) are client methods. `DamageManager`, `DamageCtrl`, `HunterCtrl`, and `EvilCtrl` ship as first-party client classes. | No ordinary field-combat damage route is recovered. Server-backed battle modes and result routes exist, but their validation bodies are absent. | Damage calculation has a confirmed client execution boundary. The base formula, defense reduction, random distribution, and whether particular modes verify the result remotely remain unresolved. |
| Critical hit and miss/evasion | `HunterData` stores obscured `critical`, `dodge`, `rankCritical`, and `rankDodge` values with local getters/setters. `RandDamage` confirms a local combat-randomization boundary. | No recovered route or response field proves per-hit crit/miss validation. | Crit/evasion state is client-resident. Trigger probability, multiplier, miss semantics, RNG source, and remote verification are unresolved. |
| Target selection | `HunterCtrl`, `EvilCtrl`, `TargetBuildData`, position, area, state, and target-building fields are packaged/client-resident. | No ordinary hunting target-selection request is recovered. Matchmaking routes exist for separate online modes. | The client has the actors and state needed to choose/display targets, but the actual monster-target algorithm and its authority cannot be claimed from current metadata. |
| HP and death | `HunterData` stores obscured `hp`, `nowHp`, `dead`, `deadAlive`, `revive`, `revivePoint`, state, and multiple revive modifiers, all with local accessors. Death/revive scene controllers and assets also ship locally. | No ordinary field death/HP route is recovered. Shared modes may submit outcomes, but no per-hit server simulation contract is visible. | Hunter HP/death/revive state is confirmed in the local aggregate. Damage-to-HP, death transition, revive timing/cost, and mode-specific remote checks remain unresolved. |
| Mana and cooldown | `SkillData` stores obscured `skillIndex`, `level`, and `coolTime`. Basic and sub-job QuickSheet rows contain exact cooldown/effect definitions. No mana field is present in the captured 109-field `HunterData` schema. | No per-skill cooldown or mana route is recovered. | Cooldown state and definitions are client-resident. A general Hunter mana resource is not evidenced by the captured aggregate; mana cost/regeneration must remain unsupported until a specific type or trace proves it. |
| XP and leveling | `HunterData` stores obscured `exp` and `level`; the package contains 100 exact `exp` rows; `GameManager.GetNeedExp` (token `100690845`) is a local lookup/calculation boundary. | Online mode result submission may include progression effects, but no ordinary farming XP-grant route is recovered. | XP state and required-XP lookup are client-resident. Monster XP award values, modifiers, level-up mutation order, and remote validation are unresolved. |
| Drops | The package contains `evil` and `dropUniqueGear` tables, `DropGearData`, `AdminDropUniqueGearData`, local Hunter gear/item/consumable dictionaries, `UserData.DropGearDataDic`, `GameManager.CheckDropGearInFieldOverLog`, `CheckNewDropGearInStorage`, and `GearAddData`. | Reward/result services exist for online/content-specific modes. No ordinary field-loot grant route is recovered. | Ordinary drop definitions, pending drops, and inventory insertion have strong client-resident evidence. Monster-to-table mapping, rate/RNG, pickup, ownership transfer, and server verification remain unresolved. |
| Skill stats and learned state | Exact packaged basic/sub-job definitions include cooldowns, durations, values, levels, and costs. `HunterData.skill` is `Dictionary<String, SkillData>` and `SkillData` stores local index/level/cooldown state. `GetJobSkillDataIndex` and `GetHunterSkillIcon` are local resolvers. | No skill-learn or per-cast route is recovered in the preserved route fragments. | Definitions and per-Hunter skill snapshot state are client-resident. Handler formulas, casting order, learned-key semantics, and transaction validation are unresolved. |
| Rewards | Multiple reward QuickSheets ship locally. `UserData` stores raid/rift/inner-castle/PvP and other reward flags/lists. `GameManager.CreateRewardData(LitJson.JsonData)` and `SendRewardResult(...)` expose a JSON/result boundary. Route fragments include `game/mail/re...`, `game/inapp...`, adventure save, arena, battle-field, and world-content families. | Mail, in-app, ranked/shared modes, adventure save, and world services have direct remote-route evidence. | Rewards are hybrid. Display tables and claimed-state projections are local, while several economically sensitive reward families have remote response/submission boundaries. The package does not prove that every local farming drop or reward is server-validated. |
| Persistence | `HunterData` and the 527-field `UserData` aggregate contain progression, inventories, rewards, and mode state. `GameManager.Save`, `ThreadSaveString`, and `SaveDelete` are local method boundaries. A rooted authorized capture confirms a private 1.7 MB Unity PlayerPrefs XML with 46 entries and ACTk storage APIs. | `LoadCloud`, `LoadCloudAction`, `OnGoogleSave`, `OverWriteCloudSave`, editor cloud actions, and auth restore route fragments ship in the client. | Persistence is hybrid: a substantial obscured local save is confirmed, with optional cloud save/restore boundaries. Key semantics, serializer/call graph, merge policy, conflict authority, and exact server schema remain unresolved. |

## Remote boundary that is actually recoverable

The protected metadata preserves only partial route strings, including auth and
restore, adventure match/save, arena/PvP, battle-field rank, guild, union, mail
reward, in-app, and world-content families. These are strong evidence that
shared/account/economy services are remote. They are not sufficient to assert
that the production server re-simulates ordinary town combat, chooses every
drop, or validates every XP increment.

The strongest reward boundary is structural rather than formula-level:
`CreateRewardData` accepts `LitJson.JsonData`, while `SendRewardResult` accepts a
reward index and completion callback. Exact endpoint, signature, replay rules,
and grant transaction are not recovered.

## Implementation boundary for the rebuild

The original package evidence indicates substantial client-side simulation and
local persistence, but it is not a safe authority model for a web rebuild. The
rebuild must continue to keep movement decisions, combat RNG, damage, HP/death,
cooldowns, XP, drops, rewards, inventory mutations, and persistence authoritative
on its Rust server. This is an independent security/architecture decision and
must not be presented as a recovered original-server behavior.

## Unresolved capture targets

1. One controlled ordinary hunt before/after trace to correlate target, monster
   HP, Hunter HP, crit/evasion, XP, loot, and saved-state changes.
2. One controlled skill cast trace to correlate `SkillData.coolTime` with the
   actor/UI timer and determine whether any separate resource is consumed.
3. One controlled death/revive trace to recover transition values, timing, cost,
   and persistence.
4. One controlled online reward claim to distinguish response-provided rewards
   from client table lookup without exposing account data or credentials.
5. Native method-body recovery or stable instrumentation for `RandDamage`,
   `GetNeedExp`, drop selection, and skill handlers.

## Reproduction and validation

The committed evidence can be checked without running the game or accessing a
network:

```sh
python3 tools/extract-hunter-info-tables.py
python3 tools/extract-core-economy-tables.py
python3 -m unittest tools.tests.test_hunter_runtime_schema_evidence
jq empty reverse-engineering/evidence/hunter-info-tables-v1.json
jq empty reverse-engineering/evidence/hunter-manager-runtime-schema-android-api30-v1.json
git diff --check
```

The full QuickSheet header inventory in this report was read directly from
`game-assets/extracted/joined_unity_files/sharedassets1.assets` with UnityPy;
no row values were inferred from names.
