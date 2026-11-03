# ADR-001: Git Submodule for Horizon Integration

**Status:** Accepted
**Date:** 2026-10-30

## Context

Belumi needs to integrate with Horizon's codebase. Multiple integration
strategies were considered: Git dependency, Git submodule, documented fork with
upstream remote, or other technically justified integration.

## Decision

Use a Git submodule at `vendor/horizon`.

## Rationale

- Keeps Horizon's code separate and clearly attributed
- Enables straightforward upstream updates via `git submodule update --remote`
- Avoids copying Horizon's code into Belumi's repository
- Maintains clear license boundaries
- Allows Belumi to pin to specific Horizon commits for reproducibility
- Horizon crates are not published to crates.io, making Git dependency infeasible

## Alternatives Considered

- **Git dependency:** Not feasible (Horizon crates not on crates.io)
- **Fork with upstream remote:** More complex merge process, harder to track
- **Copying code:** Violates "do not blindly copy" principle, creates maintenance burden

## Consequences

- Developers must initialize submodules: `git submodule update --init`
- Path dependencies must account for submodule location
- Horizon's workspace settings don't propagate to Belumi's workspace