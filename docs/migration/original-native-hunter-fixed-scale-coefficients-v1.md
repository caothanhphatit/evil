# Original Hunter Fixed-Scale Coefficients v1

This evidence-only pass classifies three exact `HunterCtrl.getDamage` callers.
It preserves the package literals exactly, including two unexpectedly large
float32 scales. It does not reinterpret those values as percentages, assign
public skill rows, or connect the formulas to live Rust combat.

The deterministic analyzer is
`tools/analyze-original-native-hunter-fixed-scale-coefficients-pass15.py`; its
generated evidence is
`reverse-engineering/evidence/original-native-hunter-fixed-scale-coefficients-pass15.json`.

## Exact caller arithmetic

| Native caller | `getDamage` vector | Coefficient pipeline | Confirmed route |
| --- | --- | --- | --- |
| `JDONOEEBDCD(EvilCtrl)` | `(true,false,false)` | `trunc(float32(baseDamage) * float32(decode(FEATHER_SHOT_POWER_VALUE)) * 0.01f)` | Damage is forwarded in ARM64 `x4` to native target `0x32ff6cc`; its managed identity remains unresolved. |
| `BGIJEDLALGE(EvilCtrl)` | `(true,false,false)` | `trunc(float32(baseDamage) * float32(decode(FEATHER_SHOT_POWER_VALUE)) * 1193.0f)` | Damage is forwarded in ARM64 `x4` to the same unresolved target `0x32ff6cc`. |
| `BGHEAJHAICN(EvilCtrl)` | `(true,true,false)` | Build an `ObscuredFloat` from `float32(decode(DARK_RIFT_POWER_VALUE)) * 1597.0f`, decode it, multiply by `float32(baseDamage)`, then truncate. | `FlameExplosionCtrl.Action`, damage parameter 4, selector parameter 6 = `1`. |

All final conversions use ARM64 `FCVTZS`, so they truncate toward zero. Every
multiplication shown in the table is float32. The Dark Rift caller additionally
performs the captured `ObscuredFloat` initialize/decode round trip before the
base-damage multiplication.

## Package literal proof

The analyzer reads the tracked Evil Hunter Tycoon `1.411` XAPK, opens
`config.arm64_v8a.apk`, then hashes and reads `lib/arm64-v8a/libil2cpp.so`.
The exact values are:

| Module offset | Raw little-endian bytes | float32 |
| --- | --- | --- |
| `0xD2AC8C` | `0ad7233c` | `0.009999999776482582` (`0.01f`) |
| `0xD2A064` | `00209544` | `1193.0` |
| `0xD29EB8` | `00a0c744` | `1597.0` |

The `1193.0f` and `1597.0f` values are not assumed to be typos, encoded
percentages, or array indices. The native instructions load them as float32 and
multiply them into the coefficient chains above.

## Route boundary

Both Feather Shot bodies also call native target `0x27e8964`, with selector
register `w3` equal to `0` or `1`. The computed damage is not passed to that
call. The later `0x32ff6cc` call receives the computed damage in `x4`, so the
first call is recorded only as a presentation/control boundary until its managed
identity is captured.

The API35 emulator was retried for exact managed-target resolution, but the
IL2CPP domain aborted before returning a method record. No failed capture is
committed or used as evidence.

## Remaining blockers

- Resolve managed identities and full parameter contracts for `0x27e8964` and
  `0x32ff6cc`.
- Recover the public skill-row mappings for all three obfuscated callers.
- Explain, from callers or runtime data rather than guesses, why each named
  ConstantData field has both percent-scale and fixed-large-scale siblings.
- Continue classifying the remaining 31 exact caller arithmetic bodies.
