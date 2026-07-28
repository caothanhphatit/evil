# Evil Hunter Migration Knowledge Transfer

Last updated: 2026-07-28
Baseline commit: `41fae603e23f298cafb9a18b6dd45f1f491cb707`

## Mission

This repository is an educational rebuild and migration study of the supplied
Evil Hunter Tycoon `1.411` package. The implementation must be driven by
recoverable source evidence, serialized Unity data, controlled runtime captures,
and user-provided screenshots. Never fill an evidence gap with a plausible
mapping or silent fallback.

Implementation-facing feature descriptions live under `docs/game-design/`.
They distinguish package-confirmed evidence, official public references,
user-supplied raw tables, and unresolved rules.

## Repository map

- `apps/web/`: Vite/TypeScript game client and town/building/Hunter UI.
- `apps/server/`: Rust server, session handling, simulation, products, Hunter
  roster, trading post, and building services.
- `infra/db/migrations/`: relational schema and seed migrations through `0023`.
- `packages/content/`: generated schemas and normalized runtime catalogs.
- `game-assets/normalized/`: normalized assets used by the rebuild.
- `reverse-engineering/evidence/`: machine-readable extracted evidence.
- `docs/migration/`: evidence boundaries, UI audits, and migration reports.
- `tools/`: deterministic extractors, validators, generators, and runtime tools.

Large package/native inputs are already tracked with Git LFS. After cloning:

```sh
git lfs install
git lfs pull
```

Do not commit `target/`, `node_modules/`, `dist/`, Python caches, credentials, or
new raw study inputs unless their inclusion and LFS treatment are deliberate.

## Current implementation

- Town projection, camera/depth handling, building placement, normalized base
  building versus skin data, and visible-world packaging are implemented.
- Building registries, conditions, product stock, crafting/service routes,
  trading post, blacksmith/gear shop, potion route separation, and related DB
  migrations exist. UI fidelity is still an iterative migration, not proof that
  every building matches the original behavior.
- A demo Hunter roster and modular Hunter appearance projection exist across DB,
  server, and web layers.
- Hunter Info has Status, Skills, Materials/Inventory, Growth, and Riding Pet
  projections. Missing per-Hunter evidence is intentionally shown as unavailable
  rather than synthesized.

The exact original Hunter Detail tab dispatch is:

1. Status
2. Skills
3. Inventory/Materials
4. Growth
5. Riding Pet

If the client order differs, correct it only while preserving the evidence
boundary documented below.

## Strong Hunter evidence

Read these before touching Hunter generation or Hunter Info:

- `docs/migration/hunter-info-data-audit-v1.md`
- `docs/migration/hunter-detail-scene-object-graph-v1.md`
- `docs/migration/hunter-info-binding-evidence-v1.md`
- `docs/migration/android-save-runtime-audit-v1.md`
- `docs/migration/hunter-generation-flow-evidence-v1.md`
- `docs/migration/hunter-runtime-schema-capture-v1.md`
- `docs/migration/hunter-static-gameplay-evidence-v1.md`
- `reverse-engineering/evidence/hunter-info-serialized-bindings-v1.json`
- `reverse-engineering/evidence/hunter-save-serialization-v1.json`
- `reverse-engineering/evidence/hunter-info-runtime-schema-android-api30-v1.json`
- `reverse-engineering/evidence/hunter-info-runtime-schema-android-api35-v1.json`
- `reverse-engineering/evidence/hunter-domain-runtime-schema-android-api30-v1.json`
- `reverse-engineering/evidence/hunter-manager-runtime-schema-android-api30-v1.json`
- `reverse-engineering/evidence/hunter-collection-runtime-schema-android-api30-v1.json`

Confirmed model boundaries:

- `UserData` is the large aggregate: 527 fields.
- `HunterData` is the primary Hunter snapshot candidate: 109 fields.
- `HunterLookData` is the appearance projection candidate: 11 fields.
- `SaveData` is a small wrapper and is not the complete player snapshot.
- Runtime reflection corrects the protected static result: `SaveData` has four
  fields (`index`, `data`, `action`, `clear`), not one.
- `GameManager` owns separate active and waiting `HunterDataDic` values.
- `HunterDataDic.data` is `Dictionary<String, HunterData>` and its riding-pet
  map is `Dictionary<String, RidingPetData>`.
- Exact runtime field names, types, offsets, method signatures, and tokens are
  captured for the Hunter, nested skill/item/gear/pet, manager, and collection
  boundaries.
- Equipment slots: Gloves, Helmet, Necklace, Boots, Ring, Weapon, Armor, Belt.
- Scene assets expose 50 skill icons, 15 growth assets, 21 pet portraits, 21 pet
  actor thumbnails, 3 pet-skill icons, 6 pet-trait icons, and 69 job-trait icons.
