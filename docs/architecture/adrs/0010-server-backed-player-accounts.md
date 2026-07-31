# ADR-0010: Server-backed player accounts

- Status: Accepted
- Date: 2026-07-30

## Context

The local identity flow stored display names and email addresses in browser
`localStorage`. Durable game state was server-owned, but access to it depended
only on one browser's opaque cookie. A different browser could not recover the
same player, and deleting the cookie orphaned access to the durable world.

## Decision

The rebuild owns first-party email/password accounts in PostgreSQL. Email is
normalized and unique, passwords are stored only as salted PBKDF2-HMAC-SHA256
verifiers, and each normal account owns a stable `player_token`. Registration
and login issue a fresh opaque HttpOnly session cookie and bind its hash to that
player. Multiple browsers may hold sessions for one player, while the existing
Redis lease and PostgreSQL revision/fencing checks still allow only one active
simulation writer at a time.

`POST /session/bootstrap` no longer creates anonymous identities. It accepts
only a previously authenticated cookie. The browser login screen sends account
credentials directly to `POST /account/register` or `POST /account/login`; it
does not store passwords, session tokens or player IDs.

Three local demo credentials own separate player tokens, leases, towns, Hunter
rosters and inventories. Their first authoritative load creates the ordinary
account world and atomically applies the full-stock demo seed. Their common
development password is documented in migration
`0035_real_player_accounts.sql`; these credentials must not be enabled unchanged
in a public deployment.

## Consequences

- Protocol v31 raises the JSON debug message ceiling from 1 MiB to 4 MiB. The
  fully seeded demo inventory produces a roughly 1.84 MiB initial snapshot; the
  client and server retain the same generated cap and fail closed above it.

- A registered player can load the same durable town from another browser.
- Browser-local profile selection is no longer an identity boundary.
- Password recovery, email verification, MFA, password rotation and provider
  login remain production follow-up work.
- Public deployment must replace the development demo credentials and apply
  endpoint rate limiting, TLS and account-abuse controls.
