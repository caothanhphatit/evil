# Original Native Hunter Outgoing Helpers v3

Pass 8 analyzes the complete API35 bodies of `getSlayerDamage` (3,512 bytes)
and `getRiftNpcBuffDamage` (592 bytes). It is evidence-only.

`getSlayerDamage` begins at zero and can add positive
`RidingPetSlayerDemUp`. It indexes `DataManager.mAdminEvilData` with the method
argument and branches on the decoded `race` field. The exact branch mapping is:

| race value | GearProperty index | named StatusData additions |
|---|---:|---|
| 1 | 11 | Collection/Relic `PrimateDem` |
| 2 | 13 | Collection/Relic `UndeadDem` |
| 3 | 12 | Collection/Relic `EvilDem` |
| 4 | 46 | Collection/Relic `AnimalDem` |
| 5 | 45 | Collection/Relic `BossDem` |

Each GearProperty branch adds `(element0-element1)*0.01`. Job-trait 21,
`UserData+0xB78/+0xB80`, and helper `0x2FA2D94` introduce additional terms;
their semantics remain opaque and the method is not portable yet.

`getRiftNpcBuffDamage` starts at zero. It proceeds only when the decoded input
equals one of two GameManager static integers (`+0xC10`, `+0xC30`), then follows
a gated lookup rooted at `UserData+0xCF8`. A found nested integer contributes:

```text
result += value * 0.0001f
```

The scale is the captured literal at `libil2cpp+0xD2B6D0`, raw `17b7d138`.
Dictionary/key meanings and the two static integer labels are unresolved.

```sh
python3 tools/analyze-original-native-hunter-outgoing-helpers-pass8.py
python3 -m unittest tools.tests.test_analyze_original_native_hunter_outgoing_helpers_pass8
```
