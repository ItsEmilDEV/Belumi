# Changelog

All notable changes to Belumi are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Distant chunks (Bobby alternative): columns a server forgot stay drawn out to your render distance and are cached on disk
  for the next visit (`belumi-chunks/`, 1 GB limit); switch on the Modules page

- Changes since your last visit: cached and fresh columns are compared on a server, listed on a page, one click marks them
- Way back: a compass HUD module (`/compass`), a breadcrumb trail (`/trail`) and automatic portal and spawn waypoints
- Client plugins can have their own screens (labels, buttons, toggles): permission `screen`, shown on the client screen
- Switch account in the game (Modules page): the launcher signs in the account you pick and restarts the game on the same server

## [0.1.0] - Unreleased

### Added

- Launcher: update button (download, SHA-256 check, replace, restart), a notification when an update exists, "Check now",
  Discord Rich Presence of the launcher, a Minecraft session cache against login rate limits (429)
- Client screen on Right Shift: HUD builder with undo and per-module backgrounds, new HUD modules (FPS, ping, coordinates),
  effect icons, crosshair presets and colour steps, chat tools, shulker previews in tooltips
- Replays with camera paths (P, O, Backspace)
- Discord Rich Presence with options (server address, world name, dimension, game mode icon, time, player count)
- Voice chat is part of every Belumi build and on by default (push to talk)
- Linux `.deb` package and a Windows installer in the release workflow
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