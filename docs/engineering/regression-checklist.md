# Regression Checklist

Use this checklist for player-facing, authority, persistence, and protocol changes.

## Required gates

- `cargo fmt --manifest-path apps/server/Cargo.toml --check`
- `cargo test --manifest-path apps/server/Cargo.toml`
- `cargo clippy --manifest-path apps/server/Cargo.toml --all-targets -- -D warnings`
- `pnpm test:web`
- `pnpm build:web`
- `pnpm architecture:validate`
- `pnpm test:e2e` against the disposable Docker smoke stack for entry/UI changes

## Critical journeys

- Sign in stays separate from game loading; loading either reaches town or shows a retryable failure.
- Desktop 1366x768 and mobile 393x852 contain the Hunter roster without document overflow.
- Trading Post requests remain open until authoritative acceptance and reject without local mutation.
- Hunter sale settlement happens only at the Trading Post and persists across reconnect.
- Economy commands conserve gold/materials and remain idempotent.
- Protocol changes regenerate both Rust and TypeScript contracts from the schema.

## Migration changes

- Add a matching `.down.sql` for the newest migration.
- Run `MIGRATION_VERIFY_DISPOSABLE=true DATABASE_URL=... sh tools/verify-latest-migration.sh` only against a disposable database.
- Record backup, forward migration, rollback, and data verification steps in the pull request.

## Operational evidence

- Check structured server logs for rejected intents, lease loss, persistence failure, and slow checkpoints.
- Check `evil:telemetry` client events for loading failure, reconnect, protocol fault, and rejected intent.
- Attach screenshots or Playwright traces for responsive UI changes.
