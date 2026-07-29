# Hunter Gear Enhancement Flow

## Purpose and evidence boundary

This document defines the complete Hunter-first gear-enhancement vertical slice.
The flow does **not** begin by clicking the Enhancement Forge. It begins when the
player selects a Hunter in the world and orders that Hunter to enhance gear.

The interaction sequence, four modes and `+20` cap are product requirements
supplied by the user together with six screenshots from the original game. The
screenshots are valid evidence for visible UI and the observed example, but they
do not prove the complete original cost table, material-selection rules,
probability table, RNG algorithm or failure mutation rules. Those mechanics
remain fail-closed until package or authorized runtime evidence resolves them.

## Complete six-step player flow

### 1. Select a Hunter and issue the enhancement order

1. The player selects a world Hunter.
2. The Hunter command menu exposes the gear-enhancement action.
3. Selecting the action sends only the intent to begin an enhancement visit for
   that Hunter.
4. The server validates that the Hunter exists, is available for the command and
   can reach a valid Enhancement Forge (`build_15`).
5. The Hunter travels to the forge using the normal world movement/action flow.
   The command menu closes immediately without a confirmation popup. While the
   authoritative task is traveling, a small non-interactive enhancement marker
   follows the Hunter near their feet to communicate the current destination.
6. On arrival, the Hunter stops in a server-owned waiting state. The travel
   marker is replaced by the enhancement interaction icon above their head.
7. The player clicks that icon to open the enhancement screen for that specific
   Hunter.

Opening the building directly must not silently select an arbitrary Hunter. The
screen is bound to the Hunter who received the order and reached the interaction
point. Reconnect restores the travel or waiting state from durable Hunter action
state instead of restarting the flow at the popup.

Durable enhancement tasks carry a server workflow version. A reconnect may
resume only a task written by the current compatible workflow. Rows written by
an older build without that marker are released to `idle` during restore; the
server must not strand a Hunter by interpreting an obsolete task shape as a
current waiting/configuration state. An enhancement action label without a
corresponding durable task is normalized the same way. This compatibility
repair does not mutate gear, materials, gold, or enhancement results.

### 2. Select owned gear

The first popup state shows the selected Hunter's eligible owned/equipped gear.
The player chooses one gear instance, not an aggregate shop product or catalog
definition. Each row uses the authoritative instance ID, icon, name, quality and
current enhancement level.

- The maximum enhancement level is `+20`.
- Gear already at `+20` remains inspectable but cannot start another attempt.
- A `+N` badge is rendered only from a persisted, non-null server value.
- Empty, unowned, ineligible or unbound gear cannot be submitted.

After selection, the chosen item moves into the central gear slot and the screen
advances to configuration.

### 3. Select materials and enhancement mode

The configuration state contains:

- the selected gear instance and its current `+N` level;
- the required enhancement-material slot;
- optional-material controls shown by the original UI, including the visible
  Light Stone and Enhancement Ore choices when their rules are resolved;
- the success-rate presentation for the next attempt;
- the gold cost for **the next attempt only**;
- the Hunter's current gold and relevant carried-material quantities;
- four mutually exclusive enhancement modes.

| Wire mode | Visible label | Stop target |
|---|---|---:|
| `single` | Chỉ 1 lần | Run at most one attempt |
| `to_10` | Cho đến 10 | Stop when the item reaches `+10` |
| `to_15` | Cho đến 15 | Stop when the item reaches `+15` |
| `to_20` | Cho đến 20 | Stop when the item reaches `+20` |

The cost row is not an up-front quote for the entire multi-attempt run. It shows
only the cost of attempting the current level once. For a target mode, the final
gold and material consumption cannot be known before RNG runs. The server
therefore executes the attempts sequentially, accumulates actual consumption and
returns the totals in the result.

Before each attempt, the server recalculates the cost, required materials and
probability from authoritative gear state. If the Hunter lacks the gold or any
required material for that next attempt, the loop stops immediately. It does not
partially charge an attempt that cannot start. All completed attempts remain
committed, their resources remain deducted, and the result reports the level and
totals reached before the shortage.

### 4. Process the authoritative attempt loop

After confirmation, the client sends only:

- command/correlation ID;
- Hunter ID;
- owned gear instance ID;
- selected optional-material choices by resolved material ID;
- one of the four modes.

The client never sends a claimed level, cost, probability, random roll,
consumption total or result. The server validates and accepts the command before
the processing presentation begins.

For every permitted attempt, the server performs this ordered transaction work:

1. lock the Hunter wallet, relevant inventory rows and gear instance;
2. re-read the current gear level and derive the next-attempt rule;
3. stop if the cap/selected target has already been reached;
4. stop if the next attempt cannot be fully funded;
5. deduct exactly that attempt's gold and materials;
6. draw the authoritative RNG result;
7. apply the recovered success or failure mutation;
8. persist the new gear level and append an attempt-ledger entry;
9. continue only while the selected target has not been reached and another
   attempt can be funded.

