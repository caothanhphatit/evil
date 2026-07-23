# Observability

## Principles

Observability must answer: what happened to this player command, which authoritative decision was made, which content version applied, whether valuable state committed, and where latency occurred. Use structured logs, metrics, distributed traces, and audit records with shared correlation IDs.

## Structured Logging

Log stable event names and fields: timestamp, level, service/version, environment, trace/correlation ID, session ID pseudonym, zone ID, command/event type, content version, outcome/error code, duration, and relevant bounded counts. Avoid raw payloads and personal or secret data.

Economy/security audit records are durable and separate from ordinary diagnostic retention.

## Metrics

### Server

- command accepted/rejected by reason;
- acknowledgement and end-to-end latency histograms;
- simulation tick duration, lag, active zones/entities, queue depth;
- snapshot rate, bytes, compression ratio, dropped/coalesced updates;
- DB/Redis latency, errors, pool saturation, transaction retries;
- reward, spend, reconciliation mismatch, duplicate/idempotent response counts;
- reconnect, resync, disconnect, and version incompatibility rates.

### Client

- boot milestones, bundle/cache hit, decode/load errors;
- FPS/frame-time distribution, long tasks, memory/device tier;
- WebSocket connect/reconnect, RTT, snapshot gap, correction magnitude;
- missing asset/localization key and renderer fallback counts;
- journey failure rates without capturing sensitive user input.

Control label cardinality. Player/entity IDs do not belong in metric labels.

## Tracing

Trace bootstrap and representative commands through gateway, simulation, economy, database, and response. Propagate W3C trace context where possible. Sample ordinary traffic adaptively; retain errors, high latency, and valuable transactions at a higher rate with privacy controls.

## Dashboards And Alerts

Provide dashboards for player experience, simulation health, network, persistence/economy, assets/content release, and infrastructure capacity. Alerts are SLO- or symptom-based, actionable, deduplicated, severity-labelled, and linked to a runbook.

Critical examples: acknowledged transaction loss/reconciliation failure, login unavailable, simulation lag exceeding budget, database saturation, mass asset load failure, and incompatible content release.

## Release Correlation

Every signal includes server build, client build, protocol version, and content/asset manifest version. Deployment annotations make regressions attributable. Canary releases compare error, latency, correction, and economy-integrity metrics before promotion.

## Local Development

Docker development exposes readable structured logs, trace inspection, and a small metrics dashboard. Developers can enable a diagnostics overlay showing FPS, RTT, snapshot/tick versions, interpolation buffer, entity/draw counts, and loaded content release.
