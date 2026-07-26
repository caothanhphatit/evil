# Hunter information serialized binding evidence v1

## Result

The original scene contains a central `ImageManager` MonoBehaviour with exact
serialized sprite arrays for every Hunter-detail content family. This replaces
the earlier filename-only assumption with source PPtr evidence.

The reproducible artifact is
`reverse-engineering/evidence/hunter-info-serialized-bindings-v1.json`, produced
by `tools/extract-hunter-info-serialized-bindings.py`.

## Exact serialized objects

| Object | Level path ID | Script | Raw bytes |
| --- | ---: | --- | ---: |
| Image catalog | 103588 | `ImageManager` | 87,996 |
| Hunter detail controller | 74554 | `HunterDetailPop` | 5,388 |

The controller directly references the five pictured tabs/groups:

- `StatGroup` at PPtr offset 628;
- `SkillGroup` at offset 640;
- `InventoryGroup` at offset 652;
- `GrowUpGroup` at offset 664;
- `RidingPetGroup` at offset 676.

These are direct scene references, not names inferred from screenshots.

## Exact ImageManager arrays

| Array | Count offset | Entries | First | Last |
| --- | ---: | ---: | --- | --- |
| Hunter skills | 31208 | 50 | `skill_h1_01` | `skill_h5_10` |
| Secret Point properties | 73324 | 15 | `growth_ic_00` | `growth_ic_14` |
| Riding-pet portraits | 78376 | 21 | `ride_pet_tb_01` | `ride_pet_tb_20` |
| Riding-pet skills | 78632 | 3 | `rp_skill_01` | `rp_skill_03` |
| Riding-pet traits | 78672 | 6 | `rp_trait_01` | `rp_trait_06` |
| Riding-pet actor thumbnails | 78748 | 21 | `tb_ride_pet_01` | `tb_ride_pet_20` |
| Job Traits | 79364 | 69 | `job_trait_all_01` | `job_trait_h5_s4_04` |

Every count prefix, PPtr file ID, path ID and Sprite name is validated by the
extractor. The arrays have the exact same cardinalities as the decoded
QuickSheet families: 50 total skills, 15 growth properties, 21 pets, 3 pet
skills, 6 pet traits and 69 Job Traits.

## Skill UI references

The `HunterDetailPop` hierarchy contains:

- `FirstSkillGroup/Icon`, whose Image component serializes
  `skill_h1_01`;
- `SecondSkillGroup/Icon`, whose Image component serializes
  `skill_h1_02`;
- two Heroic Skill groups serializing `skill_h1_09` and `skill_h1_10`;
- three Sub Job and three Third Job display groups.

Combined with the supplied original Berserker screenshot and exact QuickSheet
rows, the first two mappings are confirmed:

| Skill row | Name | Sprite |
| ---: | --- | --- |
| 0 | Fury | `skill_h1_01` |
| 1 | War Cry | `skill_h1_02` |

The ImageManager skill array is serialized in four job-major blocks:

1. suffixes `01..02` for H1 through H5;
2. suffixes `03..04` for H1 through H5;
3. suffixes `05..06` for H1 through H5;
4. suffixes `07..08` for H1 through H5;
5. suffixes `09..10` for H1 through H5.

This layout is an exact property of the serialized array. It strongly matches
the Basic, Sub Job, Third Job and Heroic UI slots, but it does not itself prove
the native lookup expression for all 50 QuickSheet rows.

In particular, the 40 `subJobSkill` rows are not stored in the same order as
the 40 icon positions. H5 rows are appended after H1-H4 in QuickSheet, whereas
the sprite array remains job-major. Mapping them using `row index - 90` would
be wrong. No such fallback is permitted.

## Growth, pet and Job Trait status

The following are now source-bound asset pools rather than filename guesses:

- all 15 `growth_ic` Sprites;
- all 21 riding-pet portrait and actor-thumbnail positions;
- all 3 riding-pet skill Sprites;
- all 6 riding-pet trait Sprites;
- all 69 Job Trait Sprites.

Their array positions align one-to-one with contiguous table indices and their
asset names encode the same grouping. This is sufficient to preserve the exact
serialized catalogs. It is still labeled `serialized-position-match`, not
`confirmed-runtime-binding`, because the protected method body that indexes
each array has not been recovered.

Riding-pet position zero and one intentionally share the first Sprite, which
matches the presence of the table's row-zero empty pet. An implementation must
preserve this duplicate instead of normalizing the asset list.

## IL2CPP call-graph result

Protected metadata type index 281 is the strongest `HunterDetailPop`
correlation:

- 194 fields and 71 methods, consistent with the large serialized controller;
- surviving field fragments include `skillTitle`, `skillDesc`, `skillIcon`,
  `inventory`, `expImage`, `mPetTitleText`, `mPetSkillName` and pet-trait
  fields;
- surviving methods include `TabAction` at method index 6312,
  `SkillClick...` at 6320 and `SkillTr...` at 6306.

The native call graph remains unavailable for this build:

- metadata tokens for two of those methods are poisoned and do not have the
  normal `0x06xxxxxx` MethodDef form;
- the stripped ARM64 binary still requires Android relocation/loader repair to
  materialize the Assembly-CSharp method-pointer table;
- current static tooling therefore cannot associate methods 6306/6312/6320
  with trustworthy native VAs or decompile the array-index expression.

This is a narrower blocker than before: the complete arrays and scene slots are
known, but the final table-row selection code is not.

## Migration boundary

Safe without fallback:

- package every Sprite recorded by the serialized arrays;
- use the exact tab and slot hierarchy;
- bind Fury and War Cry to `skill_h1_01` and `skill_h1_02`;
- preserve array positions and duplicate pet position zero/one as evidence
  metadata.

Not yet safe to label original behavior:

- the remaining 48 skill row-to-icon bindings;
- direct growth, pet and Job Trait row lookup solely from matching positions;
- any computed sub-job icon mapping that has not been confirmed by runtime
  tracing or recovered native code.
