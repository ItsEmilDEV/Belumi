# Belumi Roadmap

## Overview

Belumi's development is divided into six milestones, from foundation to
first complete release. This roadmap is preliminary and may be adjusted based
on technical findings.

## Version History

### 0.1.0 — Foundation (Current)

**Status:** In progress

**Scope:**
- Repository structure and Cargo workspace
- Git submodule integration with Horizon upstream
- `belumi-core` crate with configuration management, logging, and path resolution
- Type-safe TOML configuration with validation
- Structured logging via `tracing`
- CI pipeline (format, lint, test)
- Project documentation

**Deliverables:**
- Working `belumi-core` crate with tests
- Upstream analysis document
- Architecture and decisions documentation
- Professional project scaffolding

### 0.2.0 — Belumi Experience

**Focus:** Quality-of-Life features and modern user experience

**Planned:**
- `belumi-qol` crate: QoL features (minimap, zoom, FPS counter, coordinates HUD)
- `belumi-launcher` crate: Client launcher and configuration
- Modern UI components
- Settings integration with Horizon's options system
- User preferences management

### 0.3.0 — Rendering

**Focus:** Modern graphics pipeline extensions

**Planned:**
- `belumi-render` crate: Rendering pipeline extensions
- Integrated shader support (beyond pack-based)
- Post-processing effects
- Performance optimizations
- HDR and advanced lighting

### 0.4.0 — Modding

**Focus:** Plugin and mod runtime

**Planned:**
- `belumi-mods` crate: Plugin/mod runtime
- `belumi-api` crate: Public extension interfaces
- WASM sandboxed runtime (wasmtime)
- Lua scripting support (mlua)
- Mod loading and lifecycle management
- Hook point system for mod integration

### 0.5.0 — Ecosystem

**Focus:** Developer tools and community

**Planned:**
- Mod development SDK and tooling
- Documentation for mod authors
- Plugin registry integration
- Community features
- Performance profiling tools

### 1.0.0 — First Complete Release

**Focus:** All planned core features implemented and stabilized

**Criteria:**
- All core features from 0.1.0-0.5.0 complete
- Stable public API
- Comprehensive test coverage
- Performance benchmarks
- Full documentation
- Community-ready

## Versioning Policy

- **0.x.x:** Development versions. Breaking changes allowed between minor versions.
- **1.0.0:** First complete release with all planned core features.
- **1.x.x:** Stable releases. Breaking changes only in major versions.

## Notes

- This roadmap is preliminary and may be adjusted based on technical findings.
- Features may be moved between versions based on dependencies and feasibility.

## Protocol Compatibility (Cross-Cutting Concern)

Belumi aims to support connections to older Minecraft Java servers,
comparable to ViaFabricPlus. This is a binding product requirement.

**Target versions (staged):**
1. Minecraft 26.2 (first target)
2. Minecraft 26.1 (second target)
3. Older releases (1.21.x) — evaluation pending

**Architecture:** `belumi-protocol-compat` crate (optional, feature-gated) that
translates between protocol versions. Does not modify `horizon-protocol`.

**Key separation:** Native client version (26.3), target server version, and
protocol version are tracked independently.

**Technical references:** ViaVersion (MIT), ViaFabric (MIT), ViaFabricPlus (GPL-3.0,
concepts only — no code reuse).

See `docs/wire-differences-26.2.md` for full analysis.
- Community input is welcome via GitHub issues.

---

## Phase-Based Roadmap (Product Vision)

### Phase 0 — Audit and Foundation

**Goal:** Create a technically clean foundation.

- Repository and Rust project structure review
- Build system and test setup
- Minecraft connection and protocol requirements investigation
- Module boundaries and architecture documentation
- Rendering and resource architecture planning
- Error handling, logging, and diagnosis foundations
- Technical risks and dependencies documentation

**Completion criteria:** Project builds and tests reproducibly; central
architecture decisions are traceable.

### Phase 1 — Playable Core

**Goal:** Develop a minimal, working client.

- Connection to supported Minecraft servers
- Basic game world and resource processing
- First working rendering pipeline
- Stable client lifecycle
- Reproducible error diagnosis and basic performance measurements

**Completion criteria:** Supported Minecraft version works in a clearly defined,
tested scope.

### Phase 2 — Customization Core

**Goal:** Belumi should be recognizably customizable early.

- Central configuration system
- Settings UI and themes
- Keybinds and module management
- Profiles and import/export functions
- Foundations for customizable HUD
- First internal interfaces for extensions

**Completion criteria:** Important settings can be changed, saved, and reliably
restored.

### Phase 3 — Extension Platform

**Goal:** Belumi becomes an extension platform.

- Documented plugin API

### Phase 7 — Launcher Foundation (Standalone)

**Goal:** Develop Belumi's own launcher.

- Technology comparison (Tauri, Slint, Iced, egui)
- UI prototype for main view, instance list, and settings
- Instance and profile management
- Download and installation pipeline
- Client start and monitoring
- Logs and error diagnosis
- Update and recovery strategy

**Completion criteria:** A user can manage, configure, start, and diagnose
supported installations.

### Phase 8 — Authentication and Release Process

**Goal:** Secure and platform-independent distribution.

- Official authentication process integration
- Secure token processing
- Release metadata and integrity checks
- Error and recovery case testing
- Cross-platform packaging

**Completion criteria:** Installation and start process works under defined
security and platform requirements.
- Event system and defined extension points
- Plugin lifecycle and error isolation
- Permission model and technical sandbox decision
- Dependency and compatibility management
- Optional expert mode
- Developer documentation and example plugins

**Completion criteria:** External extensions can be developed and tested using
documented interfaces.

### Phase 4 — Rendering Expansion

**Goal:** Bring graphics and customizability to a new level.

- Extended native rendering functions
- Own shader architecture
- Rendering profiles and graphics diagnosis
- Experimental Iris/OptiFine shaderpack compatibility
- Tests with selected shaderpacks
- Performance and stability measurements

**Completion criteria:** Native rendering functions are stable; experimental
compatibility is transparently documented and evaluated against defined test cases.

### Phase 5 — Power-User Ecosystem

**Goal:** Give advanced users extensive control.

- Extended configuration interfaces
- Scripting and automation where technically sensible
- Custom UI components and advanced HUD customization
- Developer tools and diagnosis functions
- Improved plugin developer experience
- Community presets and theme exchange

**Completion criteria:** Users can implement significant customizations without
changes to the core.

### Phase 6 — Community Identity

**Goal:** Belumi receives a unique personality.

- Easter egg system and hidden UI states
- Private insiders and public community secrets
- Optional cosmetic surprises
- Community contributions and credits
- Documented guidelines for additional Easter eggs

This phase can be developed partially in parallel with earlier phases once
necessary architecture foundations exist.