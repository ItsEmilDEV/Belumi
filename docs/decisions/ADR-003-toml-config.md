# ADR-003: TOML for Configuration

**Status:** Accepted
**Date:** 2026-10-30

## Context

Need a configuration format that is human-readable and type-safe.

## Decision

Use TOML with serde for configuration serialization.

## Rationale

- Consistent with Horizon's mod manifest format (TOML)
- Human-readable and editable
- Strong typing via serde derive macros
- Well-supported in Rust ecosystem
- Comments allowed in config files

## Alternatives Considered

- **JSON:** Less human-friendly, no comments
- **YAML:** More complex, whitespace-sensitive
- **RON:** Less familiar to users

## Consequences

- Configuration files use `.toml` extension
- serde derive macros required for all config structs