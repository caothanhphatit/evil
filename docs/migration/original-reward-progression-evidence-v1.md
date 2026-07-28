# Original Reward And Progression Evidence V1

## Scope

This record covers Evil Hunter Tycoon `1.411` only. It normalizes exact live
API35 IL2CPP method boundaries, focused runtime schemas, and the packaged
QuickSheet EXP/monster catalogs. It does not change the rebuild server or claim
unresolved original formulas.

Primary machine-readable outputs:

- `reverse-engineering/evidence/original-reward-progression-runtime-v1.json`
- `packages/content/releases/evil-hunter-1.411/experience-runtime-catalog.json`
- `reverse-engineering/evidence/original-reward-progression-methods-api35-v1.json`
- `reverse-engineering/evidence/original-reward-progression-helpers-api35-v1.json`
- `reverse-engineering/evidence/original-reward-material-full-api35-v1.json`
- `reverse-engineering/evidence/original-reward-progression-schema-api35-v1.json`

## Exact EXP lookup

`GameManager.GetNeedExp/2` is a 404-byte exact native body. Its arguments are:

1. `HunterData.revive`
2. current Hunter level

The method reads packaged `AdminExpData[currentLevel + 1]`, then selects
`exp1..exp6` using `revive` values `0..5`. Therefore the old description of
this method as a job/class-column lookup is incorrect. The six packaged values
are retained under the neutral evidence name `experienceByDifficulty`; the
native caller proves the selector is `revive`, but does not by itself rename the
product meaning of each revive value.

## Exact PlusExp carry behavior

Recovered pseudocode, omitting the still-partial modifier construction and UI
side effects:

```text
grant = trunc_toward_zero(rawIncomingExp * recoveredMultiplier)

if hunter.level >= globalMaxLevel:
    discard grant
    return

while hunter.level < globalMaxLevel:
    need = GetNeedExp(hunter.revive, hunter.level)
    remaining = grant - (need - hunter.exp)

    if remaining <= 0:
        hunter.exp += grant
        break

    hunter.level += 1
    hunter.exp = 0
    grant = remaining
```

The comparison is strictly `remaining > 0`. Landing exactly on the threshold
stores `exp == need` without increasing the level. A later positive grant
crosses the level. Positive overflow repeats through multiple levels. At the
global maximum level, incoming EXP is discarded rather than accumulated. The
numeric maximum-level constant is not yet named by evidence.

## EXP modifier boundary

The native body confirms a multiplier beginning with positive
`UserData.mBuildingExpUp`, otherwise `1.0`. Confirmed additive participants are
`TimeData.expScroll` (`+1.0`), `TimeData.BoxExp` (`+0.5`),
`UserData.ExpGemPack_Active` (`+1.0`), `StatusData.CostumeExpUp`,
`StatusData.CollectionExpUp`, `HunterData.reviveWisdom`, and a
`StatusData.GearProperty` delta scaled by `0.01`. Final float-to-integer
conversion is ARM64 `fcvtzs`, which truncates toward zero.

This is not yet a complete ordered formula. One singleton Boolean/static float
pair and several event/collection branches remain unnamed. Rust integration
must fail closed for those modifiers instead of silently inventing constants.

## Ordinary material roll

The complete 30,732-byte `HunterCtrl.RewardMetrial/5` body confirms:

```text
for slot in 0 .. materialIndices.length:
    roll = UnityEngine.Random.Range(1, 10001)  // 1..10000
    baseThreshold = materialPercentValues[slot] * 10
    effectiveThreshold = applyRuntimeModifiers(baseThreshold)
    if effectiveThreshold >= roll:
        grant(materialIndices[slot], materialCounts[slot])
```

Slots run in ascending array order. The loop bound is
`materialIndices.length`; therefore monster row `34`'s trailing fourteenth
percent value is outside the ordinary 13-slot loop. `StatusData.CalcHighValueMet`
participates in a confirmed branch that truncates
`(CalcHighValueMet + 1.0) * priorThreshold` when a material property condition
is at least `3`. Earlier global/event modifiers and their order remain pending.

## Captured helpers and unresolved unique gear

Exact bodies are now preserved for:

- `HunterCtrl.GHPHHEFFNKN/2` — 4,236 bytes
- `HunterCtrl.LDHAEMDJCFF/5` — 2,120 bytes
- `GameManager.GetRamblePetGoldUp/1` — 424 bytes
- `GameManager.IsCostumeExpUp/1` — 8 bytes
- `GameManager.GetFairyGoldUp/1` — 128 bytes
- `GameManager.GetNeedExp/2` — 404 bytes
- `GameManager.GetVillTax/0` — 24 bytes

The obfuscated Hunter helpers are intentionally not renamed. Unique gear still
requires mechanical proof for `AdminEvilData.uniqueLevel` to pool linkage,
`dropRange`/`dropCut` ordering and denominator, gear type/index selection, and
the exact RNG call sequence. Gold and village-tax helper capture is sufficient
for the next analysis pass, but not yet for an original-formula Rust port.

## Reproduction

```sh
python3 tools/generate-experience-runtime-catalog.py
python3 tools/analyze-original-reward-progression.py
python3 -m unittest tools.tests.test_original_reward_progression
```
