# Original Gear Creation Writer Boundary v1

The ARM64 API 35 session recovered the managed writer identities and the
`GearData` shape, but not a safe gameplay contract. `GearData` contains four
plus/minus option arrays (including additional arrays) and a stored `buyGold`
field. The writer entry points are `CreateGear`, `SetRandOption`,
`SetRandOptionValue`, `SetAddedRandOption`, and `SetAddedRandOptionValue`.

The captured native method identities do not by themselves establish the
quality pool, option pool, default-option branch, roll order, option enum
meanings, or whether `buyGold` changes with rolled modifiers. The package also
detects the in-process debug hook used for live return-value capture, so no
runtime values are claimed here.

The server therefore rejects gear creation with
`gear_creation_evidence_unresolved` before debiting materials. It must not
generate a synthetic quality/option or advertise rating-only prices as
mod-dependent prices. See
`reverse-engineering/evidence/original-gear-creation-writer-boundary-v1.json`.
