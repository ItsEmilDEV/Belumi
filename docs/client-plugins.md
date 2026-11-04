# Belumi Client Plugins

Client plugins are WebAssembly modules the game runs in a sandbox. They add HUD lines, chat output and
commands, and react to ticks, keys, chat and joins. They never touch game state directly: everything goes
through a small host API with permissions.

## Turn them on

1. Build the client with the cargo feature `belumi-plugins`:
   `cargo build --profile fast -p horizon-client --features belumi-plugins` (in `vendor/horizon`).
2. Options > Horizon Settings > Belumi Settings > Plugins > **Client plugins: On** (`plugins = true` in `belumi.toml`).
3. Put each plugin in `belumi-plugins/<name>/` next to `options.txt`, with `plugin.toml` and `plugin.wasm`.

`/plugins` lists the loaded plugins. A build without the feature says so in chat when the switch is on.

## Manifest (`plugin.toml`)

```toml
name = "hello-client"          # 1-32 characters of a-z 0-9 - _
version = "0.1.0"
api_version = 1                # must equal the client's ABI version
description = "..."
permissions = ["chat", "hud"]  # what the plugin may do
subscribe = ["tick", "key"]    # events it receives: tick, key, chat, join, leave
commands = ["hello"]           # /hello [args] runs the plugin, never reaches the server
```

## ABI v1

Events cross the boundary as JSON.

| Export | Signature | Purpose |
|---|---|---|
| `belumi_alloc` | `(len: i32) -> i32` | The host writes the event JSON into plugin memory |
| `belumi_on_event` | `(ptr: i32, len: i32) -> i32` | 0 continues, 1 cancels (only `command` events can be cancelled) |
| `belumi_init` / `belumi_shutdown` | `() -> ()` | optional |

Host imports (module `belumi`):

| Import | Returns | Permission |
|---|---|---|
| `log(level, ptr, len)` | none | none |
| `chat_print(ptr, len)` | 0 ok, 1 denied, 2 queue full, -1 invalid memory | `chat` |
| `hud_set(id_ptr, id_len, text_ptr, text_len)` | 0 ok, 1 denied, 2 line limit, -1 invalid memory | `hud` |
| `player(ptr, cap)` | JSON length, larger than `cap` means retry with a bigger buffer, -1 error | none |

`hud_set` with empty text removes the line. Limits: 16 HUD lines of 120 characters per plugin, 64 queued chat lines,
256 characters per chat line.

`player` returns `{"x","y","z","yaw","pitch","dimension","fps"}`, refreshed every tick.

Events: `tick {tick}`, `key {key, pressed}` (vanilla key names such as `key.keyboard.g`), `chat {text}` (every line the
chat receives), `command {name, args}` (only to the owner of the command), `join {address}`, `leave`.

## Safety

Each call gets a fuel budget (20 million instructions) and each instance 32 MiB of memory. A trap, an endless loop or an
invalid memory access disables only that plugin; its HUD lines disappear and the game reports the error in chat.

## Writing a plugin in Rust

```rust
use belumi_client_plugin_sdk::{Event, EventResult, Plugin, export_plugin, host};

#[derive(Default)]
struct Mine;

impl Plugin for Mine {
    fn on_event(&mut self, event: &Event) -> EventResult {
        if let Event::Tick { tick } = event && tick % 20 == 0 {
            let _ = host::set_hud("clock", &format!("{} s", tick / 20));
        }
        EventResult::Continue
    }
}
export_plugin!(Mine);
```

`crate-type = ["cdylib"]`, then `cargo build --release --target wasm32-unknown-unknown` and copy the `.wasm` file to
`belumi-plugins/<name>/plugin.wasm`. `examples/hello-client-plugin` is a complete example.

## Where the code lives

| Part | Crate |
|---|---|
| Manifest, events, ABI constants | `crates/belumi-client-plugin-api` |
| Host (wasmtime) | `crates/belumi-client-plugins` |
| Guest SDK | `crates/belumi-client-plugin-sdk` |
| Game glue (one-line hooks) | `vendor/horizon/crates/horizon-client/src/game/belumi_plugins.rs` |

The server's plugins are separate; see [server-plugins.md](server-plugins.md).