- Packaged QuickSheet catalogs enumerate 10 basic skills, 40 sub-job skills, 15
  growth properties, 69 job traits, 21 riding pets, 3 pet skills, 6 pet traits,
  100 experience rows, 671 gear definitions, 369 materials, 5 consumables, 61
  runes, and 10 rune-craft definitions. These are definitions, not proof of a
  particular Hunter's owned state or calculation result.
- Only Fury to `skill_h1_01` and War Cry to `skill_h1_02` are currently confirmed
  exact skill bindings. Do not bind the other skill icons by array index.
- API 35 runtime resource inventory confirms the installed split set and local
  Addressables catalog (`2.8.1`). Runtime external storage duplicates
  `global-metadata.dat` byte-for-byte; only Unity version metadata and opaque
  shader cache are newly observable. No additional gameplay AssetBundle or
  StreamingAssets payload appeared outside the APK. See
  `docs/migration/android-api35-runtime-resource-inventory-v1.md` and
  `reverse-engineering/evidence/android-api35-runtime-resource-inventory-v1.json`.
- The BE now has an explicit `web-rebuild-v1-fixture` Hunter flow: pinned-zone
  hunting advances on server ticks, deterministic loot returns and sells with
  town-wallet conservation, H1-only confirmed skill learning, and death/revive.
  Hunt state and command fingerprints persist through PostgreSQL migration
  `0018_hunter_flow_v1.sql`; this does not resolve original formulas, costs, or
  RNG. See `docs/migration/hunter-flow-be-v1.md`.

Still unresolved without runtime evidence:

- The remaining skill row-to-icon and per-Hunter learned-state bindings.
- Runtime Inventory/Materials rows and quantities.
- Growth node IDs, costs, effects, and learned state.
- Riding Pet ownership, skill, trait, and gear values per Hunter.
- The serializer call graph and exact local save encoding.
- Live dictionary-key meanings and per-Hunter runtime values.
- Original formula/RNG bodies for stat generation, gear rolls, enhancement, rune
  creation, and trait effects remain unresolved; see
  `docs/migration/hunter-static-gameplay-evidence-v1.md`.

Recovered native combat increment:

- API 35 memory capture now maps Assembly-CSharp method tokens to decrypted
  native bodies through the IL2CPP code-registration table.
- `GameManager.RandDamage` is exactly recovered as a wrapping 30-entry fixed
  multiplier stream; the Rust evidence policy stores it as integer hundredths.
- `GameManager.GetNeedExp` reads packaged row `currentLevel + 1` and selects
  `exp1..exp6` with `HunterData.revive`; it is not a job-column lookup.
  `GetGearDamage` is decrypted and its structural helper/fields are now
  resolved, but its caller adjustment and the wider aggregation chain remain
  unresolved, so fixture combat must not claim the complete original stat yet.
- Evidence: `reverse-engineering/evidence/original-native-combat-runtime-v1.json`.

## Runtime capture on Mac

Use a physical, authorized Android ARM64 test device where possible. The package
is ARM64-only; emulating it on a small x86 host is unnecessarily expensive.

- Guide: `docs/migration/hunter-info-runtime-capture-macos.md`
- Frida script: `tools/runtime/hunter-info-runtime-dump.js`

The current dumper waits for `libil2cpp.so`, attaches to the IL2CPP domain, and
uses exported reflection APIs to emit fields, types, offsets, methods, and tokens
for `HunterData`, `HunterLookData`, `UserData`, `SaveData`, and
`HunterDetailPop`. It retries while the IL2CPP domain initializes and accepts
an explicit target-type list through
`tools/runtime/capture-hunter-info-schema.py`. It deliberately does not read
arbitrary managed objects or claim save-value bindings.

Android 11 and Android 15 ARM64 emulator schema captures are complete and their
normalized primary Hunter class payloads match. The API 30 app returns to the
launcher shortly after startup without Frida. The API 35 app reaches a tutorial
town and displays Hunter `Sharon` during a clean run, but exits with Android
status `SIGNALED`/`9` about ten seconds after the successful schema-only attach.
No Java/native fatal or ANR was logged. No live Hunter value was read.
Controlled Hunter-value capture remains pending and should use an authorized
ARM64 session that stays stable after attachment.

When returning captured output to another agent, include package version,
package ID, device ABI, Frida client/server versions, UTC timestamp, exact user
action, and both before/after captures. A field-name resemblance is not proof of
a UI binding.

## Development commands

```sh
pnpm install
pnpm test:web
pnpm build:web
cargo test --manifest-path apps/server/Cargo.toml
pnpm test:assets
pnpm building:validate
python3 -m unittest tools.tests.test_scene_evidence
```

