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
- `infra/db/migrations/`: relational schema and seed migrations through `0026`.
- `infra/db/core_game/`: deterministic static core-game catalog SQL bundle and
  `psql` init entrypoint; it is separate from player-state migrations.
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

- The former monolithic `original_flow.rs` has been split behind an application
  facade into command dispatch, economy, Hunter actions, trade workflow,
  building/Hunter domain policies, world projection, and tests.
  `OriginalFlowSession` retains deterministic
  orchestration and durable-change detection; feature modules own their use
  cases. Hunter trade transitions are centralized in a typed workflow helper so
  command, reconnect normalization, settlement, and release cannot silently
  drift into different string-state rules.
- A repository-wide source-boundary audit now records the remaining modular
  monolith hotspots and their ordered decomposition in
  `docs/architecture/source-boundary-audit.md`. `pnpm architecture:validate`
  prevents domain-to-adapter and game-to-UI dependency inversions and ratchets
  current oversized files so new features cannot make them larger. Hunter actor
  and Spine presentation are now owned by the web game layer and reused by world,
  roster, and Hunter Info renderers instead of the world importing UI modules.
- The authoritative monster world is no longer a 3,653-line mixed module. Its
  public state/config facade is separated from fixed-tick runtime, control,
  skills, navigation, combat settlement, rewards/progression, spawn, presentation,
  and scenario tests. The deterministic tick order and public simulation API are
  unchanged, and the architecture ratchet now caps the facade at 550 lines.
- Player persistence is now a repository port facade rather than a mixed
  2,658-line adapter. In-memory and PostgreSQL implementations, aggregate codec,
  normalized building codec, full Hunter runtime codec, and persistence tests
  have separate modules. PostgreSQL transaction/idempotency behavior is
  unchanged; the facade ratchet is reduced to 150 lines.
- WebSocket command handling now creates structured `authoritative_command` and
  `authoritative_persist` tracing spans carrying session, player, correlation,
  revision, lease fence, and operation count. Hunter trade workflow transitions
  emit structured Hunter/command/building events. Architecture validation also
  ratchets wildcard parent imports toward explicit package APIs.

- ADR 0009 accepts tiered authority: ordinary movement/combat/common-farm
  reports may be client-predicted and asynchronously validated, while payment,
  premium currency, gacha, Hunter/protected-item ownership and player trade
  remain synchronous PostgreSQL transactions. The protocol/worker migration is
  incremental; existing server simulation remains authoritative until each
  low-value field is explicitly moved behind the farm-report validator.

- ADR 0010 replaces browser-local profiles with PostgreSQL-backed accounts.
  Registration/login issue fresh HttpOnly sessions bound to one durable player,
  so the same account can resume from another browser. Redis leases and
  PostgreSQL fencing still permit only one active simulation writer per
  account. Three development demo logins own independent player worlds and
  receive the full-stock seed on their first authoritative load.
- Protocol v32 raises the generated JSON message ceiling to 4 MiB and adds the
  authoritative `HunterInfoSnapshot.weapons` projection for individually owned
  rebuild weapon instances. The browser receives the immutable weapon id,
  English/Vietnamese names, legacy icon path, quality, rolled attack damage,
  base range, enhancement level, compatibility and ruleset; it never rolls or
  derives the outcome locally.
- Protocol v31 raised the generated JSON message ceiling to 4 MiB because the
  fully stocked demo welcome snapshot is about 1.84 MiB. Keeping the old 1 MiB
  ceiling caused the client to reject the welcome and reconnect forever at 92%.

- The WebSocket scheduler may run at a configured 1-60 Hz, but the gameplay
  domain advances through a deterministic 100 ms fixed-step accumulator. This
  keeps movement, combat, respawn, buffs and cooldowns invariant when transport
  cadence changes; scheduler elapsed time must be passed to
  `OriginalFlowSession::advance_simulation_step` rather than adding new
  hard-coded per-network-tick decrements.

- Town projection, camera/depth handling, building placement, normalized base
  building versus skin data, and visible-world packaging are implemented.
- Town building rows now use a `24 x 18` rebuild grid mirrored by client
  projection and authoritative server obstacles. Idle Hunter patrol is limited
  to a visually confirmed interior safe zone; newly added town Hunters enter
  through the recovered Bridge C tunnel route, and completed revival is placed
  beside the authoritative `build_2` Sanctuary footprint. Exact original
  navigation polygons, arrival FSM coordinates, and revival offset remain
  unresolved.
