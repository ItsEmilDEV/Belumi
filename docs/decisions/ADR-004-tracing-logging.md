# ADR-004: tracing for Logging

**Status:** Accepted
**Date:** 2026-10-30

## Context

Horizon uses the `log` crate. Belumi needs structured logging.

## Decision

Use `tracing` for Belumi's logging, bridging Horizon's `log` messages via
`tracing-log`.

## Rationale

- `tracing` provides structured, span-based logging
- `tracing-log` bridges `log` crate messages into `tracing`
- Compatible with Horizon's existing `log` usage
- Supports JSON output and environment filter override
- Industry standard for Rust async applications

## Consequences

- Additional dependency (`tracing-log`) for bridging
- Two logging systems coexist (Horizon's `log` and Belumi's `tracing`)