# ADR-0001: Rust Authoritative Server

- Status: Accepted
- Date: 2026-07-22

## Context

The rebuild needs deterministic game simulation, strong authority over valuable state, predictable latency, safe concurrency, and efficient hosting. The original appears client-heavy, but reproducing that trust model would make a browser release easy to manipulate.

## Decision

Use Rust for the authoritative server. The server owns simulation, game time, RNG, combat outcomes, drops, inventory, economy, progression, and durable state. Begin as a modular monolith with explicit domain crates/modules and extract services only after evidence demonstrates the need.

## Consequences

- Strong type and memory safety, controlled performance, and efficient active-zone simulation.
- Shared authority rules stay centralized and testable.
- Hiring/onboarding and compile times may be harder than Java; conventions and modular boundaries must remain simple.
- Browser prediction is presentation only and reconciles to server state.
- Deterministic math, injected clocks/RNG, and bounded asynchronous work are mandatory.

## Rejected Alternatives

- Java: operationally mature and viable, but Rust is selected for tighter control of simulation cost and memory while retaining safety.
- Client-authoritative TypeScript: fastest prototype, rejected for economy and competitive integrity.
- Immediate microservices: rejected because it adds distributed consistency and operations cost before scale is known.