Run validation in proportion to the change. Lightweight mining-only changes
should at least pass `git diff --check`, relevant Python compilation/tests, and
`node --check` for changed JavaScript. Do not regenerate large catalogs unless
the source evidence or generator changed.

## Handoff checklist

Before reporting work complete:

1. State exactly what evidence supports each new mapping or behavior.
2. List unresolved fields explicitly; do not hide them behind default values.
3. Keep DB migrations, server projections, protocol, and FE models consistent.
4. Record newly generated evidence and the deterministic command that created it.
5. Report tests actually run and any tests skipped.
6. Check `git status`, large-file handling, and secrets before committing.

## 2026-07-26 mining additions

- `reverse-engineering/evidence/zendesk-hunter-skills-v1.json` captures the
  official Vietnamese skill trees for five jobs, ten Hero branches and 160
  published nodes, plus the Hunter skill-learning workflow article. Its
  package comparison identifies ten Hero-root definition matches and leaves
  the remaining node/icon mappings unresolved.
- `docs/migration/zendesk-hunter-skill-catalog-v1.md` records the source/version
  boundary and implementation policy for those public trees.
- `docs/game-design/hunter-skills-official-reference.md` is the concise
  implementation-facing index; full node text and numeric expressions remain
  in the evidence JSON.
- `docs/migration/hunter-content-catalog-coverage-v1.md` inventories the exact
  package catalogs for skills, growth, traits, pets, gear, materials, runes and
  their remaining runtime-capture gaps.
- `docs/game-design/hunter-personality-system.md` records all 33 packaged
  personality rows. The rebuild product rule is uniform `1/33` assignment; this
  is explicitly not claimed as the original game's recovered RNG.
- Rooted API35 access now confirms a private 1.7 MB ACTk-backed PlayerPrefs XML
  with 46 entries. Sanitized inventory and ACTk schema evidence are stored in
  `android-api35-private-playerprefs-inventory-v1.json` and
  `android-api35-actk-storage-schema-v1.json`; raw keys, values and account data
  are deliberately not committed.
- Protocol v17 now contains the vertical Hunter hunt flow: server-owned hunt
  ticks, fixture-zone assignment, return, loot sale, revival, and the two
  confirmed H1 skill-learning bindings. Hunt state and action-command keys are
  persisted through migration `0018_hunter_flow_v1.sql`; `migration-zone-1` is
  an explicit rebuild fixture, not an original zone mapping.
- Protocol v18 separates three fixture hunting regions from their wooden
  density boards. Its current `enter_monster_map` name is a migration misnomer:
  town and all three hunting regions are one village world instance, and this
  intent changes camera/interaction focus rather than replacing the world or
  its actor roster. Each board changes only its region's durable density
  I/II/III; monster runtime remains server-authoritative and ephemeral, while
  world difficulty stays global.
- An ACTk semantic-key matching attempt was run against the rooted API35 save.
  It produced zero matches from 39 candidate names, so no PlayerPrefs entry is
  claimed as `UserData` or `HunterData`; the sanitized attempt is recorded in
  `android-api35-actk-semantic-key-match-v1.json` and the pause/save trace in
  `android-api35-actk-playerprefs-pause-key-trace-v1.json`.
- The packaged monster catalog is now normalized at
  `packages/content/releases/evil-hunter-1.411/monster-runtime-catalog.json`:
  195 monster rows, 75 `(area,type,createLevel)` groups, and 61 exact unique
  gear pools. The ordinary material loop rolls each source slot independently
  with `Range(1,10001)`, uses `rawPercent * 10`, and grants when the effective
  threshold is greater than or equal to the roll. The complete modifier chain
  and unique-gear linkage/order remain unresolved; see
  `docs/migration/monster-runtime-catalog-v1.md`.
- Live-decrypted native AI evidence now covers the principal Hunter and monster
  movement/attack methods. Both actors use queued FSM boundaries rather than
  the current fixture's modulo-tick movement. Exact method hashes, normalized
  disassembly, confirmed calls/fields, and unresolved obfuscated semantics are
  recorded in `docs/migration/original-native-ai-evidence-v1.md` and
  `reverse-engineering/evidence/original-native-ai-runtime-v1.json`.
- The FE now renders town and every hunting region through one persistent Pixi
  scene graph. The obsolete split village/field layers and floating HTML density
  panel were removed. Exact `sign_01..03` Unity sprites, transforms, colliders,
  and I/II/III states are packaged into the visible-world release; clicking a
  board sends `set_monster_region_density(region_id, level)` and mutates only
  that region without changing world ownership or camera focus.
- The basic shared-world Hunter/monster loop now uses typed server-owned states
  for region entry, target acquisition, chase, attack, loot collection, death,
  patrol, and respawn. Ordinary monster HP/damage/armor/EXP/gold/material rows
  are loaded from the exact `1.411` mapping; unresolved movement/combat timing,
  damage compatibility, density counts, revival position, and level-threshold
  continuation remain explicitly named temporary tuning. See
  `docs/migration/hunter-monster-runtime-implementation-v1.md`.
