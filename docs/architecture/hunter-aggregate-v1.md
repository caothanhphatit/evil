# Hunter Aggregate And Evidence Projection

## Purpose

This design defines the first evidence-safe Hunter aggregate for the web
rebuild. It incorporates the Android `1.411` runtime schema capture without
claiming values, rates, dictionary-key meanings, or mechanics that were not
captured.

## Authority And Compatibility

- PostgreSQL remains authoritative for durable Hunter state and the Rust
  server remains authoritative for every mutation.
- The existing operational demo roster remains a migration fixture. Its
  generated values and convenience fields are not original-game evidence.
- Existing roster and Hunter Info fields remain wire-compatible while the new
  `runtime_evidence` projection is introduced additively.
- An evidence section contains an `evidence_state` and a nullable `value`.
  `schema_confirmed` means the runtime type is known; it does not mean a player
  value was captured. Missing values are serialized as `null`.
- Source dictionary keys are opaque strings. The server must not infer their
  format or replace them with the rebuild's numeric Hunter ID.

## Aggregate Boundary

`player_hunter` is the aggregate root. Active and waiting membership remains a
relational discriminator for efficient server queries, while
`source_dictionary_key` preserves the captured `HunterDataDic` key separately.
The current capacity, ordering, and promotion behavior are rebuild policies and
must not be labelled as recovered original rules.

Stable owned state is normalized by responsibility:

- core identity, job chain, raw vitals, and raw combat values on
  `player_hunter`;
- learned runtime skill state in `player_hunter_runtime_skill`;
- item, gear, and consumable inventory in separate child tables;
- the `GUP_Property_LV` array as ordered growth rows;
- riding-pet linkage and exact captured scalar fields in a dedicated row.

Captured arrays use PostgreSQL arrays. JSONB is not used to avoid modelling
known stable fields. Nested `ConsumData` and riding-pet gear values remain
unrepresented because their live value schemas have not yet been captured.
Runtime child rows represent complete reflected objects, so their captured
fields are non-null. Partial captures must not insert a row; they leave the
corresponding section value null.

Raw appearance indices and hide flags are stored separately from
`player_hunter_visual_component`. The former preserves source evidence; the
latter remains the rebuild's resolved rendering composition. A future content
resolver may link them only when the mapping is evidenced.

## Read Projection

The server publishes seven stable sections under `runtime_evidence`:

1. `job`
2. `status`
3. `skills`
4. `appearance`
5. `inventory`
6. `growth`
7. `riding_pet`

Each section is independently nullable. Rows loaded from the normalized runtime
tables become `value_captured`; an absent row remains `schema_confirmed` with a
null value. The compatibility `hunter_info` projection continues serving the
demo UI until the client migrates to these sections.

Protocol v16 adds this projection additively. Its member field, section names,
evidence-state vocabulary, and compatibility mode are declared in
`packages/protocol/world-v1.schema.json`; generated TypeScript consumes that
contract. Rust projection structs remain server-owned DTOs and do not expose
database row types or IL2CPP backing-field names.

## Write Boundary

This slice adds transactional persistence round-tripping and read projection. It does not add browser
commands for editing skills, inventory, growth, or pets. Future commands require
controlled before/after runtime evidence, an idempotency key, transactional
validation, and a protocol version change.

## Explicitly Unresolved

- live dictionary-key semantics and serialization order;
- generation RNG, rarity/stat distributions, and `AddHunter` arguments;
- cooldown interpretation and remaining skill icon bindings;
- inventory ownership and quantities for any real player;
- growth costs, effects, wallet rules, and learned values;
- riding-pet ownership, pet gear contents, and ranch behavior;
- `SaveData.data` encoding and the serializer call graph.
