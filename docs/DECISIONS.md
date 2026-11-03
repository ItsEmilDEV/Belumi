# Architecture Decisions

## ADR-001: Git Submodule for Horizon Integration

**Status:** Accepted

**Context:** Belumi needs to integrate with Horizon's codebase. Multiple
integration strategies were considered.

**Decision:** Use a Git submodule at `vendor/horizon`.

**Rationale:**
- Keeps Horizon's code separate and clearly attributed
- Enables straightforward upstream updates via `git submodule update --remote`
- Avoids copying Horizon's code into Belumi's repository
- Maintains clear license boundaries
- Allows Belumi to pin to specific Horizon commits for reproducibility

**Alternatives considered:**
- Git dependency: Not feasible as Horizon crates are not published to crates.io
- Fork with upstream remote: More complex merge process, harder to track changes
- Copying code: Violates "do not blindly copy" principle, creates maintenance burden

**Consequences:**
- Developers must initialize submodules: `git submodule update --init`
- Path dependencies must account for submodule location
- Horizon's workspace settings don't propagate to Belumi's workspace

## ADR-002: belumi-core as Independent Foundation

**Status:** Accepted

**Context:** The foundation crate needs to provide configuration, logging, and path
resolution without depending on Horizon.

**Decision:** `belumi-core` is independent of Horizon crates for v0.1.0.

**Rationale:**
- Avoids pulling in Horizon's heavy dependencies (wgpu, etc.) for basic functionality
- Keeps the foundation lightweight and fast to compile
- Allows `belumi-core` to be used by tools and tests without Horizon
- Horizon integration will be added in later crates (`belumi-render`, `belumi-mods`, etc.)

**Consequences:**
- Some code (like platform path resolution) may duplicate Horizon's logic
- Future integration work needed to connect Belumi crates with Horizon

## ADR-003: TOML for Configuration

**Status:** Accepted

**Context:** Need a configuration format that is human-readable and type-safe.

**Decision:** Use TOML with serde for configuration serialization.

**Rationale:**
- Consistent with Horizon's mod manifest format (TOML)
- Human-readable and editable
- Strong typing via serde derive macros
- Well-supported in Rust ecosystem
- Comments allowed in config files

**Alternatives considered:**
- JSON: Less human-friendly, no comments
- YAML: More complex, whitespace-sensitive
- RON: Less familiar to users

## ADR-004: tracing for Logging

**Status:** Accepted

**Context:** Horizon uses the `log` crate. Belumi needs structured logging.

**Decision:** Use `tracing` for Belumi's logging, bridging Horizon's `log` messages
via `tracing-log`.

**Rationale:**
- `tracing` provides structured, span-based logging
- `tracing-log` bridges `log` crate messages into `tracing`
- Compatible with Horizon's existing `log` usage
- Supports JSON output and environment filter override
- Industry standard for Rust async applications

**Consequences:**
- Additional dependency (`tracing-log`) for bridging
- Two logging systems coexist (Horizon's `log` and Belumi's `tracing`)

## ADR-005: No Empty Crates

**Status:** Accepted

**Context:** The architecture proposes several crates (belumi-render, belumi-mods, etc.)
that are not needed for v0.1.0.

**Decision:** Only create crates when they contain real functionality.

**Rationale:**
- Avoids placeholder code that pretends to implement functionality
- Keeps the codebase honest and maintainable
- Reduces compilation time
- Prevents premature abstraction

**Consequences:**
- Architecture documentation lists planned crates
- Crates will be created incrementally as features are implemented

## ADR-006: Platform-Independent Paths

**Status:** Accepted

**Context:** Configuration and data must be stored at appropriate locations per platform.

**Decision:** Implement custom path resolution following Horizon's conventions.

**Rationale:**
- Consistent with Horizon's `data_dir()` implementation
- Honors environment variables (`BELUMI_HOME`, `HORIZON_HOME`, `XDG_*`)
- No external dependency on `dirs` crate
- Testable via explicit directory injection

## ADR-007: Launcher Technology Evaluation (Pending)

**Status:** Pending

**Context:** Belumi needs its own standalone launcher. Multiple UI technologies
are candidates.

**Candidates:**
1. **Tauri (Rust + TypeScript):** Flexible modern desktop UIs, web technologies
   for complex layouts, Rust for installation logic.
2. **Slint (Rust):** Declarative UI with native Rust integration, cross-platform.
3. **Iced (Rust):** Rust-oriented architecture, async support.
4. **egui (Rust):** Fast prototyping for settings and developer tools.

**Evaluation criteria:**
1. Startup time and idle RAM usage
2. Smoothness of animations and interactions
3. Quality and freedom of UI design
4. Windows, Linux, and macOS support
5. Maintainability and ecosystem size
6. Accessibility and keyboard navigation
7. System integration and process control
8. Testability and automated builds
9. Dependencies and license conditions
10. Development effort for the existing project

**Decision:** Pending. A documented comparison and small prototype per serious
candidate will be created before the final decision.

**Rationale:** Technology is not selected based on language or hype alone.
The launcher needs a high-quality UI and reliable system integration.

## ADR-008: Standalone Launcher (Not Horizon Fork)

**Status:** Accepted

**Context:** Belumi could either fork Horizon's launcher or build its own.

**Decision:** Belumi will have its own standalone launcher, not a fork of Horizon's.

**Rationale:**
- Own branding and design system
- Independent development and testing
- No dependency on Horizon's launcher internals
- Full control over UX and features

**Consequences:**
- Must implement all launcher functionality from scratch
- Need to define clear interfaces between launcher and client
- More development effort but greater long-term flexibility

## ADR-009: Four Logical Areas

**Status:** Accepted

**Context:** Belumi is a complex ecosystem with multiple components.

**Decision:** The product is divided into four logical areas:

1. **Belumi Launcher:** Installation, authentication, updates, profiles, instances
2. **Belumi Client:** Minecraft communication, game state, rendering, client features
3. **Belumi Platform:** Plugins, configuration, extension interfaces, developer tools
4. **Belumi Community:** Themes, extensions, resources, Easter eggs, community contributions


## ADR-010: Minecraft Backward Compatibility

**Status:** Accepted (Design Phase)
**Date:** 2026-10-30

**Context:** Belumi is built on Minecraft Java Edition 26.3 but should support
connections to older Minecraft Java servers, comparable to ViaFabricPlus.

**Decision:**
1. Separate version concepts (native, target, protocol)
2. Modular compatibility layer (`belumi-protocol-compat`)
3. Staged approach (26.2 → 26.1 → older)
4. No code reuse from ViaVersion/ViaFabric/ViaFabricPlus (GPL incompatibility)
5. Translation pipeline: detection → packet → registry → data → gameplay

**Rationale:** Horizon's protocol is tightly coupled to 26.3. Separate crate
allows optional loading. License compliance prevents GPL contamination.

**Consequences:** Requires version-specific data, translation overhead needs
benchmarking, launcher must display compatibility status.
**Rationale:**
- Clear separation of concerns
- Independent development and testing
- Well-defined interfaces between areas
- Scalable architecture

**Consequences:**
- Need to define and maintain interfaces between areas
- Some shared functionality may require common libraries

**Consequences:**
- Must maintain platform-specific code
- Need to keep in sync with Horizon's path resolution logic