- Clicking a world Hunter now opens the screenshot-reconstructed command flow;
  `Di Chuyển` sends an authoritative assignment for `Thuộc Địa`, `Tử Địa`, or
  `Ma Giới`. The exact region order and locale evidence are recorded in
  `docs/migration/original-hunter-movement-animation-revival-evidence-v1.md`.
- The monster-material supply catalog now cross-links all `179` unique
  monster-droppable materials, `1,617` exact drop slots, `5,797` recovered
  recipe inputs, and `132` building cost slots. Every droppable material has a
  recovered Trading Post unit price and `sell_hunter_loot` settles arbitrary
  catalog-backed material stacks with town-wallet conservation and one durable
  settlement line per material. Exact gates remain unresolved for `2,810`
  recipes and are not synthesized. See
  `docs/migration/monster-material-market-catalog-v1.md`.
- Packaged Hunter weapon presentation is now mechanically inventoried in
  `hunter-weapon-attack-presentation-v1.json`: weapons are Spine skin
  attachments on the `sword`, `hammer`, `bow`, `wand`, `spear`, and secondary
  weapon slots; basic attacks have exact 0.3333-second front/back clip pairs.
  `mon_a_01_1` similarly has exact `atk`/`atk_b` and `walk`/`walk_b` pairs.
  Exact gear-index-to-skin and native target-axis facing rules remain
  unresolved; see
  `docs/migration/original-hunter-weapon-attack-presentation-evidence-v1.md`.
- Packaged Hunter skill use is now bounded in
  `hunter-skill-use-runtime-v1.json`: ten basic and 40 sub-job definitions are
  preserved with their exact content parameters; per-Hunter `SkillData` stores
  index, skill index, cooldown, and level; native `HuntingAttackAction()`
  directly reaches confirmed melee, ranged, trait, familiar, and effect
  helpers. Exact skill-to-animation/icon/effect bindings and native branch
  conditions remain unresolved; see
  `docs/migration/original-hunter-skill-use-evidence-v1.md`.
- Protocol v23 adds `use_hunter_skill`. The rebuild exposes all ten packaged
  basic skills with exact base-job ownership and catalog cooldowns after
  server-side learned-state, target-range, and readiness validation. Exact
  effect formulas and skill-specific animation/effect bindings remain
  unavailable; activation advances the matching class presentation without
  synthesizing combat outcomes.
- A second native combat capture now preserves sixteen live API35 method bodies
  spanning Hunter damage/critical, monster damage/reduction and gear
  damage/armor/accessory modifiers. `EvilCtrl.GetReduceAttackValue` is recovered
  as the exact multiplicative stack `(1-a)*(1-b)*(1-c)` over fields at offsets
  `0x1E4`, `0x1EC`, and `0x1F4`; the obfuscated field writers are still being
  resolved before Rust integration. See
  `docs/migration/original-native-combat-formula-recovery-v2.md`.
- The complete API35 `EvilCtrl` native-method set now resolves the three attack-
  reduction writers further: effect type `8` writes `0x1E4`, effect type `55`
  writes `0x1EC`, and a still-obfuscated GameManager effect identifier writes
  `0x1F4`; all three convert integer values with float32 `0.01`. Matching end
  branches clear the same slots. One complete consumer truncates toward zero
  after applying the multiplicative stack, but its gameplay role and the third
  effect identifier remain unresolved, so the formula is not yet connected to
  Rust combat. See
  `docs/migration/original-native-evil-attack-reduction-v1.md`.
- Exact cadence capture confirms monster `UnitAttack` writes
  `0.08 * max(field_572, 1)` and Hunter `HuntingAttackAction` writes
  `AttackAniTime = composite > 1 ? 0.333 / composite : 0.7`, where the
  composite is a float multiplied by a decoded `ObscuredFloat`. The equations
  and FSM reset fields are exact, but the factor writers remain obfuscated, so
  current fixed recovery ticks have not yet been replaced. See
  `docs/migration/original-native-combat-cadence-stat-chain-v1.md`.
- `apps/server/src/simulation/original_combat.rs` has disconnected, unit-tested
  reference functions for the confirmed critical base, cadence branches and
  three-slot attack-reduction arithmetic. These helpers deliberately do not
  affect live combat until their obfuscated input writers and caller semantics
  are proven.
- Hunter critical chance now has an exact core branch:
  `threshold = min(100, CalcCritical + enabledBonus)`, Unity rolls
  `Random.Range(0,100)`, and critical succeeds only when `roll < threshold`.
  The conditional bonus writer and surrounding attack gates are unresolved;
  the Rust reference helper is tested but disconnected from live combat.