- Building registries, conditions, product stock, crafting/service routes,
  trading post, blacksmith/gear shop, potion route separation, and related DB
  migrations exist. UI fidelity is still an iterative migration, not proof that
  every building matches the original behavior.
- Material icon binding uses the complete exported `src_00000` through
  `src_00368` sprite sequence. The numeric `shop_product_*` namespace is cash
  shop content and must not be used as a material-ID mapping.
- Weapon core data v1 is now generated as a separate immutable rebuild release.
  It imports 8 difficulty bands, 4 rarity slot budgets, 40 English/Vietnamese
  weapon bases, all 125 decoded gear properties, 5 Virtue effects, and all 61
  collection-set rows into normalized `core_game` tables. The ordinary weapon
  pool now has 12 prefixes, 8 suffixes, and 160 difficulty-bound tier rows. A
  rebuild-designed flat-attack prefix scales from the accepted base-power
  curve; package-backed tiers stay inside their recovered ranges. Duplicate
  groups prevent incompatible families from stacking. Transformation
  acquisition and collection-set option semantics remain fail-closed. The
  Basic Auth admin exposes separate Weapon Bases, Modifiers, Modifier Tiers,
  Weapon Pools, Virtue Effects, and Collection Sets pages, plus a readable
  Weapon Wiki that groups every active modifier with weight and all eight level
  bands. See
  `docs/game-design/weapon-core-data-v1.md` and
  `docs/game-design/weapon-affix-pool-v1.md`.
- Weapon crafting is now connected for the 35 package recipe rows whose legacy
  icon bindings resolve to rebuild bases from level 0 through 600. The server
  rolls an inclusive flat Attack Damage value inside the selected base range,
  writes one durable gear instance per quantity, moves it through the Weapon
  Shop purchase path, and projects it into Hunter Inventory. The five level-700
  bases still have no distinct source recipe. Armor/accessory creation,
  prefix/suffix instance rolling remain explicit follow-up work rather than
  fabricated fallbacks. A Hunter may equip a compatible owned weapon through
  the authoritative `equip_hunter_weapon` command; the exact instance identity
  is persisted in the existing weapon-slot `catalog_kind` reference and its
  rolled Attack Damage contributes to projected ATK, normal attacks and skill
  fallback damage.
- Trading Post orders store the remaining requested quantity. Hunter auto-sale
  is capped by that remainder, decrements it atomically with the wallet/stock
  transfer, and leaves excess carried loot with the Hunter; zero restores the
  Request action.
- Hunter material sales now create a durable pending trade and clear any farm
  assignment so the authoritative world agent returns through the town corridor
  and walks to the constructed `build_3` Trading Post interaction point. Wallet,
  stock, request and settlement rows mutate only after arrival; the world
  projection then emits gold-received and material-sold presentation data. The
  exact original visit coordinate and cadence remain unresolved, so this uses
  the existing rebuild building placement and obstacle-aware pathing contract.
- Trading Post request and cancellation commands remain available while the
  unified world is focused on a hunting region (`field`) as well as town. The
  prior Village-only guard contradicted the single-instance world contract and
  caused valid order attempts to fail with `village_unavailable`.
- Building, crafting, shop, service and enhancement commands now share the same
  `Village | Field` world-availability guard, and service clocks continue while
  the camera is focused on a hunting region. Exact-town-only navigation intents
  such as entering the field remain separate. Authoritative command rejections
  are logged with player, session, correlation, intent and stable reason fields.
- Monster death projects gold as its own ground drop alongside independently
  rolled material drops. Gold and experience are awarded only when the owning
  Hunter collects the gold drop. Once a pickup starts, nearby aggro no longer
  resets it indefinitely; the presentation includes the collected quantity.
- Service crafting and Hunter-to-town service payment are connected for Inn,
  Infirmary, Restaurant and Tavern. An accepted rebuild rule now decays the
  three non-HP gauges on authoritative hunting ticks, preempts farming below
  10%, routes all four gauges to their matching service house, and keeps an
  out-of-stock Hunter waiting/complaining near that house. Exact original decay
  and choice formulas remain unresolved; see
  `docs/migration/hunter-autonomous-service-evidence-v1.md`.
- Alchemist (`build_14`) craft now routes finished consumables to Potion Shop
  (`build_11`) for display/purchase using the resolved recipe inputs/outputs
  and `hunterPaysTownGoldByTier` price rows. Craft timers, stack limits and
  native queue semantics remain unresolved; see
  `docs/game-design/alchemist-crafting-and-purchase.md`.
