# Original Native Dodge Consumer Pass 18

This pass supersedes the `CalcDodge` consumer status in hit/miss pass 12. The
Android API 35 package is Evil Hunter Tycoon `1.411`; claims below come from
exact native bodies captured after IL2CPP decryption. Normalized evidence is
`reverse-engineering/evidence/original-native-dodge-consumer-pass18.json`.

## Normal Hunter

`HunterCtrl.DGPHLIIAEFL()` is the recovered normal consumer:

```text
if IsMezeState(): return false
primary = Random.Range(0, 100)
threshold = wrapping_i32(StatusData.CalcDodge + HunterCtrl.EFPJBDACDNH)
if primary < threshold: result = true
else:
    pet = Random.Range(0, 1000)
    result = pet < StatusData.RidingPetDodge
if result: DamageManager.Show(type = 3, ...)
return result
```

The comparison is signed and exclusive. The method contains no explicit cap.
The Meze branch consumes no RNG; a successful primary roll consumes no pet RNG.
`BuffSetting` effect type `5` writes the value payload into
`HunterCtrl.EFPJBDACDNH@0x728`, proving an additive threshold bonus without
proving a public buff name.

`HunterCtrl.Damaged/2` calls this method and branches out before HP processing
when it returns true. `HunterCtrl.EvilDeBuffAction/3` calls it only on an
internal decoded value `51`; that value's public debuff identity remains
unresolved.

## Producer

The common `StatusData.CEOBAMNDIIL()` producer computes:

```text
rawDodge = HunterData.dodge + OptionDodge + PersonalDodge + RankDodge + GUP_Property[8]
StatusData.Dodge = rawDodge
if Dodge <= 0: Dodge = 0
StatusData.CalcDodge = banker_round_to_i32(Dodge)
```

The structurally similar `PFIONPOHHJK()` uses `GUP_Property[7]` and unresolved
static operands in its only direct wrapper (`IEBHFGJHELH`), so it is not merged
into the common village formula.

## Other Modes

- Raid uses the same `[0,100)` plus `[0,1000)` pet fallback with a plain
  `RaidHunterCtrl.EFPJBDACDNH@0x298` additive field.
- World Boss uses `max(EFPJBDACDNH + CalcDodge - decode(NDJAJDFKPFF) -
  trunc(PANOJKHNLEM), 0)`, `Random.Range(0,101)`, then the same pet fallback.
- Fallen Pasture subtracts a decoded field from `CalcDodge + field@0x390` and
  uses `[0,100)` plus the pet fallback.
- Guild Battle and PvP use an additive base, an own subtractor, and an
  optional opponent-owned subtractor before `[0,100)` and the pet fallback.

World Boss subtractor writers, PvP/Guild opponent semantics, and indirect
dispatch for Fallen/PvP remain unresolved. These fields are not projected into
the live rebuild profile.

## Integration Boundary

`apps/server/src/simulation/combat_core/hit_resolution.rs` contains a pure,
RNG-independent resolver and a deterministic contribution calculator. The
rebuild treats `profile.evasion_rate_bps` as total evasion for display and
converts that total to `CalcDodge` with the recovered clamp and ties-to-even
rounding. Missing contribution sources are explicit zero values; callers can
add or remove named sources before recalculating the total.

Live monster attacks now roll the recovered `[0,100)` exclusive threshold and
emit `Evade` before armor, shield and HP processing. The authoritative rebuild
uses its deterministic uniform combat stream because Unity's global PRNG state
sequence is not recovered. Effect type 5 and per-Hunter riding-pet dodge remain
zero until their live state is modeled.