The original screenshot warns that continuous enhancement can perform many
attempts and that failures from enhancement level 10 onward can reduce the
level. This is observable UI evidence, not yet a recovered executable rule: the
exact downgrade amount, floor, trigger probability and maximum-attempt semantics
must be mined before connecting the loop.

While the accepted command runs, the client displays the dedicated
"Đang cường hóa..." processing state. Closing, refreshing or reconnecting must
not duplicate or cancel already committed attempts. Reusing the same correlation
ID returns the original transaction result.

### 5. Show the result

The result view is rendered only from the authoritative response. It includes:

- final enhancement level;
- target reached or exact stop reason;
- number of attempts, successes and failures;
- total gold consumed;
- total quantity consumed for each material;
- remaining Hunter gold and material quantities;
- the final gear instance snapshot.

Valid stop reasons include target reached, `+20` cap reached, insufficient gold,
insufficient material, ineligible/changed ownership, and an evidence-disabled
rule. A resource shortage after completed attempts is a successful partial run,
not a rollback of those attempts.

The supplied result mock shows a gear instance at `+20`, a lower Hunter wallet
and a lower Ultimate Enhancement Stone quantity. This confirms the observed
presentation and consumption in that example only. The implementation must not
assume that Ultimate Enhancement Stone is universally required for every gear,
rating or enhancement level until the material binding is recovered.

### 6. Persist and display the enhanced gear

The final `enhancement_level` belongs to the owned gear instance. It must be
persisted in the same database transaction as the attempt ledger and resource
mutation, then projected consistently in:

- the Enhancement Forge selection/result screen;
- the Hunter equipment screen;
- gear details and inventory rows;
- any server-side stat calculation that is explicitly proven to consume the
  enhancement level.

The Hunter equipment screen displays `+20` on a capped item, as shown in the
supplied mock. Presentation may display the level immediately from the accepted
server result, but no stat increase may be guessed while the enhancement-to-stat
formula remains unresolved.

## State machine

| State | Owner | Entry | Valid exit |
|---|---|---|---|
| `hunter_selected` | Client presentation | Player selects a world Hunter | Open command menu or deselect |
| `enhancement_order_requested` | Server | Player sends Hunter enhancement intent | Reject, or assign forge travel |
| `traveling_to_enhancement_forge` | Server simulation | Command accepted and route available | Arrive, cancel through a supported command, or fail route |
| `waiting_for_enhancement_interaction` | Server simulation | Hunter reaches `build_15` interaction point | Player opens UI, or a supported cancellation/reassignment occurs |
| `selecting_gear` | Client UI over server snapshot | Player clicks the Hunter's overhead icon | Select eligible instance or close |
| `configuring_enhancement` | Client UI over server preview | Player selects gear | Change gear/options/mode, confirm, back, or close |
| `enhancement_submitted` | Server | Confirm sends one idempotent intent | Reject without mutation, or accept transaction |
| `enhancement_processing` | Server transaction + client presentation | Server accepts command | Complete/partial result or transactional failure |
| `enhancement_result` | Client UI from server result | Server returns final snapshot and ledger summary | Enhance again if eligible, choose other gear, or close |

The exact original behavior after closing the UI, cancelling Hunter travel, or
issuing a competing Hunter command has not been recovered. Until it is, the
rebuild must use one explicit documented product policy rather than inferring
native behavior, and must not move economy/RNG authority to the browser.

## UI contract

All states follow
`docs/engineering/project-rules.md#source-style-ui-consistency`.

- The world command begins from the selected Hunter; the forge popup is not the
  entry point for choosing a Hunter.
- Arrival presents the enhancement icon above that Hunter and the popup opens
  only from that interaction.
- The popup uses the established source-style frame, title treatment, spacing,
  green primary action, red Close action, disabled/focus states and touch/
  keyboard behavior. Its dimensions may differ to fit the workspace.
- Gear selection and configuration are distinct states matching the supplied
  mock sequence. Do not show every slot/mode before the player selects gear if
  that collapses the two steps into one unrelated dashboard.
- The central composition retains the optional-material slot, selected gear,
  required-material slot, Hunter, anvil/forge and smith presentation.
- The next-attempt gold cost is labelled unambiguously; a target mode does not
  replace it with a guessed total.
- Processing is a dedicated blocking presentation after server acceptance.
- Result and persisted `+N` badges are sourced from server-owned state only.
- An unresolved evidence boundary appears as a compact player-readable disabled
  state inside the complete screen. Raw blocker keys remain debug-only.

## Server authority, transaction and concurrency contract

