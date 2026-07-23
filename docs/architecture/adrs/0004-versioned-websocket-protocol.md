# ADR-0004: Versioned WebSocket Protocol

- Status: Accepted
- Date: 2026-07-22

## Context

The browser needs low-overhead bidirectional commands, acknowledgements, and state deltas. SSE is one-way and would require a second command transport. Frequent JSON snapshots create avoidable bandwidth and parsing cost.

## Decision

Use HTTPS for bootstrap and ordinary APIs, plus a versioned binary WebSocket protocol for live play. Generate Rust and TypeScript contracts/codecs from one schema source. Envelopes include version, session, sequence, correlation, type, and payload.

The binary encoding is the production target, not the encoding currently used by Slice 1. Slice 1 deliberately uses a transitional JSON debug codec declared by `packages/protocol/world-v1.schema.json` as `wire.encoding = "json-debug"`. The generated Rust and TypeScript contracts still define the wire shape from that single schema.

The current debug envelope contains `version`, `sequence`, `session_id`, `correlation_id`, and a tagged `payload`. It is bounded to 16 KiB per message. The server accepts only the exact implemented protocol version and the next client sequence for the active session; the client likewise rejects duplicate, skipped, or out-of-order server sequences. Reconnect establishes a new session and ordered stream, followed by an explicit resync.

JSON debug transport must not be presented as the production hot path. Promotion to a production transport requires all of the following:

- a generated binary codec for Rust and TypeScript from the same schema source;
- compatibility fixtures and a declared supported-version window;
- malformed-frame, message-size, sequencing, resync, and backpressure tests;
- measured snapshot/delta bandwidth, encode/decode cost, and server/client memory use;
- Docker end-to-end coverage across reconnect and incompatible-version cases.

Until those gates pass, protocol changes preserve the versioned envelope semantics and JSON remains an explicit development implementation.

Protocol v5 adds a server-owned visual world projection and navigation/selection intents. This projection deliberately carries no client-authored outcomes and no combat or economic values. Entity selection is ephemeral; only screen state is persisted. Adding visible entities therefore does not move simulation authority into the browser or promote unresolved field gameplay bindings.

## Consequences

- Efficient bidirectional updates and explicit reconnect/resync semantics.
- Schema generation, fixture compatibility, backpressure, bounds, and observability require dedicated infrastructure.
- Server snapshot rate remains independent from client render rate.
- A declared client compatibility window and content-version pinning are required.
- Slice 1 pays JSON serialization and full-snapshot costs while the binary codec and delta protocol remain release blockers.

## Rejected Alternatives

- SSE plus HTTP commands: rejected for live command overhead and split semantics; may still be used for non-game operational feeds if justified.
- JSON WebSocket snapshots: retained as the bounded Slice 1 debug implementation, rejected as the production hot path.
- WebTransport: deferred until browser/support and operational benefits outweigh complexity.
