# Original Combat Runtime Integration v1

This slice connects the evidence-backed neutral ordinary-attack spine to the
authoritative Rust simulation. It does not claim that unresolved optional
traits, skills, gear effects or mode modifiers are zero in the original game;
zero and one are used only as the native disabled/identity values for producers
that the rebuild does not yet possess.

## Hunter to monster

The connected order is:

1. use the operational Hunter `attack` projection as already-aggregated
   `StatusData.CalcDamage`;
2. resolve the exact critical threshold `min(100, CalcCritical + bonus)` and
   exclusive Unity-style roll comparison;
3. apply the recovered neutral `1.75x` critical factor when successful;
4. replay the recovered outgoing final truncation chain with unresolved
   optional multipliers at identity;
5. consume the exact persisted 30-entry `GameManager.RandDamage` stream;
6. apply the recovered mood/feel band, monster armor, minimum-one damage and HP
   order;
7. emit authoritative normal or critical combat presentation.

Captured runtime `critical`, `feel` and `nowFeel` values take precedence when
present. Otherwise the operational profile critical basis points and mood gauge
feed the same formula boundary. Those fallback values remain rebuild fixture
inputs, not captured original Hunter values.

The original critical comparison bounds are exact. The original Unity global
PRNG state is not recovered, so the server uses its deterministic authoritative
percent-roll stream. This changes the roll sequence, not the recovered
threshold rule.

## Monster to Hunter

The connected tail consumes the exact RandDamage stream, recovered feel-adjusted
armor, default `0.75` final factor, minimum damage, first-shield routing and HP
floor. No shield state is modeled yet, so the live input is the exact disabled
value `0`.

The native caller multiplies catalog monster damage by a selected runtime factor
whose writer is unresolved. The pre-existing rebuild projection divisor remains
isolated before the recovered tail; raw catalog damage is not falsely claimed
as the value passed directly to `HunterCtrl.Damaged`.

## Hit-result boundary

- Critical damage is now emitted live by the server.
- Effect-54 Miss is implemented in the resolver and proven by tests, but the
  live attacker effect state is not modeled, so its exact disabled value keeps
  it inactive.
- Pass 18 connects total `profile.evasion_rate_bps` to `CalcDodge`, then applies
  the recovered exclusive roll before armor, shield and HP. Missing named
  contribution sources, effect type 5 and riding-pet dodge default to zero.
  The rebuild uses a deterministic uniform roll with the exact native bounds;
  Unity's global PRNG state sequence remains unresolved. See
  `original-native-dodge-consumer-pass18.md`.

## Validation

```sh
cargo test --manifest-path apps/server/Cargo.toml
pnpm test:web
pnpm build:web
```
