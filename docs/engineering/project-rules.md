# Project Rules

## Operating Principles

- Server authority is a product invariant, not an optimization toggle.
- Determinism, auditability, and data integrity outrank convenience.
- Build vertical player value; avoid long-lived disconnected subsystems.
- Prefer a modular monolith until metrics justify distribution.
- Keep source evidence, canonical content, and generated runtime output distinct.
- Make invalid states unrepresentable where practical and reject them at boundaries otherwise.

## Repository Ownership

Use CODEOWNERS or equivalent for server domains, protocol/schema, database migrations, security-sensitive economy code, asset pipeline, and operations. Require at least two reviewers for authentication, authorization, purchases, wallet/inventory ledgers, admin grants, cryptography, and destructive migrations.

Generated files contain a header and are changed only through their generator. Dependency lockfiles, schema changes, and asset manifests are reviewed like source code.

## Branch And Change Policy

- Keep branches short-lived and changes small enough to reason about.
- Use conventional commit intent: `feat`, `fix`, `refactor`, `test`, `docs`, `build`, `chore`.
- A pull request states player impact, authority/data impact, test evidence, migration/rollback, performance impact, security considerations, and screenshots/traces when relevant.
- No direct production changes, hidden feature switches, or undocumented manual database edits.

## Dependency Policy

- Add a dependency only with a documented need, maintenance assessment, license check, and security review proportional to risk.
- Pin versions through lockfiles and container image digests.
- Remove unused dependencies promptly.
- Do not introduce overlapping frameworks for the same responsibility without an ADR.

## Data And Migration Rules

- Database migrations are forward-only, deterministic, and safe for rolling deployment.
- Use expand/migrate/contract for incompatible schema changes.
- Backfills are resumable, observable, rate-limited, and idempotent.
- Never edit player wallet/inventory without a ledger/audit entry and actor/reason.
- Never rely on Redis as the only copy of valuable state.

## Protocol Rules

- Change the schema source before generated Rust/TypeScript types.
- Additive compatible changes are preferred; removals require a deprecation window.
- Commands have sequence and idempotency semantics.
- Payload sizes, collection lengths, string lengths, and command rates are bounded.
- Internal errors are mapped to stable public error codes without leaking secrets.

## Asset And Content Rules

- Preserve original extracted evidence as read-only.
- Track every approved source object in the manifest.
- All transformations are scripted and reproducible.
- Content releases are immutable after publication.
- Missing references, duplicate IDs, unsupported effects, and unreviewed licenses block promotion.

## Environment Rules

- `docker compose up` is the supported local integration path.
- Configuration comes from validated environment variables or mounted secrets, never committed credentials.
- Development defaults are safe and use fake external providers.
- Production parity includes database versions, migrations, protocol generation, and observability semantics.

## Quality Gate

Before merge, relevant formatting, linting, unit, integration, contract, migration, security, and asset validation checks pass. Main remains deployable. Flaky tests are treated as defects and assigned an owner; they are not repeatedly rerun until green without diagnosis.

## Documentation Rules

Update documentation in the same change when behavior, operations, architecture, protocol, content schema, or asset handling changes. Record durable architectural decisions as ADRs. Runbooks must be executable by an engineer who did not implement the feature.

## Incident And Security Rules

- Never hide integrity failures or security-relevant events in debug logs only.
- Preserve correlation IDs and audit trails.
- Rotate exposed secrets immediately; do not merely delete them from the latest commit.
- Use blameless incident review focused on system improvement, with explicit actions and owners.
