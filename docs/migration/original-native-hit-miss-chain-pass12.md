# Original Native Hit/Miss Chain Pass 12

This pass audits the Android API 35 native attack paths for Evil Hunter
Tycoon `1.411`. It is evidence-only. No accuracy, dodge, combat-runtime, or
protocol behavior is connected from these findings.

Normalized evidence:

`reverse-engineering/evidence/original-native-hit-miss-pass12.json`

## Hunter attacking Evil

The normal attack path calls:

```text
HunterCtrl.HuntingAttackAction
  -> HunterCtrl.getDamage(false, false, false)
```

The only integer `Random.Range` inside `getDamage` is exactly:

```text
threshold = min(100, StatusData.CalcCritical + gatedOptionalBonus)
critical = Random.Range(0, 100) < threshold
```

The field load is `StatusData+0xB0`, which is the captured
`CalcCritical` backing field. The selected branch calls
`HunterCtrl.getCriticalDamage` and changes the returned result discriminator
from `1` to `2`. It does not abort damage construction. This is therefore a
critical-selection roll, not an accuracy or hit roll.

`HuntingAttackAction` has five additional integer RNG sites. Their inputs are
rooted in Hunter-owned state, `HunterData`, `StatusData.GearProperty`,
`StatusData.GearSetPropertyValue`, and DataManager tables. None is sourced by
an `EvilData` evasion field. The API 35 `EvilData` schema has no field named
dodge, evasion, accuracy, or hit; the captured `StatusData` schema likewise has
no accuracy/hit field.

Exact boundary: no accuracy-versus-monster-evasion RNG exists in the captured
normal-action and `getDamage` construction path. This does not prove that every
projectile, effect, or skill delivery path always hits; those downstream paths
are still incomplete.

## Evil attacking Hunter

The captured direct normal chain is:

```text
EvilCtrl.FixedUpdate
  -> EvilCtrl.EBNOJHOGEMM
  -> HunterCtrl.Damaged
```

`EBNOJHOGEMM` has one pre-damage integer gate:

```text
if EvilCtrl.OCLFGGEJKMI@0x1E8 >= 1:
    proc = Random.Range(0, 100) < OCLFGGEJKMI
```

When the comparison succeeds, native code routes to `DamageManager.Show` and
returns before `HunterCtrl.Damaged`. Otherwise it computes the incoming value
and calls `HunterCtrl.Damaged`. Existing writer evidence binds effect type 54
to this Evil-owned field, but does not provide a public gameplay name for the
effect. Calling it blind, miss, accuracy, dodge, or evasion would still be a
semantic guess.

The alternate direct caller `EvilCtrl.OFEIPNBMNML` calls
`HunterCtrl.Damaged` twice and contains no integer `Random.Range`. The four
integer RNG sites inside `HunterCtrl.Damaged` occur after damage entry and are
Hunter gear/effect procs already rejected as direct dodge checks.

`StatusData.CalcDodge` is present at `0xC0`, but it is not read by the captured
direct Evil chain above. Pass 18 recovered its separate global consumers and
common producer; see `original-native-dodge-consumer-pass18.md`.

## Integration boundary

Do not add a generic accuracy-versus-dodge formula from these captures. The
safe conclusions are narrower:

- the `getDamage` percentage roll is critical selection;
- the direct Evil normal path has an attacker-owned effect-54 abort gate;
- the direct Evil path does not prove a `CalcDodge`-based hit test; the separate
  Hunter damage-intake path now does;
- target-specific and indirect delivery paths still require native evidence.

Validation:

```sh
python3 tools/analyze-original-native-hit-miss-pass12.py
python3 -m unittest tools.tests.test_original_native_hit_miss_pass12
```
