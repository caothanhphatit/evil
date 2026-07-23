# Security And Anti-Cheat

## Trust Model

The browser, local storage, network input, clocks, asset files, and client-reported telemetry are untrusted. The server is authoritative for identity, time, movement validity, combat, RNG, loot, inventory, wallet, progression, rewards, rankings, and persistence.

## Command Model

Clients send bounded intent, not outcomes. The server verifies authentication, authorization, session ownership, protocol/content version, sequence, state preconditions, cooldown/resources, spatial constraints, rate limits, and idempotency before applying a command.

Examples:

- valid intent: equip an owned item on an eligible hunter;
- invalid claim: set damage, gold, item count, completion, server time, or random result.

## Integrity Controls

- Server-owned RNG streams and encounter state.
- Fixed-step authoritative simulation with movement/cooldown plausibility.
- Transactional wallet/inventory ledger and periodic reconciliation.
- Idempotency keys for every valuable command and provider callback.
- Server-time offline progress with caps and monotonic state versions.
- Signed, short-lived sessions; rotation and revocation supported.
- Strict schema validation, message size/collection bounds, and command budgets.
- Rankings derive from authoritative facts, never uploaded scores.

Client obfuscation, WASM, integrity checks, or asset encryption may slow casual tampering but are never treated as authority.

## Application Security

- Follow OWASP ASVS/API guidance appropriate to the exposed surface.
- TLS everywhere outside local development; secure cookie/token handling and origin checks.
- Parameterized SQL and least-privilege database roles.
- Secrets come from a secret manager or mounted runtime secret, not images or source.
- Containers run non-root with minimal capabilities and read-only filesystems where practical.
- Dependency, image, license, SAST, and secret scanning run in CI.
- Administrative endpoints use separate authorization, strong authentication, audit logging, and network controls.

## Abuse Resistance

Apply per-IP, account, session, and command-class rate limits with cautious handling of shared networks. Detect impossible command frequency, repeated invalid transitions, extreme correction, transaction abuse, account sharing indicators, and automation patterns. Use staged responses: observe, throttle, challenge/re-authenticate, restrict valuable actions, then review/ban.

Avoid opaque automatic permanent bans based on one heuristic. Preserve evidence and support appeals/false-positive analysis.

## Current Local Session Control

The educational Docker runtime uses a server-stored opaque session cookie, Redis identity mapping, Redis command budgets, single-owner player leases, and PostgreSQL revision/fencing checks. It never accepts a player identity from WebSocket query parameters or command payloads and does not reuse original-game credentials. This is a safe migration bootstrap, not production login: the production gate still requires a real identity provider, TLS-only secure cookies, rotation/revocation, HTTP CSRF controls, and bootstrap abuse protection.

## Purchases And Ads

Verify receipts or rewarded-ad callbacks with the provider server-to-server. Bind grants to provider transaction IDs and account/product context. Handle retries, refund, revocation, sandbox/production separation, replay, and delayed callbacks. Client success screens never cause grants.

## Threat Modeling Checklist

For each feature identify assets of value, trust boundaries, commands, replay/duplication, concurrency, authorization, input bounds, information disclosure, denial of service, rollback/recovery, audit evidence, and monitoring. Security review is mandatory before exposing new value-changing or administrative commands.

## Privacy

Collect the minimum data required. Define retention and deletion, separate security/audit needs from analytics, pseudonymize operational identifiers, and protect exports/backups. Do not migrate original users or production data.

## Incident Readiness

Maintain procedures for credential exposure, economy exploit, account takeover, malicious content release, dependency compromise, and data loss. Support feature disablement, session revocation, content rollback, transaction reconciliation, evidence preservation, and scoped remediation.
