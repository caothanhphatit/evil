# ADR 0009: Tiered Client Simulation Authority

## Status

Accepted

## Context

Running pathfinding, ordinary-monster AI and presentation combat for every
Hunter on the active-session task does not scale to the product target of many
concurrent towns with roughly twenty Hunters each. The product accepts bounded
fraud risk for common farm materials, but does not accept speculative grants or
eventual validation for premium or ownership-changing transactions.

## Decision

Authority is classified by value rather than applied uniformly to every world
field.

- The browser predicts ordinary movement, targeting, combat presentation and
  common-farm outcomes from a server-issued revision and rule version.
- The browser periodically submits bounded farm-report deltas. Reports are
  sequenced, deduplicated, rate limited and queued for asynchronous validation.
- Validation clamps or rejects implausible elapsed time, distance, damage,
  kill-rate and common-material claims. Repeated violations create an audit
  record and a ten-minute login cooldown.
- A farm report never grants premium currency, paid items, rare/event items,
  Hunters, gacha results, entitlements or player-to-player value.
- Payment, premium currency, gacha, Hunter ownership, protected-item ownership
  and player trading remain synchronous server-authoritative commands. The
  browser shows pending state and commits only after the PostgreSQL transaction
  and idempotency ledger commit.
- A protected transaction may consume common farm value only after the
  relevant queued reports have reached an accepted durable revision.
- Redis Streams is the initial farm-validation queue because Redis is already
  an ephemeral runtime dependency. PostgreSQL remains the system of record.

## Consequences

The world protocol must identify the authority class of projected fields and
carry report-window/revision metadata. Farm validation workers may scale
independently and apply backpressure without delaying premium transactions.
Queue loss may require a client resubmission but must never create value.

This ADR supersedes the blanket statement that all ordinary movement and
common-farm simulation outcomes are computed by the Rust world loop. It does
not weaken server authority for valuable economy, RNG or ownership changes.