Enhancement changes protected ownership, inventory, currency, RNG and gear
progression. It is therefore always a synchronous authoritative server command;
it must not enter the ordinary farm-report queue.

- PostgreSQL is the durable source of truth for owned gear, Hunter wallet,
  material inventory, command idempotency and the attempt/result ledger.
- One transaction locks all mutable rows in a stable order.
- A gear instance cannot be sold, equipped by another Hunter or enhanced by a
  second command while the transaction owns its lock.
- The server derives every attempt rule from its own versioned content/evidence
  tables and current persisted state.
- A duplicate correlation ID cannot consume resources twice.
- A conflicting reuse of a correlation ID is rejected.
- On an unexpected transaction error, the current uncommitted attempt rolls
  back atomically. Previously committed behavior depends on whether the loop is
  stored as one transaction or a recoverable transaction batch; that choice
  must preserve a single idempotent user result and be documented before live
  activation.
- Telemetry records request mode, start/final level, attempt count, stop reason,
  consumed resources and evidence ruleset version without exposing RNG seeds to
  the client.

## Evidence audit

### Resolved enough to model or display

| Evidence | Status | Safe use |
|---|---|---|
| `build_15` serialized catalog entry is the Enhancement Forge | Confirmed package evidence | Route the Hunter to the correct building type |
| Per-Hunter `GearData`/owned gear is an instance-shaped domain boundary | Confirmed reflection/rebuild ownership boundary | Address one owned gear instance, not a catalog stack |
| Material definitions include Light Stone (`material:137`), Enhancement Ore (`material:154`), Enhancement Stones (`material:156..160`) and Ultimate Enhancement Stone (`material:160`) | Confirmed package catalog names/IDs | Display a material only after a separate enhancement-use binding selects it |
| Four modes: once, to 10, to 15 and to 20 | User-supplied product requirement and screenshot evidence | Versioned command enum and UI choices |
| Maximum displayed enhancement level `+20` | User-supplied product requirement and screenshot evidence | Cap/projection contract |
| UI shows next-attempt gold cost, Hunter wallet, optional materials, required material, processing and final result | User-supplied screenshot evidence | Reconstruct the visible workflow |
| Final enhancement level is shown on Hunter equipment | User-supplied screenshot evidence | Project persisted `+N` on owned/equipped gear |

### Still unresolved and blocked

| Blocker | Missing proof | Required evidence |
|---|---|---|
| `enhancement_cost_binding` | Gold cost by gear kind/rating/current level and any modifiers | Authorized before/after captures plus exact native/table consumer |
| `enhancement_probability_binding` | Success chance by current level, gear and optional materials | Exact probability producer, RNG call/order and controlled vectors |
| `enhancement_material_binding` | Required material ID and quantity by gear/current level; when Ultimate Enhancement Stone applies | Exact selection/quantity consumer plus inventory diffs across representative attempts |
| Optional Light Stone effect | Whether it protects level, changes rate, or has another effect and when it is consumed | Toggle-on/off captures and native branch recovery |
| Optional Enhancement Ore effect | Exact effect, quantity and consumption condition | Toggle-on/off captures and native branch recovery |
| Failure mutation | Downgrade amount/floor, whether material changes it, and the precise `+10` boundary | Failed-attempt state diffs and writer body |
| Enhancement stat contribution | How `enhancement_level` changes damage/armor/accuracy/options | Writer/reader chain and golden vectors |
| Original travel/cancel/dialogue details | Exact Hunter AI method chain, destination anchor and close/cancel behavior | Controlled Hunter command/arrival captures |
| Original popup/controller binding | Exact serialized popup prefab/controller and animation bindings | Scene/prefab binding evidence |

Until the first three blocker rows are resolved, the live server must reject the
enhancement execution before consuming gold/materials or changing the gear
level. The complete Hunter travel, selection and configuration UI may be
implemented, but its confirmation action remains fail-closed with a concise
player-facing status.

## Required runtime capture

For each controlled capture, record package version, package ID, device ABI,
Frida client/server versions where applicable, UTC timestamp, exact player
actions and before/after state. Minimum useful scenarios are:

1. one successful single attempt with no optional material;
2. one failed attempt below `+10` and one failed attempt at/above `+10`;
3. one attempt with Light Stone and one with Enhancement Ore;
4. the same current level across different gear ratings/types;
5. one target-mode run stopped by insufficient gold;
6. one target-mode run stopped by insufficient material;
7. one run that reaches `+20` and the subsequent capped interaction;
8. Hunter command, travel, arrival icon, popup open, close and reconnect states.

Each scenario must capture Hunter gold, all relevant material quantities, gear
instance fields, selected options, displayed cost/rate and the authoritative
result. A matching material name or screenshot position alone is not a formula
or consumption binding.
