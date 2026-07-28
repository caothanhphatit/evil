# Original Native Hunter Outgoing Damage SSA v2

Pass 7 verifies exact ARM64 instruction offsets in the complete API35 bodies;
it remains evidence-only and does not modify live combat.

`getDamage` finishes with this exact SSA chain:

```text
A = D10 * (1 + float64(S12))
B = A * (1 + float64(S13))
C = B * (1 + D14)
D = C * D11
E = D * float64(S15)
F = E * float64(stackFloat@0xC)
G = F * float64(S8)
H = G * float64(S9)
result = truncTowardZero(H)
```

Those register producers are not all semantically closed, so this is not yet a
portable formula.

`getCriticalDamage` starts at `1.75`, then conditionally adds the positive
collection, relic, village-pet, riding-pet, Sylph and heroic-trait critical
damage values. Collection/relic and `UserData+0xA14` are scaled by `0.01`;
village-pet and the decoded obscured fields are added directly. Three direct
HunterCtrl inputs remain obfuscated: `BDDEONCMGHK@0x7FC`,
`FBNMALOOBKK@0x810`, and `AKBENLLFPCC@0x854`.

The temporary GearProperty contribution is exact without assigning semantics:

- index 43 can set `T = current + (element0-element1)*0.01`;
- index 59 can do the same only when `mAdminEvilData[input].race == 1`;
- index 14 clamps `T` to the runtime float32 literal `1.8` when enabled;
- the return is `critical accumulator + T`.

The cap was read at `libil2cpp+0xD2ABAC`: raw `6666e63f`, float32
`1.7999999523162842`. The method argument indexes the admin-evil array, but its
gameplay label remains unresolved.

Run:

```sh
python3 tools/analyze-original-native-hunter-outgoing-damage-pass7.py
python3 -m unittest tools.tests.test_analyze_original_native_hunter_outgoing_damage_pass7
```

Still fail-closed: register producer meanings in the final getDamage chain,
opaque-field writers, skill coefficient/caller vectors, and the monster
armor/minimum-damage consumer.
