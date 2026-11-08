<p align="center"><img src="docs/assets/belumi-logo.png" alt="Belumi" width="160"></p>

# Belumi

Belumi is a Minecraft: Java Edition client written in Rust, with its own launcher and a plugin-capable server side.
It is built on [Horizon](https://github.com/horizon-rust/horizon), an open-source Rust rewrite of the Minecraft
client and server (MIT).

Belumi is an independent project. It is not made, approved or supported by Mojang Studios or Microsoft. Minecraft is
a trademark of Mojang Studios. Belumi contains no Minecraft files: textures, sounds and data come from your own
Minecraft installation.

## What you get

- A native client for Minecraft 26.3 that also plays on 26.2 servers through a translation layer
- A launcher with Microsoft sign-in, a server list, one-click updates and Discord Rich Presence
- A client screen on **Right Shift**: a HUD builder (keystrokes, CPS, armor, effects with icons, FPS, ping, coordinates, clock,
  inventory), a crosshair editor, chat tools, toggle sprint and sneak, freelook, shulker previews in tooltips
- Voice chat that works with Simple Voice Chat servers, in every build
- Replays with free camera, speed and seek, camera paths and clips
- Discord Rich Presence that shows what you choose (server, world, time, game mode)
- WebAssembly plugins for the client and for servers

## Status

Early development. The launcher's own Microsoft sign-in is approved by Mojang;
it can also reuse an account from Prism Launcher, and offline play works.

## Install

Download the archive or installer of the newest release from the
[releases page](https://github.com/ItsEmilDEV/Belumi/releases):

| System | File | How |
|---|---|---|
| Any Linux (x86_64) | `belumi-<version>-linux-x86_64.tar.gz` | unpack, run `./install.sh` (user install) or `./install.sh --prefix /usr/local` |
| Any Linux (x86_64) | `belumi-<version>-x86_64.AppImage` | `chmod +x` it and run it; libopus is inside |
| Debian, Ubuntu, Mint | `belumi_<version>_amd64.deb` | `sudo apt install ./belumi_<version>_amd64.deb` |
| Fedora, openSUSE | `belumi-<version>-1.x86_64.rpm` | `sudo dnf install ./belumi-<version>-1.x86_64.rpm` |
| Arch, CachyOS | `packaging/PKGBUILD` | `makepkg -si` in that folder |
| Windows 10 and 11 (x64) | `belumi-<version>-windows-x64-setup.exe` | run the installer; the `.zip` is the portable version |

The game needs Minecraft's files (they are Mojang's and never ship with
Belumi): after you sign in, the launcher's **Download and prepare** button fetches them from Mojang's own servers (about 1 GB)
and prepares them; see [docs/launcher.md](docs/launcher.md). Once installed, the launcher offers new versions itself
(an **Update now** button, checked by SHA-256; for the AppImage and the packages use the new file or your package manager).

## Parts

- `crates/belumi-launcher`: the launcher (egui): Microsoft sign-in, server list, game log, starting the client
- `crates/belumi-protocol-compat`: protocol version detection and compatibility helpers
- `crates/belumi-core`: configuration, logging and paths
- `crates/belumi-server`, `crates/belumi-plugin-api`, `crates/belumi-plugin-sdk`: WASM plugin framework for servers
- `crates/belumi-client-plugins`, `belumi-client-plugin-api`, `belumi-client-plugin-sdk`: WASM plugins for the client
- `crates/belumi-voice`: Simple Voice Chat compatible voice chat
- `vendor/horizon`: the Horizon fork with Belumi's client changes (git submodule)

## Building

```sh
git clone --recurse-submodules https://github.com/ItsEmilDEV/Belumi.git
cd Belumi
cargo test --workspace
cargo build --release -p belumi-launcher
cd vendor/horizon && cargo build --profile fast -p horizon-client --features belumi
```

The game needs prepared Minecraft resources: the launcher downloads and prepares them (see [docs/launcher.md](docs/launcher.md)); by hand, see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Documentation

- [Development](docs/DEVELOPMENT.md)
- [Launcher](docs/launcher.md), [Discord Rich Presence](docs/discord-presence.md)
- [Minecraft 26.2 vs 26.3 on the wire](docs/wire-differences-26.2.md)
- [Client features](docs/client-features.md): HUD, zoom, maps, waypoints, freecam, inspector, effects
- [Client plugins](docs/client-plugins.md), [Voice chat](docs/voice-chat.md), [Screen effects](docs/effects.md)
- [Server plugins](docs/server-plugins.md)
- [Releases and updates](docs/releases.md)
- [Architecture](docs/ARCHITECTURE.md), [Roadmap](docs/ROADMAP.md), [Decisions](docs/DECISIONS.md)

## Credits

- The default look is the shader pack **Solas** by Septonious. It is not part of Belumi: the launcher offers to download it
  from [Modrinth](https://modrinth.com/shader/solas-shader) (all rights reserved by its author).
- Belumi is built on [Horizon](https://github.com/horizon-rust/horizon).

## License

MIT. Horizon is Copyright (c) 2026 Ludovic Marie and the Horizon contributors, also MIT; its notice is kept in
[LICENSE](LICENSE) and in the submodule.