- Ordinary Hunter and monster HP bars are now recovered from their serialized
  `sharedassets1.assets` prefabs and live-decrypted initializers. Both use
  `hp_in` plus `hp_bg`, scale the fill by authoritative current/max HP, and use
  exact native thresholds: below 20% red, 20%-49.999% orange, and 50%+ green.
  Protocol v21 projects nullable current/maximum HP; the Pixi world renders an
  empty framed bar at zero HP. Hunter level-badge text, shields, and a complete
  post-death visibility audit remain unresolved. See
  `docs/migration/original-actor-health-bar-presentation-evidence-v1.md` and
  `reverse-engineering/evidence/original-actor-health-bar-presentation-v1.json`.
- Exact reward/progression evidence now recovers strict EXP carry semantics:
  landing exactly on required EXP does not level, positive overflow carries
  through multiple levels, and grants at the global maximum level are discarded.
  The ordinary material loop is exact (`Range(1,10001)`, slot order, `percent *
  10`, inclusive threshold), while full modifier order and unique-gear linkage
  remain unresolved. See `docs/migration/original-reward-progression-evidence-v1.md`.
- Pass-2 native control-flow evidence fixes the exact `Reward` mutation order,
  `RewardMetrial` helper-call counts, and RNG range families. It deliberately
  leaves unique-level pool linkage and dynamic semantic branches unresolved;
  see `docs/migration/original-reward-progression-control-flow-v2.md`.
- Pass 3 preserves complete native arithmetic order for EXP/gold/tax and binds
  exact `AdminEvilData` material/type row accesses. It still finds no safe
  `uniqueLevel -> AdminDropUniqueGearData` object-flow binding, so pool linkage,
  cut/percent evaluation and gear selection remain fail-closed. See
  `docs/migration/original-reward-progression-arithmetic-v3.md`.
- `apps/server/src/simulation/original_progression.rs` now parses the generated
  100-row EXP catalog and unit-tests the exact lookup/carry behavior. It remains
  disconnected from live rewards because the complete EXP multiplier order is
  not yet resolved.
- Reward/progression pass 4 resolves the live `1.411` stored Hunter level cap as
  `99` from the exact `PlusExp` static `ObscuredInt`; the same
  method displays `HunterData.level + 1`, so this path's displayed cap is `100`.
  Normal `GetNeedExp` calls therefore accept stored levels `0..98` and consume
  packaged rows `1..99`;
  row `0` is unused by this path. The separate `75/100/125` branch is gated by
  stage level, revive `5`, Hunter level `99`, and flows toward
  `mBuildingSoulUp`; those constants are not level caps. The static holder's
  product-facing name and remaining EXP/gold operands stay unresolved; see
  `docs/migration/original-reward-progression-level-domain-v4.md`.
- Reward/progression pass 5 orders every currently named `PlusExp` accumulator
  input from building, scroll, box, gem pack, revive wisdom, gear property,
  costume, collection and area-dependent branches, then records all three
  truncate-toward-zero grant sites and the gated level-up side effects. One
  singleton Boolean and its branch meaning, static area IDs/tables and several
  presentation/mission/DSoul helpers remain unnamed. The reused literal at
  `0xD2AAB8` is now cross-verified as float32 `0.2`, but no full golden caller
  vector exists and live integration stays blocked; see
  `docs/migration/original-reward-progression-plus-exp-chain-v5.md`.
- Reward/progression pass 6 binds `Reward -> CalVillTax -> PlusGold`, the named
  building/fairy/ramble-pet/relic operands, tax whole/fraction carry, durable tax
  and money sinks, and the early-stage float32 `0.3` scaling branch. Disconnected
  Rust references cover the fully bound tax-candidate and `PlusGold` segments;
  two tax-rate operands, the cap and remaining event/static branches still block
  live integration. See
  `docs/migration/original-reward-progression-gold-tax-chain-v6.md`.
- Reward/progression pass 6 fixes the Gold mutation chain as `Reward ->
  CalVillTax -> PlusGold`, binds building, fairy, ramble-pet and relic-collection
  operands, and identifies the exact `UserData.tax`, `taxRemainder`, and
  `HunterData.money` sinks. The ARM64 package literal used by the early-stage
  `PlusGold` branch is exactly float32 `0.3`. Tax-rate identities, the tax cap,
  and remaining event/static tables are unresolved, so the full chain remains
  disconnected; see `docs/migration/original-reward-progression-gold-tax-chain-v6.md`.
