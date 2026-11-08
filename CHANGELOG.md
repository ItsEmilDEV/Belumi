# Changelog

All notable changes to Belumi are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Distant chunks (Bobby alternative): columns a server forgot stay drawn out to your render distance and are cached on disk
  for the next visit (`belumi-chunks/`, 1 GB limit); switch on the Modules page

- Riptide fix: Depth Strider no longer slows a Riptide launch (vanilla bug MC-136249)
- Ender chest tooltip: the item shows what the ender chest held; chest memory follows items moved while open
- Sound controller: mute or turn down single sounds
- Switch notices on the HUD, in chat or off; warnings when a switch could be seen as cheating
- Blueprint: `rotate` and `mirror x|z`, with the block states turned along
- Inspect looks 20 blocks far, shows players, objects, the tool in hand and the break time
- Gamma boost beyond the slider with keys for it and for fullbright
- Inspect HUD module (Jade): name, hardness, state and remembered chest contents of the block in the crosshair, name and health of a creature
- Light overlay: red and yellow Xs where monsters can spawn
- Accurate placement (off by default): fast block placing with the right button held and no creative break pause, with the vanilla wait kept for containers, doors and scaffolding
- Blueprint: `.schem` and `.litematic` plans as outlines of missing and wrong blocks, with a material list (`/blueprint`)
- Pings (Z) and shared waypoints (`/sharewp`, `/acceptwp`) between Belumi players on a Horizon server
- Replay video: F7 saves the replay as pictures and makes an MP4 with ffmpeg; `/replay play N`
- Redstone workbench: signal numbers over dust, /tick buttons and a Profiler HUD module with the server tick's biggest parts
- World history and `/rollback RADIUS MINUTES`: older chunk versions are kept, and the chunks around you go back to how they were
- Sound physics (off by default): the world's sounds ring with the size of the room you are in
- Launcher: install, update, switch off and remove client plugins from a plugin list (checksum, permissions shown); the game reloads plugins when they change
- Join links: `belumi://join/host:port` opens the launcher (with a question first); `/joinlink` copies the link
- Combat log: swings, hits, damage dealt and taken and clicks per second for every fight, on a page (`/combat`)
- Photo mode on the free camera: orbit (G), time-lapse (H) and a stitched 360° panorama (J)
- Death clips: a death saves a clip by itself, named after the killer; K in the replay follows the killer with the camera
- Chest memory: `/chests TEXT` finds the containers you opened that hold it, with a waypoint and compass target on a click
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