# Original Native Hunter getDamage Producers v4

Pass 10 traces the final SSA registers back through the complete 9,496-byte
body. It records mechanical gates only and does not assign product labels to
obfuscated fields.

- `D10` is the base accumulator: `D8 * (1 + S9_early)`, with another optional
  percent multiplier in a later branch.
- `S12`, `S13`, and `D14` are additive modifier accumulators consumed as
  `(1+S12)`, `(1+S13)`, `(1+D14)`.
- `D11` defaults to `1`; on the target-gated path it becomes the sum of the
  Slayer and Rift helpers.
- stack slot `+0xC` defaults to `1` and can become
  `1 + GearSetPropertyValue[4][1]*0.01` after GearSetProperty and HunterData
  ratio gates.
- `S8` defaults to `1` and collects GearSetProperty, GearProperty 67, and an
  opaque job-trait-21 branch.
- final `S9` is selected by `HunterData.job`; it uses the matching Collection
  and Relic class-damage fields for jobs 0 through 4.
- `S15` defaults to `1` and receives `getCriticalDamage` only through the
  successful critical/target gate.

Boolean mechanics are separate: argument 1 selects an early base path;
argument 2 bypasses the critical roll when true; argument 3 bypasses the
target-object critical/Slayer/Rift additions when true. Their gameplay-facing
names remain unresolved.

The analyzer and normalized evidence retain exact opaque HunterCtrl field names
and offsets. No live combat integration is made.

```sh
python3 tools/analyze-original-native-hunter-get-damage-producers-pass10.py
python3 -m unittest tools.tests.test_analyze_original_native_hunter_get_damage_producers_pass10
```