- Reward pass 7 proves the remaining unique-gear blocker across the complete
  `RewardMetrial`, `LDHAEMDJCFF`, and `GHPHHEFFNKN` bodies: no typed
  `AdminEvilData.uniqueLevel` or `AdminDropUniqueGearData` object crosses the
  helper boundaries. Pool linkage, cut/percent evaluation and gear type/index
  RNG order remain unset; see
  `docs/migration/original-reward-progression-unique-gear-boundary-v7.md`.
- Gear pass 8 preserves the exact damage/armor/accessory formulas while proving
  their direct `GearData` reader boundary excludes plus/minus option arrays and
  runes. Generation roll order, enhancement writers, rune participation and the
  caller level adjustment remain unresolved and disconnected; see
  `docs/migration/original-gear-generation-boundary-v8.md`.
- Native gear recovery confirms `GetGearArmor` and `GetGearAcc` share
  `roundToEven(base * rating/100 * (1 + level/100) * qualityMultiplier)`, with
  quality multipliers `0.8/0.9/1.0/1.1/1.2`. A disconnected Rust reference also
  covers the exact Seal Attack ID selector. This pass-1 boundary is extended by
  the pass-3 Gear Damage recovery documented later in this section; see
  `docs/migration/original-native-gear-formula-recovery-v1.md`.
- Hunter damage-intake recovery now pins a two-stage armor block: decode
  `StatusData.CalcArmor`, truncate `CalcArmor * selectedArmorFactor`, then apply
  `(1 - HunterCtrl field 0x7A0)` and truncate again. Factor selection, later
  subtraction/clamps and dodge identity remain unresolved. The Evil damage
  prefix evidence was corrected: fields `476 + 480` are compared with float
  bits `0x00000001`, not numeric `1.0`; see
  `docs/migration/original-native-defense-damage-order-v1.md`.
- Gear formula pass 3 resolves `GameManager.GetFirstPercent` and the exact
  structural `GetGearDamage` expression over `AdminGearData.firstValue`,
  `ratingValue`, `firstPercent`, and `secondValue`. The disconnected Rust
  reference now covers the step schedule, rating clamp, quality multiplier and
  ties-to-even rounding. Caller level adjustment, options, enhancement, runes
  and trait aggregation remain unresolved; see
  `docs/migration/original-native-gear-formula-recovery-v2.md`.
- Hunter damage-tail recovery confirms the common order after unresolved
  modifiers: subtract the armor scratch value, forward `1` when the result is
  non-positive, otherwise truncate after the selected final factor, then apply
  `nowHp = max(nowHp - forwardedDamage, 0)` in the default pool branch. Four
  internal RNG branches are gear/set/effect procs rather than proven dodge or
  accuracy checks; see `docs/migration/original-native-hunter-damage-tail-v1.md`.
- Damage-tail pass 4 resolves the armor selector to five ordered
  `nowFeel/feel` bands with factors `1.2/1.1/1.0/0.9/0.8`, and resolves the
  auxiliary branch as first-entry `ShieldData.CurrentShield` absorption before
  HP spillover. Pass 5 corrects the prior ACTk direct-XOR error: the owner is
  `ConstantData`, `DEFALUT_DAMAGE_DECREASE_VALUE` is static init-only, ACTk
  `UnShuffle` decodes both captured sessions to exactly `0.75`, and
  `ConstantData..cctor` is its writer. The same pass pins all 32 optional
  pre-armor accumulator mutations and operation 33 armor subtraction. Several
  obfuscated gate meanings and multi-shield ordering still block live
  integration; see `docs/migration/original-native-hunter-damage-tail-v3.md`.
- Reward pass 2 fixes the exact top-level order `PlusExp -> CalVillTax ->
  PlusGold` and inventories every recovered RNG family/call count across
  `RewardMetrial`, `LDHAEMDJCFF`, and `GHPHHEFFNKN`. Unique-pool linkage,
  `dropCut`, `gearPercent` denominator and final type/index selection remain
  blocked; see `docs/migration/original-reward-progression-control-flow-v2.md`.
- Hunter attack-speed pass 4 resolves native target `0x33f79bc` as
  `HunterCtrl.InitHunterHpBar` (`0x06005C11`), not an attack-speed helper. The
  exact `HuntingAttackAction` body reads `DANCPPLMKIK` and decoded
  `BCEBGLKCDHN`, writes the confirmed `AttackAniTime` branch, and raw-copies the
  complete `StatusData.CalcAttackSpeed` `ObscuredFloat` into `mAttackDelay`.
  The two factor writers, `CalcAttackSpeed` producer and `mAttackDelay` FSM
  reader remain unresolved; see
  `docs/migration/original-native-hunter-attack-speed-chain-v1.md`.
