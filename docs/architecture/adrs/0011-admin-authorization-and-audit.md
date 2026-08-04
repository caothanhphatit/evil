# ADR 0011: Administrative Authorization And Audit Boundary

- Status: accepted
- Date: 2026-08-04

## Context

The operations console is a separate web application, but its catalog and
future content mutations are served by the authoritative Rust server. Basic
Auth is a temporary deployment identity and must not become an unbounded,
unaudited write surface.

## Decision

- All `/admin` routes pass through server-side Basic Auth and a Redis-backed
  actor rate limit. Credentials are compared using fixed-size SHA-256 digests.
- `viewer` can read safe methods. `operator` and `admin` may use future
  `POST`, `PUT`, `PATCH`, and `DELETE` routes.
- Unsafe methods require a ten-minute HMAC-SHA256 CSRF token bound to the
  username and role. The token is issued only over authenticated HTTPS in
  deployment and is never cached.
- Before an unsafe handler runs, the server creates a durable audit row. A
  missing PostgreSQL audit backend fails closed; completion records the HTTP
  status and leaves a pending row if completion itself fails.
- The `/admin/audit` read model includes command, reward, and admin mutation
  events without exposing passwords or request bodies.

## Consequences

This is intentionally a transitional identity layer. Production still needs a
proper identity provider, secret rotation, and TLS-only deployment. Admin CRUD
handlers must keep their domain transaction and audit semantics aligned; the
middleware prevents unaudited requests from reaching them but cannot make a
multi-table content edit atomic by itself.
