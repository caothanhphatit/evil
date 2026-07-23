# ADR-0003: PostgreSQL And Redis

- Status: Accepted
- Date: 2026-07-22

## Context

Accounts, progression, inventory, wallet, purchases, and social state require durable transactions and auditability. Sessions, presence, leases, limits, and hot projections benefit from low-latency ephemeral storage.

## Decision

Use PostgreSQL as the sole durable system of record and Redis for disposable ephemeral coordination/cache concerns. Valuable mutations use database transactions, idempotency, and append-only ledger/audit records.

## Consequences

- Economy invariants can be enforced transactionally and reconciled.
- Redis failure may reduce availability of presence/cache features but cannot lose player value.
- Authenticated command admission fails closed when Redis-backed session resolution, command limits, or player leases are unavailable; see ADR-0006.
- Schema migrations and index/query design require disciplined operational testing.
- JSONB is available for versioned extension data but is not a substitute for stable relational modeling.

## Rejected Alternatives

- Redis as primary game state: rejected because durability and recovery semantics are insufficient for valuable state.
- Document database as primary store: rejected because core economy/social relationships benefit from relational constraints and transactions.
- Event store only: rejected initially due to projection and operational complexity; domain ledgers/events can be added selectively.
