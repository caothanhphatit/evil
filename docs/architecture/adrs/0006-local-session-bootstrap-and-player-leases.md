# ADR-0006: Local Session Bootstrap And Player Leases

- Status: Accepted for local educational runtime
- Date: 2026-07-23

## Context

The first slice used one fixed player UUID and guarded ownership with an in-process set. That fixture let every browser share one identity and could not prevent two server replicas from simulating or persisting the same player.

This clean-room rebuild must not connect to the original service, import original accounts, or reuse any original credential, token, certificate, or signing material. A production identity provider is outside the current migration phase.

## Decision

`POST /session/bootstrap` creates or resumes a local educational session. The server generates an opaque random session token and a separate random player UUID, stores their mapping in Redis, and returns only the token as an `HttpOnly; SameSite=Strict` cookie. WebSocket identity is always resolved from Redis; query parameters and client payloads cannot select a player.

Before loading player state, a server replica acquires a Redis lease for that player. Lease acquisition issues a monotonic fencing token. The owner renews the lease while connected and releases it with compare-and-delete semantics. PostgreSQL writes require both the expected state revision and a fencing token no older than the last accepted token, so an expired replica cannot overwrite a newer owner.

Redis also enforces a bounded per-session command budget. Readiness checks both PostgreSQL and Redis because authenticated command admission is unsafe without either dependency.

An in-memory implementation remains available only when dependencies are deliberately omitted in unit tests. Docker configures both durable and coordination adapters.

## Browser Contract

1. Call `POST /session/bootstrap` with `credentials: "include"`.
2. Require HTTP 200 before opening the WebSocket.
3. Open `/ws` on the same site; browsers attach the HttpOnly cookie automatically.
4. Never store, read, or send a player UUID or session token from JavaScript.
5. If bootstrap or WebSocket upgrade returns 401/503, stop command admission and retry bootstrap with bounded backoff.

## Limits And Production Gate

This is pseudonymous local identity, not production account authentication. Before public deployment, integrate an approved identity provider, add session rotation/revocation and CSRF protection for state-changing HTTP routes, terminate TLS, set `SESSION_COOKIE_SECURE=true`, apply bootstrap abuse controls, and define account recovery/deletion. Redis persistence is not identity durability; losing local Redis sessions signs players out but does not delete PostgreSQL game state.

## Consequences

- Multiple replicas cannot concurrently own one player under normal Redis/PostgreSQL operation.
- Stale checkpoints fail closed with a revision conflict.
- Redis loss makes the server unready instead of silently weakening ownership or rate limiting.
- Local browser identity is isolated from the original game backend and credentials.
