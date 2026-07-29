# Tiered Farm Validation Threat Model

## Trust boundary

Farm reports are untrusted claims. Session identity, report sequence, issued
revision, elapsed-time bounds and content/rule version are validated before a
report enters the worker queue.

## Low-value report scope

- ordinary Hunter position and route digest;
- ordinary monster targets, damage totals and kill facts;
- common material claims under server-defined per-window ceilings;
- presentation-only drop positions and animations.

The report schema must not contain premium currency, receipt data, gacha
results, Hunter ownership, protected item IDs or trade settlement outcomes.

## Enforcement

- Duplicate windows return the recorded result.
- Mild deviations are clamped and add risk points.
- Impossible revisions, time travel, protected claims or repeated excessive
  budgets reject the report.
- A confirmed threshold violation terminates the session, records the reason
  and applies a ten-minute login cooldown.
- Security logs store account/session identifiers, rule version, report window,
  reason codes and claimed/accepted totals. They do not store credentials or
  raw payment data.

## Critical transaction rule

Payment, premium currency, gacha, Hunter/protected-item ownership and player
trade bypass the asynchronous farm queue. They validate and commit directly in
one idempotent PostgreSQL transaction before the response is acknowledged.
