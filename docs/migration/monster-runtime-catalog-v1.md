# Monster runtime catalog v1

## Scope

`packages/content/releases/evil-hunter-1.411/monster-runtime-catalog.json` is a generated, evidence-only normalization of the packaged monster and unique-gear tables. It is data/runtime documentation, not a UI contract.

The catalog contains:

- 195 monster rows, preserving packaged indices `0..194`;
- 75 sorted `(area,type,createLevel)` groups;
- 18 composite-key groups with more than one monster row;
- 61 unique-gear pools, preserving packaged indices `0..60`.

The composite monster key is intentionally one-to-many. Consumers must not collapse a group to its first row.

## Exact static fields

Every monster row preserves the decoded `uniqueLevel`, `race`, `hp`, `damage`, `armor`, `experience`, `gold`, `materialIndices`, `materialCounts`, and `materialPercentValues` values.

Every unique-gear pool preserves `dropRange`, `dropCut`, `gearTypes`, `gearIndices`, and `gearPercentValues`. These arrays are exact packaged values; the catalog does not infer the unresolved unique-gear selection algorithm or the `uniqueLevel`-to-pool binding.

The runtime field names and types are checked against `AdminEvilData` and `AdminDropUniqueGearData` in `reverse-engineering/evidence/evil-ai-drop-runtime-schema-android-api35-v1.json` before generation.

## Material roll control flow

The primary ordinary-material loop in `HunterCtrl.RewardMetrial/5` is recoverable from the captured ARM64 prefix:

| Address | Evidence |
| --- | --- |
| `0x73ab225fe4` | Reads `materialIndices.length` as the loop bound. |
| `0x73ab225ff0` | Sets integer RNG minimum to `1`. |
| `0x73ab225ff4` | Sets integer RNG exclusive maximum to `10001`. |
| `0x73ab225ffc` | Calls `UnityEngine.Random.Range(Int32,Int32)`. |
| `0x73ab22603c` | Loads the parallel `materialPercentValues` array. |
| `0x73ab226064` | Begins exact integer scaling `raw + raw * 4`. |
| `0x73ab226068` | Doubles the intermediate value, producing `raw * 10`. |
| `0x73ab2268a8` | Compares the effective threshold with the saved roll. |
| `0x73ab2268ac` | Skips the grant when `effectiveThreshold < roll`. |
| `0x73ab2268e0` | Loads `materialCounts[slot]`. |
| `0x73ab2269b4` | Loads `materialIndices[slot]`. |
| `0x73ab226a0c` | Increments the slot index by one. |

The called RNG overload is independently captured in `reverse-engineering/evidence/unity-random-range-native-methods-android-api35-v1.json`: token `100665478`, address `0x73ad876240`, module offset `0x5a76240`, parameters `(System.Int32,System.Int32)`.

Therefore the exact base behavior is:

```text
for slot in 0 .. materialIndices.length:
    roll = Random.Range(1, 10001)       // outcomes 1..10000
    baseThreshold = materialPercentValues[slot] * 10
    effectiveThreshold = applyRuntimeModifiers(baseThreshold)
    if effectiveThreshold >= roll:
        grant materialIndices[slot], materialCounts[slot]
```

The raw `materialPercentValues` denominator is `1000`: before modifiers, one raw unit becomes ten successful outcomes out of 10,000. The effective roll denominator is `10000`, and the success comparison is inclusive.

Runtime hunter/global modifiers are present between base scaling and comparison. Their complete formula is not yet decoded, so the catalog records it as `null` and must not be used to invent a modifier implementation.

## Packaged anomaly

Monster index `34` has array lengths `13/13/14` for indices/counts/percent values. The primary loop is bounded by `materialIndices.length`, so it iterates 13 slots. The trailing fourteenth percent value is packaged but unused by this loop.

## Unresolved semantics

The following stay explicit `null`:

- unique-gear selection order;
- unique-gear chance denominator/application;
- `uniqueLevel`-to-unique-pool binding;
- complete ordinary-material modifier formula.

No rate or linkage should be ported until native control flow and controlled runtime outcomes agree.

## Regeneration

Run:

```sh
python3 tools/generate-monster-runtime-catalog.py
python3 -m unittest tools.tests.test_monster_runtime_catalog
```

The generator hashes all four evidence inputs into the output and validates the critical runtime schema, method tokens, RNG overload, and ARM64 instructions before emitting the catalog.
