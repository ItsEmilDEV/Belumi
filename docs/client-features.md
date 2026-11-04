# Belumi client features

Everything here is configured in **Options > Horizon Settings > Belumi Settings...** and stored in `belumi.toml` next to
`options.txt`. Every feature is off or harmless by default; you choose what to use. Fullbright and Mouse Tweaks (Horizon's
extras) are on the same page as quick toggles.

| Feature | Default key | What it does |
|---|---|---|
| HUD | | Corner overlay: FPS, coordinates, compass heading, ping, speed |
| Zoom | C (hold) | Narrows the field of view; the wheel adjusts it while held; smooth easing |
| Minimap | | Corner map with height shading, round or square, rotating or north up |
| World map | M | Full-screen overlay of everything you explored, saved per world and server, wheel zooms |
| Waypoints | B | Saves your position; `/waypoint add [name]`, `remove NAME`, `list`, `clear`; labels in the world, on the maps and a nearest list |
| Packet inspector | F8 | Live packet rates and sizes, top packets by bytes, the latest packets |
| Freecam | F9 | Detaches the camera, you stay put; WASD, Space, Shift, Ctrl for speed, the wheel changes speed |
| Screen effects | | Vignette, saturation, contrast, colour temperature, sharpen, film grain ([effects.md](effects.md)) |
| Client plugins | | WebAssembly plugins from `belumi-plugins/` ([client-plugins.md](client-plugins.md)) |
| Voice chat | V (push to talk), K (mute) | Simple Voice Chat compatible ([voice-chat.md](voice-chat.md)) |

## Builds and features

The plugin host and voice chat are cargo features of `horizon-client` so the default build stays small:

```sh
cd vendor/horizon
cargo build --profile fast -p horizon-client                                          # core features
cargo build --profile fast -p horizon-client --features belumi-plugins,belumi-voice   # plugins and voice chat (needs libopus)
```

With the switch on but the feature missing, the game says so in chat once.

## Files

| File | Content |
|---|---|
| `belumi.toml` | All settings above |
| `belumi-waypoints.toml` | Waypoints, grouped by world or server and dimension |
| `belumi-maps/<world or server>/<dimension>.bin` | The explored world map (compressed tiles) |
| `belumi-plugins/<name>/` | Client plugins |

## Performance

Passive features do their work in the frame without extra threads: the HUD builds a few strings, the world map scans at
most 0.15 ms of chunk columns per frame (nearest first) and saves on a background thread every 30 seconds, the screen
effects add about 0.04 ms (measured, [effects.md](effects.md)). The world map overlay recomposes its 256x256 image only
when you moved half a map pixel or explored something new, and at most every third frame.

## Known limits

- The world map shows the surface; caves and the Nether are not mapped.
- Freecam keeps the render distance of your body: it stays within that range.
- The in-world waypoint labels assume the default projection and ignore the small speed-based field of view changes.
- Voice chat has not been tried against a live Simple Voice Chat server ([voice-chat.md](voice-chat.md#status)).
