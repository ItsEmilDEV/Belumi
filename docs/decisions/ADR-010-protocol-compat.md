# ADR-010: Minecraft Backward Compatibility

**Status:** Accepted (Design Phase)
**Date:** 2026-10-30

## Context

Belumi is built on Minecraft Java Edition 26.3 but should support connections
to older Minecraft Java servers, comparable to ViaFabricPlus. This is a binding
product requirement.

## Decision

1. **Separate version concepts:** Native client version (26.3), target server
   version (26.2, 26.1, etc.), and protocol version (777, etc.) are tracked
   independently.

2. **Modular compatibility layer:** Create `belumi-protocol-compat` crate that
   depends on `horizon-protocol` but does not modify it. Feature-gated in Belumi Client.

3. **Staged approach:** Start with 26.2, then 26.1, then evaluate older versions.

4. **No code reuse from ViaVersion/ViaFabric/ViaFabricPlus:** Concepts may be
   studied, but all implementation must be original Rust code. ViaFabricPlus
   (GPL-3.0) is license-incompatible with Belumi (MIT).

5. **Translation pipeline:** Version detection → packet translation → registry
   mapping → data translation → gameplay corrections.

## Rationale

- Horizon's protocol is tightly coupled to 26.3 (hardcoded packet IDs, registry IDs,
  item components, entity data, chunk format)
- A separate crate allows optional loading and clean separation
- Staged approach manages risk and effort
- License compliance prevents GPL contamination

## Consequences

- Requires version-specific data (version.json, registries.json) for each target
- Translation overhead needs benchmarking
- Game logic may need version-specific corrections
- Launcher must display compatibility status per instance