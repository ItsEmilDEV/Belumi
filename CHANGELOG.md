# Changelog

All notable changes to Belumi are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added

- Client: Belumi Settings page with `belumi.toml`; HUD overlay, zoom, minimap, persistent world map, waypoints, freecam,
  packet inspector and built-in screen effects ([docs/client-features.md](docs/client-features.md))
- Client plugins: WebAssembly host, API, SDK and an example ([docs/client-plugins.md](docs/client-plugins.md))
- Voice chat compatible with Simple Voice Chat servers ([docs/voice-chat.md](docs/voice-chat.md))
- Launcher: game version per server (26.3 native, 26.2 translated), update check with stable and nightly channels,
  Linux release archive with installer and a release workflow ([docs/releases.md](docs/releases.md))
- Initial project structure with Cargo workspace
- `belumi-core` crate with:
  - Type-safe TOML configuration with validation
  - Structured logging via `tracing` with JSON support
  - Platform-independent path resolution
  - Unified error types
- Git submodule integration with Horizon upstream
- Project documentation (README, ARCHITECTURE, ROADMAP, UPSTREAM, DECISIONS)
- CI pipeline (format check, clippy, tests)
- Professional project scaffolding (.gitignore, .editorconfig, rustfmt.toml, clippy.toml)

### Changed

- N/A (initial release)

### Fixed

- N/A (initial release)