- Hunter vitals are four independent current/maximum pairs, not fixed percent
  bars. Packaged class rows prove HP base ranges of `5600..5800` or
  `6000..6200`; exact HP RNG and the original mood/satiety/stamina generation
  and decay formulas remain unresolved. See
  `docs/migration/hunter-vitals-mining-v1.md`.
- The disposable rebuild roster no longer seeds all four gauges as `100`:
  migration `0027_non_percent_hunter_vitals.sql` uses deterministic fixture
  current/max values, with HP kept inside the recovered class bounds. These
  values are presentation/test fixtures, not recovered constructor RNG; live
  captured values still supersede them when available.
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
  I/II/III; monster actors and ground drops remain server-authoritative and
  ephemeral, while active Hunter position/FSM runtime is checkpointed for
  reconnect continuity and world difficulty stays global.
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
- Protocol v23 introduced `use_hunter_skill`; later runtime work connects all ten packaged
  basic skills with exact base-job ownership and catalog cooldowns after
  server-side learned-state, target-range, and readiness validation. Recovered
  level-1 effect values are connected as described in the current handoff;
  skill-specific animation bindings and exact native priority remain unresolved.
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
- Monster snapshots now expose the catalog-backed source index, HP, damage,
  armor, base EXP and gold reward through protocol v26. Selecting a live
  monster shows those authoritative catalog values in the web client. The
  displayed damage is not yet the exact original damage dealt to a Hunter:
  live combat still applies the explicit rebuild `250` compatibility divisor
  because the native selected runtime-factor writer remains unresolved. With
  current demo Hunter armor, many difficulty-zero attacks consequently reach
  the recovered minimum-one branch. Collecting its
  gold drop credits the catalog base EXP and emits a server-authored `+n EXP`
  presentation over the collecting Hunter. Native `PlusExp` still caps stored
  level at 99 (display level 100) and discards further EXP. EXP modifiers,
  exact class/revive threshold binding and reincarnation/rank-up remain
  explicitly unresolved and are not synthesized or activated.
- Live ordinary Hunter attacks now apply the recovered native stored-level
  factor `float32(1 + level * 0.003)` to the fixture base attack before the
  connected outgoing-damage resolver. This increases the base damage component
  by 0.3% per stored level and reaches the nominal float32 factor 1.297x at
  displayed level 100; the downstream integer conversion truncates exactly as
  native arithmetic does. No HP or defense level scaling is claimed;
  reincarnation remains disconnected.
- Field projection now renders the real authoritative `village-hunter-{id}`
  agents instead of the old independent `field-hunter-01` roaming fixture.
  This makes post-kill target acquisition/movement observable and preserves
  Hunter-targeted EXP presentations through the client event pipeline. Gold
  uses the confirmed gold icon layered as an explicit rebuild ground-pile
  presentation because no original ground-drop gold sprite is yet bound.
- Operational Hunter fixtures now start with exactly the two packaged basic
  skills for their H1-H5 job. Existing fixture rosters with an empty skill list
  are backfilled once without overwriting learned state or cooldowns.
- Basic skills now auto-cast server-side when a Hunter has a live in-range
  target and a ready skill. Cooldowns and level-1 catalog values come from the
  recovered 1.411 rows. Connected effects cover Fury attack-speed/basic-damage,
  War Cry stun proc, Holy Light/Thunderbolt/Round Slash AoE, Barrier defense,
  Multishot four-hit damage, Dodge evasion, Ice Armor retaliation slow, and
  Concentrate critical chance. Exact native skill priority/proc ordering is not
  recovered, so choosing the first ready learned skill is explicitly the
  `web-rebuild-v1-auto-skill` policy rather than an original-game claim.
- Active world frames now declare `server_authoritative_simulation`. The FE may
  predict movement and presentation between 10 Hz confirmations, but position,
  targets, hits, damage, loot, economy and rollback authority remain server-side.
- Migration `0026_demo_basic_skill_aliases` explicitly maps the rebuild's
  class-scoped `skill_h*_0*` IDs to the recovered `basic:0..9` definition rows;
  without these FK rows, backfilled learned skills cannot persist and the WS
  session fails closed while loading the player.