- Hunter attack-speed pass 5 resolves the producer and timer chain: exact
  `StatusData.COJNMPDBOOO()` computes `AttackSpeed` and clamped
  `CalcAttackSpeed`; `FGCEFJCHNCK(float)` and `BuffSetting` type 0 feed
  `FuryValue`/`BCEBGLKCDHN`; `BuffEndSetting(0)` resets the factor to `1.0`;
  and `HunterCtrl.FixedUpdate()` is the direct `mAttackDelay` countdown reader
  subtracting `Time.deltaTime`. A class-wide scan of all 391 `HunterCtrl`
  methods found no direct managed writer for `DANCPPLMKIK`, so its serialized or
  engine-side source remains fail-closed. See
  `docs/migration/original-native-hunter-attack-speed-producer-chain-v2.md`.
- Outgoing-damage pass 6 proves `StatusData.LCENGICKKGP` returns
  `float32(CalcDamage) / CalcAttackSpeed` widened into the native double chain,
  confirms the critical threshold/optional bonus boundary, the `1.75` critical
  base and named critical-damage contributors, and final truncate-toward-zero.
  `RandDamage` is not called by `HunterCtrl.getDamage`; variance is applied
  downstream in `EvilCtrl.Damaged`. Full `CalcDamage`, target/tag, skill and
  monster-armor caller vectors remain blocked; see
  `docs/migration/original-native-hunter-outgoing-damage-chain-v1.md`.
- The exact `StatusData.EBNGMMPBEDA()` producer is now identified. Its two base
  scalar producers are mechanically resolved: `CalcLevel = float32(1 +
  level * 0.003f)`, while `CalcRevive` is constructor-defaulted to `1` and
  becomes `revive * 3` when revive is at least `1`. The deterministic analyzer,
  exact native bodies and static package-literal capture are documented in
  `docs/migration/original-native-status-data-calc-damage-producers-v1.md`.
  The formerly unnamed static operand is now `GuildManager.mRankBuffAttack`,
  producing the exact layer `1 + mTormentAttackUp + mRankBuffAttack`. The
  provisional PolyIndex labeling was also corrected: IDs
  `78/418/599/600/360/748/773` are `HunterData.fairyIndex` branches; the separate
  `StatusData.PolyIndex == 49` branch applies the `1.2999999523162842` package
  multiplier. The wider formula remains disconnected from live combat until
  the target armor/minimum-damage consumer and exact caller ordering are
  complete.
- Outgoing-damage passes 7 through 10 now preserve the complete final
  `HunterCtrl.getDamage` SSA, exact `getCriticalDamage`, named Slayer race
  selection, Rift-NPC scaling, all 49 direct caller vectors, and the producers
  for S12/S13/D14/D11/S15/stack/S8/final S9. The corrected Boolean contract is:
  argument 2 bypasses the critical roll, while argument 3 bypasses the
  target-specific critical multiplier and Slayer/Rift branches. Ordinary
  `HuntingAttackAction` calls `(false,false,false)`.
- Skill-coefficient passes 11 and 13 classify 15 of 49 callers. Proven
  families are Blizzard-modified, plain float percent, decoded
  `ObscuredFloat` percent, affine percent, and the internal `ObscuredInt`
  family that scales the percentage in float32 before multiplying base damage.
  Thirty-four callers and their public skill-row bindings remain unresolved.
- Pass 14 recovers the D8/D10 base tree: argument 1 selects decoded
  `CalcDamage` or float32 `CalcDamage / CalcAttackSpeed`; the gated JobTrait(5)
  branch evaluates its opaque operands in float32, widens, multiplies by the
  exact float64 `0.01` literal at `libil2cpp+0xD282B0`, and applies the percent
  without integer rounding. The later reduction, early-percent accumulator and
  optional job/sub-job multiplier order are preserved. Dynamic row meanings
  and the trait skill-dictionary key remain unresolved.
- Hit/miss pass 12 proves the only `getDamage` percentage RNG is critical
  selection, not accuracy. The direct Evil-to-Hunter normal chain has an exact
  attacker-owned effect-54 abort gate before `HunterCtrl.Damaged`, but does not
  read `StatusData.CalcDodge`; pass 18 subsequently recovers the separate
  Hunter dodge consumer and common producer, while indirect delivery paths
  remain unresolved.
- Accuracy is an accepted migration tech debt as of 2026-07-27. `GetGearAcc`,
  ignore-evasion properties, and accuracy-reduction skill text prove that the
  concept exists, but no generic attacker-accuracy versus target-evasion
  consumer or formula is proven. Live combat must not synthesize a hit-chance
  formula; keep accuracy disconnected until its callers and delivery gates are
  recovered.
- `apps/server/src/simulation/combat_core/` is the reusable, disconnected home
  for recovered combat arithmetic. It now contains checked numeric boundaries,
  CalcDamage, D8/D10, critical selection/damage, Slayer/Rift, the final outgoing
  SSA, skill coefficient families, effect-54 gating, Hunter incoming
  armor/shield/HP, and monster incoming armor/minimum-damage/HP. It remains
  deliberately disconnected from live combat until the unresolved hit/dodge,
  caller semantics and golden vectors are complete.
