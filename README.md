<p align="center"><img src="docs/assets/belumi-logo.png" alt="Belumi" width="160"></p>

# Belumi

Belumi is a Minecraft: Java Edition client written in Rust, with its own launcher and a plugin-capable server side.
It is built on [Horizon](https://github.com/horizon-rust/horizon), an open-source Rust rewrite of the Minecraft
client and server (MIT).

Belumi is an independent project. It is not made, approved or supported by Mojang Studios or Microsoft. Minecraft is
a trademark of Mojang Studios. Belumi contains no Minecraft files: textures, sounds and data come from your own
Minecraft installation.

## Status

Early development. The client runs on Minecraft 26.3 natively and plays on 26.2 servers through a translation layer,
offline and with a Microsoft account. There are no releases yet. The launcher's own Microsoft sign-in waits for
Mojang's approval of its application; until then it can reuse an account from Prism Launcher.

## Parts

- `crates/belumi-launcher`: the launcher (egui): Microsoft sign-in, server list, game log, starting the client
- `crates/belumi-protocol-compat`: protocol version detection and compatibility helpers
- `crates/belumi-core`: configuration, logging and paths
- `crates/belumi-server`, `crates/belumi-plugin-api`, `crates/belumi-plugin-sdk`: WASM plugin framework for servers
- `vendor/horizon`: the Horizon fork with Belumi's client changes (git submodule)

## Building

```sh
git clone --recurse-submodules https://github.com/ItsEmilDEV/Belumi.git
cd Belumi
cargo test --workspace
cargo build --release -p belumi-launcher
cd vendor/horizon && cargo build --profile fast -p horizon-client
```

The game needs prepared Minecraft resources from your own installation; see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Documentation

- [Development](docs/DEVELOPMENT.md)
- [Launcher](docs/launcher.md)
- [Minecraft 26.2 vs 26.3 on the wire](docs/wire-differences-26.2.md)
- [Server plugins](docs/server-plugins.md)
- [Architecture](docs/ARCHITECTURE.md), [Roadmap](docs/ROADMAP.md), [Decisions](docs/DECISIONS.md)

## License

MIT. Horizon is Copyright (c) 2026 Ludovic Marie and the Horizon contributors, also MIT; its notice is kept in
[LICENSE](LICENSE) and in the submodule.