- Static catalogs that were previously JSON-only are now reproducibly packaged
  under the `core_game` PostgreSQL schema by `tools/generate-core-game-sql.py`:
  monster stats/drop slots, unique-gear pools, material-market cross-links,
  recipe/building material links (including unresolved conditions), EXP rows,
  and the complete packaged gear rows. The bundle records source SHA-256 values
  and has count guards; it does not replace the existing `0010` material/economy
  seed or any player ownership/ledger tables. Run `psql "$DATABASE_URL" -f
  infra/db/core_game/init.sql` from the bundle directory.
- Durable schema v15 checkpoints active Hunter world runtime and Hunter potion
  cooldown state so reconnects
  restore position, facing, action/animation, a still-valid monster target,
  timers and temporary skill presentation state before the welcome snapshot.
  Monster actors and drops remain ephemeral; invalid targets are cleared and
  interrupted loot collection resumes target acquisition without moving the
  Hunter. Abrupt process loss remains bounded by the last completed checkpoint.

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
  Hunter command bubble. The web roster is now entirely client-presentational;
  it never enters or navigates back from the removed legacy `hunter_roster`
  screen.
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

## 2026-07-28 external Hunter auto-trade mining

- A rooted API35 ARM64 session was used without an in-process long-lived hook:
  external `/proc/PID/mem` reads remain stable and recover the resident
  30-entry `RandDamage` stream. Unexecuted protected method pages are not
  treated as decrypted code.
- One-shot native capture recovered exact ARM64 bodies for Hunter
  `ItemPotionBuy`, `SpeakItemSell`, `SpeakWeaponBuy`, `SpeakArmorBuy`,
  `ItemArmorBuy`, `ItemWeaponBuy`, and `ItemAccessoryBuy`. The three gear-buy
  bodies share a `0x224`-byte structural flow with pending-object cleanup and
  repeated mutation-helper calls, but buyer, wallet, stock, ownership, and
  equip semantics remain unresolved. See
  `docs/migration/original-native-hunter-auto-trade-evidence-v1.md` and
  `reverse-engineering/evidence/original-native-hunter-auto-trade-decrypted-api35-v1.json`.
- `tools/runtime/capture-android-process-methods.py` captures stable external
  method ranges by token/module offset. It is evidence-only and does not read
  managed values or connect production services.
- `tools/tests/test_original_hunter_auto_trade_evidence.py` cross-validates the
  external method artifact against the one-shot decrypted capture (all seven
  method identities, offsets, and native sizes) and verifies body hashes.
- The rebuild now implements the user-confirmed player-guided economy flow as
  an explicit web-rebuild contract: idle Hunters and Hunters farming an ordinary
  region auto-settle only requested materials, while the Hunter tooltip can
  invoke the same seller-attributed transaction. Gold drops are credited only
  to the Hunter wallet and are never copied into sellable material inventory;
  an underfunded town buys only the quantity its wallet can afford.
  `purchase_shop_item` now carries `hunter_id` and atomically
  debits Hunter gold, credits town/Lord gold, decrements shop stock, and adds a
  durable owned product. Migration `0024_hunter_owned_items.sql` persists that
  ownership. These transaction semantics are rebuild product behavior; they do
  not claim the still-unresolved original helper/offset identities.
- Gear shop rows have durable individual-stock persistence and purchase transfer
  wiring, but creation is currently fail-closed. The original ARM64 capture
  proves `GearData` has plus/minus and additional option arrays plus `buyGold`,
  while quality/option pools, roll order, default-mod branches, and
  mod-dependent pricing remain unresolved. The former synthetic
  `web-rebuild-v1-gear-roll` generator was removed; craft rejects with
  `gear_creation_evidence_unresolved` before debiting materials. See
  `docs/migration/original-gear-creation-writer-boundary-v1.md` and
  `reverse-engineering/evidence/original-gear-creation-writer-boundary-v1.json`.
- Gear enhancement now has a versioned intent and ownership/result projection
  for the product-required `+20` cap and four UI modes. It remains fail-closed:
  original enhancement cost, material and probability bindings have not been
  captured, so the Blacksmith preview lists those blockers and the server never
  consumes resources or advances a level. See
  `docs/game-design/gear-enhancement-flow.md`. The UI route is the confirmed
  Enhancement Forge `build_15`; its popup-template binding remains unresolved.

## 2026-07-29 UI fallback removal handoff

- The web client no longer derives Hunter roster cards from old world entities,
  infirmary rows, service rows, `waiting_queue`, nested profile aliases, or the
  protocol-v14 adapter. Only the current authoritative `active_hunters` and
  `waiting_hunters` projection is rendered. A legacy `hunter_roster` screen is
  treated as a protocol fault instead of being normalized into the current UI.
