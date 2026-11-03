# Horizon Upstream Analysis

## Overview

Belumi builds upon [Horizon](https://github.com/horizon-rust/horizon), an open-source
Minecraft: Java Edition 26.3 client and server rewritten in Rust. Horizon is licensed under
the MIT license and achieves 100% vanilla parity through 300+ differential test harnesses.

- **Repository:** https://github.com/horizon-rust/horizon
- **Website:** https://playhorizon.dev/
- **License:** MIT (Copyright 2026 Ludovic Marie and the Horizon contributors)
- **Minecraft target:** Java Edition 26.3
- **Rust edition:** 2024
- **Minimum Rust version:** 1.85
- **Platforms:** Linux, macOS, Windows

## Workspace Structure

Horizon uses a Cargo workspace with 32 crates. The launcher and telemetry server are separate
workspaces.

### Core Crates

| Crate | Responsibility |
|-------|---------------|
| `horizon-core` | Simulation foundations: math, noise, RNG, geometry (vanilla parity) |
| `horizon-world` | World generation, chunks, blocks, biomes, terrain |
| `horizon-items` | Blocks and items: block facts, item data, enchantments, cooldowns |
| `horizon-command` | Command parsing (Brigadier): tree, args, selectors, suggestions |
| `horizon-protocol` | Network protocol and packets (play, login, configuration) |
| `horizon-assets` | Asset loading and management |

### Client Crates

| Crate | Responsibility |
|-------|---------------|
| `horizon-client` | Main game binary: gameplay, networking, title screens, dedicated server |
| `horizon-client-core` | Shared client core types |
| `horizon-client-input` | Input handling and window management |
| `horizon-client-render` | Rendering, meshing, lighting, HUD, menus, audio, shaders |

### Feature Crates

| Crate | Responsibility |
|-------|---------------|
| `horizon-mods` | Mod files: content mods (.hmod), modpacks, registry index |
| `horizon-shaders` | Shader pack format (Iris/OptiFine compatibility) |
| `horizon-shader-api` | Horizon's own shader packs (WGSL, stages, validation) |
| `horizon-theme` | UI theme, fonts, icons, design system |
| `horizon-hdr` | HDR display support |
| `horizon-dualsense` | DualSense controller support |
| `horizon-threads` | Thread pool and concurrency utilities |
| `horizon-prepare` | Resource preparation from Minecraft jars |
| `horizon-telemetry` | Telemetry and benchmarking |
| `horizon-bench` | Benchmarking utilities |
| `horizon-bots` | Bot/AI for testing |
| `horizon-server` | Dedicated server implementation |
| `horizon-server-files` | Server file management |
| `horizon-friends` | Friends and social features |
| `horizon-relay` | Relay/networking for multiplayer |
| `horizon-port-pack` | Port pack generation |
| `horizon-release-sign` | Release signing |

### Tooling

| Component | Responsibility |
|-----------|---------------|
| `xtask` | Build tasks and automation |
| `launcher/` | Separate workspace: Horizon launcher (gpui-based) |
| `cargo-horizon-mod` | CLI tool for creating content mods |

## Engine and Architecture

### Rendering

- **Backend:** wgpu 30.0.1 (Vulkan, Metal, DirectX 12, OpenGL ES)
- **Shader language:** WGSL (via naga for translation)
- **Features:** Chunk meshing, occlusion culling, translucency sorting, LOD, HDR
- **Shader packs:** Two formats supported:
  1. Iris/OptiFine format (GLSL, via naga translation)
  2. Horizon's own format (WGSL, with stages: sky, material, post)

### Networking

- Full Minecraft Java protocol implementation
- Multiplayer client and dedicated server
- LAN discovery and hosting
- Friends and relay system

### Modding System

Horizon has two mod types:

1. **Content mods (.hmod):** Data and assets only (blocks, items, recipes, etc.)
   - No code execution
   - Loaded at game start
   - Managed via launcher's Mods page

2. **Native mods (.hnmod):** Mixin bundles that modify Rust source before compilation
   - Uses horizon-mixins (MIT, by levzzz5154)
   - Requires recompilation
   - Full system access (trusted code)

WebAssembly mods were previously supported but have been removed. The mod system
is off by default to maintain vanilla parity.

### Configuration

- Configuration stored in the game folder (`options.txt`, `horizon-mods.txt`)
- Platform data directory resolution: `APPDATA` (Windows), `XDG_DATA_HOME`/`~/.local/share` (Linux), `~/Library/Application Support` (macOS)
- Environment variable overrides: `HORIZON_HOME`, `--home DIR`

### Logging

- Uses the `log` crate facade (not `tracing`)
- Custom log4j-style logging with file rollover
- Logs stored in `logs/latest.log` with daily rotation
- Crash reports in `crash-reports/`
- Format: `[%d{HH:mm:ss}] [%t/%level]: %msg%n`

### Asset Management

- Resources prepared from user's own Minecraft 26.3 installation
- `horizon-prepare` crate handles preparation
- No Mojang assets shipped with the project

## Extension Points

Horizon provides several extension mechanisms:

1. **Content mods:** Add blocks, items, recipes, loot tables, tags, models, textures
2. **Native mods:** Modify game source via mixin hooks
3. **Shader packs:** Custom WGSL shaders for sky, materials, post-processing
4. **Resource packs:** Standard Minecraft resource pack format
5. **Hook points:** `native_hooks.rs` provides stable hook functions for native mods

## Identified Gaps for Belumi

Based on the analysis, Horizon does not currently provide:

1. **Plugin runtime with WASM:** WebAssembly mods were removed; a new sandboxed runtime could be added
2. **Lua scripting:** No Lua integration exists
3. **Quality-of-Life features:** Limited QoL features (minimap, zoom, FPS counter are native mods)
4. **Modern UI framework:** Uses custom rendering, no modern UI toolkit
5. **Shader integration in pipeline:** Shaders are packs, not integrated into the core pipeline

## Integration Strategy

### Chosen Approach: Git Submodule

Belumi integrates Horizon as a **Git submodule** at `vendor/horizon`. This approach:

- Keeps Horizon's code separate and updatable
- Allows Belumi crates to reference Horizon crates via path dependencies
- Enables straightforward upstream updates via `git submodule update --remote`
- Avoids copying Horizon's code into Belumi's repository
- Maintains clear attribution and license compliance

### Upstream Update Procedure

1. Check for new Horizon commits: `git submodule status`
2. Update submodule: `git submodule update --remote vendor/horizon`
3. Review changes in Horizon's CHANGELOG and commit messages
4. Run Belumi's test suite to detect breaking changes
5. Update Belumi's integration code if Horizon APIs changed
6. Commit the submodule pointer update

### Path Dependency Resolution

Belumi crates can reference Horizon crates via path dependencies:

```toml
[dependencies]
horizon-core = { path = "../vendor/horizon/crates/horizon-core" }
```

Note: Horizon crates use `edition.workspace = true` and `license.workspace = true`,
which reference Horizon's workspace. When used as path dependencies from Belumi's
workspace, Belumi must define its own workspace settings.

### License Compliance

- Horizon is MIT licensed; Belumi is also MIT licensed
- Horizon contains no Minecraft code or assets
- Third-party dependencies retain their own licenses
- Belumi must not ship Mojang assets or decompiled source
- Third-party notices from Horizon must be preserved

## Verified Information Sources

All information in this document was verified by direct inspection of the Horizon
repository at commit `fca2169de` (main branch, 2026-10-30). No APIs or structures
were invented or assumed.