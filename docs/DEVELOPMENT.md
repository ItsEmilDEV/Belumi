# Developing Belumi

Placeholders used below: `$JDK25` (a Java 25 JDK), `$MC_DIR` (a launcher-shaped Minecraft 26.3 directory) and `$SERVER_DIR` (a scratch directory for the local test server).

## Repository layout and workspaces

- Root Cargo workspace: `crates/belumi-core`, `crates/belumi-protocol-compat`, `crates/belumi-launcher`.
- Horizon is the git submodule at `vendor/horizon` and is a **separate Cargo workspace**.
- The actual playable binary is `vendor/horizon/target/<profile>/horizon-client`.
- Root `cargo test --workspace` also compiles the path-dependent Horizon crates.

The submodule tracks the Belumi branch of the Horizon fork; the translation work lives on top of upstream Horizon.

## Product focus

Belumi is a native Rust Minecraft client built on Minecraft Java Edition 26.3.
P0 (reached 2026-11-03): play on Minecraft 26.2 servers (protocol 776).

- Native version constants: `vendor/horizon/crates/horizon-core/src/version.rs`
  (`VANILLA_VERSION = "26.3"`, `PROTOCOL_VERSION = 777`, `DATA_VERSION = 5023`).
- Compatibility changes in the Horizon submodule: the handshake override and protocol
  selection in `crates/horizon-client/src/net.rs`, plus the translation layers listed under
  "Current verified protocol state".
- The raw launcher (`crates/belumi-launcher`) exists; plugin API, cosmetics and UI redesign
  are not part of the first release.

## Linux / CachyOS prerequisites

Verified on CachyOS with Rust 1.99.1 (edition 2024, MSRV 1.85).

Runtime/build packages used by `horizon-client`:

- `alsa-lib` (audio; `--no-default-features` drops audio/controllers)
- `systemd-libs` (provides libudev)
- `vulkan-icd-loader` plus the vendor Vulkan driver (`mesa`/RADV for AMD)
- a working Wayland/X11 session (`DISPLAY` and/or `WAYLAND_DISPLAY`)
- `python3` for repository tooling
- **Java 25 JDK** for `horizon-prepare` (Minecraft 26.3 pins Java 25; Java 17/21 cannot
  compile the exporters). This is not needed just to run the already-built client.

The Windows-only stack overflow reported earlier has not reproduced on Linux.

## Build

```sh
cd vendor/horizon

# Normal development build (verified)
cargo build -p horizon-client

# Optimized, quick-to-link build for playing
cargo build --profile fast -p horizon-client
```

The binaries are `vendor/horizon/target/debug/horizon-client` and
`vendor/horizon/target/fast/horizon-client`.

Root workspace checks:

```sh
cd <repository>
cargo test --workspace
```

Horizon client net tests:

```sh
cd vendor/horizon
cargo test -p horizon-client --lib net::
```

## Prepared resources

`horizon-client` needs prepared 26.3 resources. On this machine they already exist at:

```text
vendor/horizon/.local/horizon-client/26.3
vendor/horizon/.local/minecraft/26.3
```

They are gitignored and must never be committed (Mojang assets/data).

If they are missing, build `horizon-prepare` and run it with a Java 25 JDK and a local
Minecraft 26.3 installation. A verified example (paths are local to this machine; adapt
as needed):

```sh
cd vendor/horizon
cargo build --profile fast -p horizon-prepare

# A launcher-shaped view of the existing 26.3 client files.
# It needs Linux natives for jtracy, netty-epoll and LWJGL 3.4.3; Windows-only
# Prism libraries are not enough. Fetch missing natives from libraries.minecraft.net.
JAVA_HOME=$JDK25 \
HORIZON_MINECRAFT_DIR=$MC_DIR \
./target/fast/horizon-prepare --prepare \
  --server-jar /path/to/mojang_26.3.jar
```

The official client jar SHA1 must be `e877b6a07acd633fb3bb475002175cec036e7b87`; the
official 26.3 server jar SHA1 is `33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c`.

## Launch commands

Headless-safe startup smoke test (loads resources, creates a window, renders two frames):

```sh
cd vendor/horizon
./target/debug/horizon-client \
  --resources "$PWD/.local/horizon-client/26.3" \
  --frames 2 --test-terrain --name TestPlayer
```

Connect to a 26.2 server with the protocol override:

```sh
cd vendor/horizon
BELUMI_TARGET_PROTOCOL=776 \
./target/debug/horizon-client \
  --resources "$PWD/.local/horizon-client/26.3" \
  --server 127.0.0.1:25565 --name TestPlayer
```

Useful environment variables: `HORIZON_NO_AUDIO=1` disables audio, `RUST_BACKTRACE=1`
prints backtraces. Sounds are optional and absent unless `horizon-prepare sounds` is run.

## Local Minecraft 26.2 test server

A real 26.2 server is enough to reproduce protocol failures. A known local copy of the
official jar (SHA1 `823e2250d24b3ddac457a60c92a6a941943fcd6a`) exists at:

```text
/path/to/mojang_26.2.jar
```

Minimal offline server setup:

```sh
mkdir -p $SERVER_DIR && cd $SERVER_DIR
cp /path/to/mojang_26.2.jar server.jar
printf 'eula=true\n' > eula.txt
cat > server.properties <<'EOF'
online-mode=false
enforce-secure-profile=false
server-port=25565
server-ip=127.0.0.1
level-type=minecraft\:flat
generate-structures=false
view-distance=4
simulation-distance=4
spawn-protection=0
EOF
setsid -f $JDK25/bin/java -Xms1G -Xmx2G -jar server.jar nogui \
  < /dev/null > server.log 2>&1
```

## Current verified protocol state (2026-11-03)

The client plays on Minecraft 26.2 servers (protocol 776), offline-mode and online-mode
(verified by the user on a real server with a Prism-issued session). `--protocol auto`
(default) pings the server and picks 776 or 777; `--protocol 776|native` forces it.
`BELUMI_TARGET_PROTOCOL` still works.

Where the translation lives (all in `vendor/horizon`, currently uncommitted work):

- `crates/horizon-client/src/net/protocol_compat.rs`: per-state packet-id tables,
  payload rewrites (Login, Respawn, tags, chunks, block updates, explode, level particles,
  advancements, recipe book, entity moves), the serverbound `PlayerAction` remap and the
  `Animate` -> `SwingAnimation` retarget.
- `crates/horizon-protocol/src/dialect.rs` + generated `dialect_26_2.rs`: process-wide
  26.2 switch (set in `net::run`) for item, data-component, particle, sound and block-state
  ids. Regenerate with `tools/gen_26_2_ids.py REPORTS_26_2 REPORTS_26_3 OUT.rs`, where the
  reports come from `java -DbundlerMainClass=net.minecraft.data.Main -jar server.jar --reports`.
- `item/skip.rs`: `component_26_2` holds 26.2 component layouts that differ (pot_decorations).
- `BELUMI_DUMP_BAD_PACKETS=DIR` writes undecodable Play packets (original bytes) to DIR.

How differences were found: `javap -p -c` of both server jars (`versions/26.x/server-26.x.jar`
inside the bundler) and `--reports` (`registries.json`, `blocks.json`, `packets.json`).

`tools/diff_26_2_codecs.sh JAR_26_2 JAR_26_3 OUT_DIR` compares the network-relevant classes of both
server jars (about 5 minutes) and lists what changed: `SHAPE` entries (fields or signatures differ)
are the ones to look at, `CODE` entries are mostly refactors. It found, among others, the
`PlayerAction` enum shift, the `Animate` renumbering and the `LevelParticles` layout.

Differential testing: run the local 26.2 server with a console FIFO
(`tail -f console.fifo | java -jar server.jar nogui`), start a client with
`--control-socket` and `BELUMI_DUMP_BAD_PACKETS=DIR`, send `/give`, `/summon`, `/particle`,
`/setblock` ... through `horizon-ctl command`, and every packet that does not decode lands in DIR.
Checked this way (no undecodable packets): items with bundle/container/crossbow/firework/profile/
spawn-egg/potion/banner/trim/horn/attribute/map/book components, block and item displays, endermen,
falling blocks, villager trades, all particle kinds with data, teams, boss bars, titles, sounds,
signs, spawners, TNT, stats screen.

Verified unchanged between 26.2 and 26.3 (no translation needed): chat packets (only `Optional`
refactors), entity data serializer ids (26.3 appends `DYE_COLOR`), heightmap types and the wire
enums we compared (pose, equipment slot, game type, difficulty, boss bar, click type, ...).