- Durable schema v16 migrates persisted `hunter_roster` navigation to `village`
  during server restore. This prevents old browser sessions from reconnecting
  forever at 92% without reintroducing the removed roster screen.
- Generic service/production popup branches have been removed. A building must
  match a current explicit route contract; otherwise its complete product frame
  shows a compact fail-closed error and no old tabs or substitute product UI.
- Trading Post purchase requests remain open and disabled while the
  authoritative `set_material_request` command is pending. The popup closes
  only after an accepted response; transport and server rejections remain
  visible so the player can retry without losing the requested quantity.
- The unused pre-authoritative Pixi world renderer, legacy animation mapper and
  legacy snapshot interpolator were deleted. Required town-building asset load
  failures now fail the game loading flow rather than silently omitting content.
- The loading screen has a 30-second watchdog and explicit reload action. A
  missing current snapshot or asset reports `ERROR`; it cannot remain silently
  parked at the post-map 92% stage.

## 2026-07-29 Vietnamese localization handoff

- The web client now uses a typed, Vietnamese-first localization runtime under
  `apps/web/src/i18n/`. Player-facing shell, login/loading, Hunter, combat,
  building, shop, crafting, Trading Post, tooltip and error wording is resolved
  through semantic message keys instead of inline literals.
- Vietnamese (`vi`) is the only currently supported locale and is the explicit
  default/fallback. The catalog is flat and compile-time checked through
  `MessageKey`; adding another language requires a complete catalog matching the
  same key set rather than DOM post-processing or an English UI fallback.
- Runtime names and descriptions supplied by authoritative snapshots remain
  data, not client translations. Original multi-language labels retained in
  `content/original-ui-labels.ts` remain migrated source evidence; current UI
  calls explicitly request Vietnamese.
- `i18n/ui-wording.test.ts` guards the primary player-facing DOM sinks against
  newly introduced hard-coded wording. Localization runtime tests cover locale
  fallback, named parameters, unresolved parameter visibility and locale-aware
  number formatting.
- Validation: TypeScript `--noEmit` passes, all 42 web test files / 199 tests
  pass, the production web build succeeds, and `git diff --check` is clean.

## 2026-07-30 Hunter presentation preload and roster interaction

- The roster and both Hunter Info overlays now share one memoized Hunter Spine
  asset bundle and initialize during the post-login game loading phase. Opening
  Hunter Info no longer starts a second alias-specific skeleton/atlas load.
- Hunter roster avatars are real accessible buttons and use the same Hunter Info
  action as the explicit Info control.
- The source difficulty sprite contains an embedded Korean label. Runtime UI
  masks only that label area with the localized world-mode text; the recovered
  skull and badge artwork remain unchanged.

## 2026-07-30 Quality baseline

- `.github/workflows/quality.yml` now enforces independent web, Rust, migration,
  and browser-smoke jobs on pull requests and pushes to `main`.
- Playwright covers the sign-in/loading/town/Hunter-roster journey at desktop
  1366x768 and mobile 393x852. The Docker smoke job also runs the authoritative
  protocol reconnect journey.
- `tools/verify-latest-migration.sh` applies all migrations, rolls back the
  newest migration, and reapplies it. It refuses to run unless the caller marks
  the database as disposable.
- Browser failures and rejected intents emit structured `evil:telemetry`
  events; server intent, lease, persistence, and checkpoint events remain in
  structured `tracing` logs.
- Review ownership and the executable regression checklist live in
  `.github/CODEOWNERS`, `.github/pull_request_template.md`, and
  `docs/engineering/regression-checklist.md`.

## 2026-07-30 relational gameplay content authority

- ADR 0010 makes PostgreSQL the production source of truth for stable gameplay
  content while formulas, RNG and state machines remain in Rust.
- Migrations `0031` through `0033` normalize world maps, density/waypoints,
  Hunter EXP progression, monster definitions/pools/drops, all 369 material
  difficulty rows, 671 gear definitions with 3,355 ratings and recipe bindings,
  9,715 gear material requirements, and 40 consumable level bindings.
- Production startup installs map, monster and progression catalogs from the
  active release and fails closed if they are missing or inconsistent. Gear,
  material and consumable runtime decisions are loaded through the normalized
  building gameplay catalog.
- Starter Hunter generation and basic-skill lookup now consume the normalized
  Hunter class, rarity, characteristic and skill tables; their former production
  match tables remain only as exact `cfg(test)` fixtures.
