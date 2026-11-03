# ADR-005: No Empty Crates

**Status:** Accepted
**Date:** 2026-10-30

## Context

The architecture proposes several crates (belumi-render, belumi-mods, etc.) that
are not needed for v0.1.0.

## Decision

Only create crates when they contain real functionality.

## Rationale

- Avoids placeholder code that pretends to implement functionality
- Keeps the codebase honest and maintainable
- Reduces compilation time
- Prevents premature abstraction

## Consequences

- Architecture documentation lists planned crates
- Crates will be created incrementally as features are implemented
- `docs/ARCHITECTURE.md` documents planned crates and their target versions