# Belumi Architecture

## Overview

Belumi is a modular extension layer built on top of [Horizon](https://github.com/horizon-rust/horizon),
an open-source Minecraft: Java Edition client and server rewritten in Rust. Belumi adds
Quality-of-Life features, a modding runtime, and rendering extensions without duplicating
Horizon's core functionality.

## Design Principles

1. **Composition over modification:** Belumi extends Horizon through composition, not
   by modifying Horizon's core code.
2. **No empty crates:** Crates are only created when they contain real functionality.
3. **Minimal dependencies:** Only justified dependencies are included.
4. **Type safety:** Configuration and APIs are strongly typed with validation.
5. **Platform independence:** Code works on Linux and Windows (macOS where possible).

## Current Structure (v0.1.0)

```
Belumi/
├── Cargo.toml              # Workspace root
├── crates/
│   └── belumi-core/          # Core: config, logging, paths
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── config.rs   # Typed configuration (TOML)
│           ├── error.rs    # Error types
│           ├── logging.rs  # tracing setup
│           └── paths.rs    # Platform-independent paths
├── docs/
│   ├── ARCHITECTURE.md
│   ├── DECISIONS.md
│   ├── ROADMAP.md
│   └── UPSTREAM.md
├── vendor/
│   └── horizon/            # Git submodule (Horizon upstream)
├── .github/
│   └── workflows/
│       └── ci.yml
├── .gitignore
├── .editorconfig

### Planned: belumi-protocol-compat

| Crate | Version | Purpose |
|-------|---------|---------|
| `belumi-protocol-compat` | 0.3.0+ | Protocol translation for backward compatibility with older Minecraft servers |

**Design:** Optional (feature-gated) crate that translates between Minecraft
protocol versions. Depends on `horizon-protocol` (for 26.3 types) but does not
modify its internals. Supports staged compatibility: 26.2 first, then 26.1,
then older versions.

**Key separation:** Native client version (26.3), target server version (26.2/26.1/etc.),
and protocol version (777/etc.) are tracked independently.
├── rustfmt.toml
├── clippy.toml
├── README.md
├── LICENSE
├── CHANGELOG.md
└── CONTRIBUTING.md
```

## Module Responsibilities

### belumi-core

The foundation crate providing:

- **Configuration management:** Typed, validated TOML configuration with sensible defaults

## Launcher Architecture (Planned)

The Belumi Launcher is a standalone application, not a fork of Horizon's launcher.
It manages installation, authentication, profiles, instances, and updates.

### Four Logical Areas

1. **Belumi Launcher:** Installation, authentication, updates, profiles, instances
2. **Belumi Client:** Minecraft communication, game state, rendering, client features
3. **Belumi Platform:** Plugins, configuration, extension interfaces, developer tools
4. **Belumi Community:** Themes, extensions, resources, Easter eggs, community contributions

### Launcher Technology Candidates

| Technology | Strengths | Considerations |
|-----------|-----------|---------------|
| Tauri (Rust + TS) | Flexible UI, web tech for layouts | WebView implementation varies |
| Slint (Rust) | Declarative UI, native Rust integration | Component availability |
| Iced (Rust) | Rust-oriented, async support | Layout and animation capabilities |
| egui (Rust) | Fast prototyping | May not carry high-quality launcher UI |

Decision pending (ADR-007). Evaluation based on 10 criteria including startup
time, RAM usage, animation smoothness, cross-platform support, and testability.

### Shared Interfaces

Defined formats and APIs for:
- Version and release metadata
- Instance and profile configuration
- Start parameters and start requests
- Progress and status messages
- Error reports and logs
- Plugin and resource metadata

Launcher and client should not depend on each other's internal implementation
details. Changes to client architecture should not unnecessarily destabilize
the launcher.
- **Logging:** Structured logging via `tracing` with JSON support and `RUST_LOG` override
- **Path resolution:** Platform-independent data and config directories
- **Error handling:** Unified error types with descriptive messages

### Future Crates (Planned)

The following crates are planned for future versions but are not yet created,
as per the project rule of not creating empty crates:

| Crate | Version | Purpose |
|-------|---------|---------|
| `belumi-render` | 0.3.0 | Rendering pipeline extensions (shaders, effects) |
| `belumi-mods` | 0.4.0 | Plugin/mod runtime (WASM, Lua) |
| `belumi-api` | 0.4.0 | Public extension interfaces for mod authors |
| `belumi-qol` | 0.2.0 | Quality-of-Life features |
| `belumi-launcher` | 0.2.0 | Client launcher and configuration |

## Horizon Integration

Belumi integrates Horizon as a Git submodule at `vendor/horizon`. Belumi crates
reference Horizon crates via path dependencies when needed:

```toml
[dependencies]
horizon-core = { path = "../vendor/horizon/crates/horizon-core" }
```

For v0.1.0, `belumi-core` does not depend on any Horizon crates. It provides
independent infrastructure that will be used by future Belumi crates when they
integrate with Horizon.

## Configuration

Belumi uses TOML for configuration, stored at platform-appropriate paths:

- **Linux:** `~/.config/belumi/belumi.toml`
- **macOS:** `~/Library/Application Support/Belumi/belumi.toml`
- **Windows:** `%APPDATA%\Belumi\belumi.toml`

Environment variable overrides:
- `BELUMI_HOME`: Override data directory
- `BELUMI_CONFIG_HOME`: Override config directory
- `RUST_LOG`: Override log level filter

## Logging

Belumi uses `tracing` for structured logging, bridging Horizon's `log` crate
messages via `tracing-log`. This ensures unified output even when Horizon's
code (which uses `log`) is integrated.

## Testing

- Unit tests in each crate's `src/` modules
- Integration tests in `tests/` directories
- CI runs `cargo test`, `cargo clippy`, and `cargo fmt --check`