- `tools/generate-progression-content-migration.mjs` and
  `tools/generate-runtime-content-migration.mjs` deterministically reproduce the
  generated SQL from immutable package/evidence inputs. Source hashes are stored
  in `content_source_manifest`; the one known monster drop-array mismatch is
  preserved as unresolved JSONB evidence rather than guessed.
- Future admin tooling must clone/edit a draft content release, validate all
  references/counts, record an audit event and promote atomically. Active release
  rows are immutable and must not be edited in place.

## 2026-07-30 player inventory authority

- Migration `0034_player_inventory_authority.sql` separates Hunter-owned stacks
  from individually rolled gear instances. `player_hunter_item_stack` stores
  stackable product quantities; `player_hunter_gear_instance` stores identity,
  recipe/catalog binding, rating and enhancement level for each gear item.
- `player_hunter.owned_items` remains a compatibility read path only. An
  authoritative save clears that legacy JSONB field and writes normalized rows
  in the same transaction as the Hunter roster.
- Gear purchases are never merged by product ID; each purchase receives a
  distinct gear instance ID. Consumables and other stackable products continue
  to aggregate by product.
- Skill cooldown persistence now stores the exact remaining duration through
  `cooldown_ready_at` instead of replacing every non-ready skill with a fixed
  one-second delay on reconnect.
- Runtime-capture tables remain evidence snapshots and are not interchangeable
  with live learned skills, owned inventory, growth state or pet ownership.
  Growth/material/pet normalized tables still require gameplay wiring before
  those features can claim complete player-state authority.

## 2026-07-30 admin console scaffold

- `apps/admin/` is a separate Vite/Tailwind operations-console application with
  responsive navigation, an item table, search, status/category controls and
  create/edit/delete interaction scaffolding.
- The current item rows and CRUD mutations are client-local demonstration data.
  They are not connected to PostgreSQL and must not be presented as a working
  content editor. The next admin slice must implement draft-release APIs,
  validation, audit records and atomic promotion before enabling durable CRUD.
- The Rust server currently exposes only authenticated `GET /admin/overview`.
  It reports service/protocol/schema status and does not mutate player or
  content data.
- Admin Basic Auth credentials are required server configuration. Docker Compose
  refuses to start the server without `ADMIN_BASIC_AUTH_PASSWORD`; the browser
  retains the username only and keeps the password in memory for the current
  page session rather than local storage.
- The intended deployment is same-origin or a trusted reverse proxy. Do not
  expose the temporary Basic Auth endpoint directly to the public internet;
  production administration still requires TLS, stronger identity/RBAC, CSRF
  protection, audit logging and rate limiting.

## 2026-07-31 demo crafting handoff

- The gear crafting popup no longer exposes destination-stock capacity or
  disables Produce from the client-side capacity projection. Material
  sufficiency remains visible and authoritative server validation still owns
  command acceptance.
- Migration `0041_demo_hunter_gold` raises existing demo Hunter wallets to
  `1,000,000,000` gold and wraps lazy demo seeding so future demo worlds receive
  the same amount. Real player accounts are not modified.
- PostgreSQL reloads the seeded demo roster before returning the first world
  snapshot, preventing the initial in-memory roster from overwriting seeded
  wallet values at the first checkpoint.
- Migration `0042_reseed_demo_town_stock` restores full demo materials, product
  stock and crafted display stock for existing demo towns without touching real
  player accounts; demo accounts without a town remain lazy-seeded on first
  world load.

## 2026-08-02 weapon authority and craft lifecycle handoff

- Rebuild weapon rolls no longer derive entropy or instance identity from the
  browser correlation ID. The server creates a UUID for each new roll; replayed
  idempotent commands return before allocating another roll. A fixed server roll
  UUID still produces deterministic values for tests.
- Skill damage now always includes the equipped rebuild weapon contribution,
  including seeded Hunters whose authoritative profile already contains
  `dps_milli`.
- `infra/db/run-migrations.sh` installs the versioned `core_game` bundle after
  numbered player-state migrations and records a combined checksum. Validation
  on a disposable PostgreSQL 17 database applied the runner twice and verified
  counts `40/126/160/20/5/61` for weapon bases, affixes, tiers, pools, virtues,
  and collection sets.
- The client applies an authoritative intent-result snapshot before transient
  feedback. Craft submission is disabled immediately while one request is
  pending, remains disabled across snapshot renders, clears on rejection or
  connection loss, and animates only the popup and recipe that originated the
  accepted request.