Also exercised without errors: `SetPlayerTeam`, light updates (glowstone fill/clear), `RemoveEntities`,
boats/minecarts/riding, sign editor. Found by that: serverbound `SignUpdate` has a different field order
in 26.2 (position, side boolean, four lines) and made the server drop the connection; fixed in
`reorder_26_2_sign_update`. `OpenSignEditor` is wire-compatible (boolean byte == VarInt 0/1).

Serverbound packets: the codec diff lists every changed one. Real wire changes were AcceptTeleportation,
Attack/Punch, PlayerAction and SignUpdate (all translated). MoveVehicle, UseItem, UseItemOn, CommandSuggestion
and the structure block's Rotation were compared with `javap` and only refactored (records, StreamCodec
wrappers), so book editing, command/structure blocks, anvil, beacon and trades need no translation.

Known gaps: component layouts other than pot_decorations are untested in rarer cases; behaviour that
only shows on real servers (plugins, custom payloads, resource-pack prompts, anti-cheat). Watch for
`skipped undecodable packet` in the client output and dump it with `BELUMI_DUMP_BAD_PACKETS`.

## Server plugins

`crates/belumi-plugin-api` (manifest, events, ABI), `crates/belumi-server` (wasmtime host),
`crates/belumi-plugin-sdk` (Rust SDK) and `examples/hello-plugin`. Design and ABI: `docs/server-plugins.md`.
The Horizon side is `vendor/horizon/crates/horizon-client/src/belumi_hooks.rs`, behind the `belumi` feature
(a path dependency on the crates above, so `vendor/horizon` builds only inside this repository).

```sh
cd vendor/horizon && cargo build --profile fast -p horizon-client --features belumi
cargo build -p belumi-server && tools/build-example-plugin.sh <server-dir>/plugins   # needs: rustup target add wasm32-unknown-unknown
target/debug/belumi-server --dir <server-dir> --client vendor/horizon/target/fast/horizon-client \
  --resources vendor/horizon/.local/horizon-client/26.3      # settings in <server-dir>/belumi.toml; stop with `stop` on stdin
```

SIGTERM skips the game's stop path. `BELUMI_PROFILE=SECONDS` logs a tick profile; `BELUMI_SPAWN_INTERVAL=N` thins natural spawning.

## Launcher

`crates/belumi-launcher` (egui): Microsoft device-code sign-in and starting the client via
`--session-stdin`. See `docs/launcher.md`. The Azure client ID is `BUILT_IN_CLIENT_ID` in
`crates/belumi-launcher/src/settings.rs`; Mojang has approved it (https://aka.ms/mce-reviewappid), so the last sign-in
step works. A fork needs its own client ID and its own approval. Never use another app's client ID.

## Open work

1. Play longer on real 26.2 servers and fix what `skipped undecodable packet` reports.
2. Keep an eye on the launcher's own sign-in on real accounts (Prism stays as a fallback).

## References

- `docs/wire-differences-26.2.md` - how 26.2 differs from 26.3 on the wire and how it is translated.
- `docs/launcher.md` - launcher behaviour and dev scripts.
- `docs/server-plugins.md` - server plugin framework.
- `docs/UPSTREAM.md` - Horizon architecture and integration notes.

## Windows

The launcher and the client are written to run on Windows 10 and 11 (x64). Build both with the MSVC toolchain
(Visual Studio Build Tools, "Desktop development with C++") and Rust stable:

```powershell
cargo build --release -p belumi-launcher
cd vendor\horizon; cargo build --release -p horizon-client --features belumi
```

`packaging\build-release.ps1` builds both, a portable zip and, with Inno Setup 6 installed, the installer
(`packaging\belumi.iss`). CI builds and tests the launcher on `windows-latest`; the release workflow attaches the zip
and the installer to the GitHub release. The files are not code-signed yet, so Windows SmartScreen asks once.

What differs from Linux: the Microsoft sign-in is kept in the Windows Credential Manager (a token it refuses is kept
in an owner-only file under `%APPDATA%\Belumi`), the browser opens through `rundll32`, the game folder defaults to
the launcher's working directory, and the Vulkan check is skipped (the game falls back to DirectX 12). Voice chat
(`belumi-voice`) builds the Opus library from source with CMake, which GitHub's Windows runners and a Visual Studio
install provide. Voice chat is part of every Belumi build, so CMake is required.
