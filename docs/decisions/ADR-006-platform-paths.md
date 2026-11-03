# ADR-006: Platform-Independent Paths

**Status:** Accepted
**Date:** 2026-10-30

## Context

Configuration and data must be stored at appropriate locations per platform.

## Decision

Implement custom path resolution following Horizon's conventions.

## Rationale

- Consistent with Horizon's `data_dir()` implementation
- Honors environment variables (`BELUMI_HOME`, `HORIZON_HOME`, `XDG_*`)
- No external dependency on `dirs` crate
- Testable via explicit directory injection

## Consequences

- Must maintain platform-specific code
- Need to keep in sync with Horizon's path resolution logic