- ConstantData coefficient pass 16 additionally classifies six exact caller
  bodies for Poison Aura, Curse Aura, Frozen Heart Spin Splash/Shadow Strike,
  Frost Archer Sniping and Thunder Dragon Fury. The reusable core preserves
  their non-equivalent intermediate truncations and integer-scaling order, but
  public skill mappings, several gates and two native target identities remain
  unresolved; these six are therefore not all counted as fully closed callers.
- Combat presentation recovery binds `DamageCtrl` type `0` to incoming damage,
  type `1` to normal outgoing damage, type `2` to CRIT, type `3` to `Evade`, and
  type `16` to `Miss`, with exact localized format strings and colors. Pass 15
  also recovers the four-stage `DamageManager` rise/shrink coroutine and its
  ideal `1.1458333333s` envelope. Protocol v24 carries monotonic server-owned
  presentation events and the Pixi world renders the recovered `DefaultFont2`
  layout at the captured hit position without deriving outcomes from HP deltas.
  The presentation contract supports incoming, outgoing, critical, Evade and
  Miss outcomes without deriving them from HP deltas. See
  `docs/migration/original-combat-presentation-evidence-v1.md` and
  `docs/migration/original-native-combat-presentation-pass15.md`.
- Runtime integration v1 now connects the recovered neutral ordinary-attack
  spine: critical threshold and `1.75x` base, the persisted 30-entry RandDamage
  stream, mood/feel bands, monster/Hunter armor, minimum damage, default `0.75`
  Hunter intake factor, HP routing and authoritative critical presentation.
  Optional unresolved producers stay at native identity values. The Unity PRNG
  sequence, monster selected-runtime-factor writer and live effect-54 state
  remain explicit boundaries. Pass 18 subsequently connects total profile
  evasion to the recovered CalcDodge consumer. See
  `docs/migration/original-combat-runtime-integration-v1.md`.
- API35 dodge consumer pass 18 recovers the normal `CalcDodge` producer and
  consumer, direct early-exit callers, effect type 5's additive writer, and
  Raid/World Boss/Fallen/Guild/PvP variants. The exact normal resolver and
  golden vectors and dynamic named-source calculator live in
  `combat_core/hit_resolution.rs`. Total profile evasion now drives live Evade
  before armor/shield/HP; missing sources, effect type 5 and riding-pet dodge
  remain zero. Public names for several mode fields and Unity PRNG sequencing
  remain unresolved. See
  `docs/migration/original-native-dodge-consumer-pass18.md` and
  `reverse-engineering/evidence/original-native-dodge-consumer-pass18.json`.
- Live Hunter progression now preserves the recovered strict EXP carry rule:
  landing exactly on the current threshold does not level, while positive
  overflow carries. The threshold itself remains the explicit fixture sequence
  until authoritative per-Hunter `revive` is bound for `GetNeedExp`.

## 2026-07-28 Hunter interaction and movement handoff

- The seeded starting Hunter profiles in
  `apps/server/src/simulation/hunter_roster.rs` remain deterministic
  `web-rebuild-v1-fixture` data. Their level, combat stats, rarity and equipment
  are useful for migration testing but are not proven original-game starter
  rolls. Do not present them as recovered generation values until the original
  Hunter creation/RNG chain is closed.
- Hunter equipment slots with a projected catalog binding are now clickable in
  Hunter Info. The detail panel exposes only known catalog kind/index and
  evidence state; empty or unavailable slots remain disabled. Original
  per-item detail layout, rolled properties and interaction semantics remain
  unresolved and must not be synthesized.
- The Hunter roster Locate action now closes both roster and detail overlays,
  selects and focuses the matching world actor, and opens the ordinary world
  Hunter command bubble. It only sends a back-navigation command when the
  authoritative screen is actually `hunter_roster`, avoiding the prior
  `navigation_unavailable` error from the village screen.
- Unassigned Hunters now roam inside explicit town bounds using deterministic
  rebuild waypoints and pause cadence. These anchors and timings are temporary
  presentation tuning, not recovered original AI. Server obstacle avoidance
  and authoritative positions remain active.
- A Hunter assigned to a hunting region acquires the nearest live monster from
  the entire assigned region when no local target exists. It never searches a
  different region. Server tests lock both full-region acquisition and town
  roaming/pause bounds.
- Validation for this handoff: all 34 web test files / 137 tests pass, the web
  production build succeeds, the focused monster-world server suite passes,
  the full Rust suite and clippy passed during the implementation cycle, and
  `git diff --check` is clean. Browser smoke testing confirms equipment detail
  opening and Locate returning to the normal world command flow.
