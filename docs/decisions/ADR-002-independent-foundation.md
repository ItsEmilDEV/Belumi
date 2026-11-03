# ADR-002: belumi-core as Independent Foundation

**Status:** Accepted
**Date:** 2026-10-30

## Context

The foundation crate needs to provide configuration, logging, and path resolution.
It could depend on Horizon crates or be independent.

## Decision

`belumi-core` is independent of Horizon crates for v0.1.0.

## Rationale

- Avoids pulling in Horizon's heavy dependencies (wgpu, etc.) for basic functionality
- Keeps the foundation lightweight and fast to compile
- Allows `belumi-core` to be used by tools and tests without Horizon
- Horizon integration will be added in later crates (`belumi-render`, `belumi-mods`, etc.)

## Consequences

- Some code (like platform path resolution) may duplicate Horizon's logic
- Future integration work needed to connect Belumi crates with Horizon