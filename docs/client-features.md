# Belumi client features

Everything here is configured in **Options > Horizon Settings > Belumi Settings...** and stored in `belumi.toml` next to
`options.txt`. Every feature is off or harmless by default; you choose what to use. Fullbright and Mouse Tweaks (Horizon's
extras) are on the same page as quick toggles.

| Feature | Default key | What it does |
|---|---|---|
| HUD | | Corner overlay: FPS, coordinates, compass heading, ping, speed |
| Zoom | C (hold) | Narrows the field of view; the wheel adjusts it while held; smooth easing |
| Minimap | | Corner map with height shading, round or square, rotating or north up; dots for players (white), hostile mobs (red), neutral mobs (yellow) and animals (green), switchable under Maps |
| Graphics quality | Settings | Default, High, Medium, Low, Minimal, Shaders off: steps the active shader pack through its own `profile.*` entries (written to `shaderpacks/<pack>.zip.txt`, only if that file is empty or the game's own), and turns the shaders off at the last step |
| World map | M | Overlay of everything you explored, saved per world and server. Left button drags, arrows/WASD move, wheel zooms, C centres on you, B sets a waypoint at the crosshair, Enter teleports there (needs permission); the player stands still while it is open |
| Waypoints | B | Saves your position; `/waypoint add [name]`, `remove NAME`, `list`, `clear`; labels in the world, on the maps and a nearest list |
| Packet inspector | F8 | Live packet rates and sizes, top packets by bytes, the latest packets |
| Freecam | F9 | Detaches the camera and blocks all movement input of the player; WASD, Space, Shift, Ctrl for speed, the wheel changes speed |
| Screen effects | | Vignette, saturation, contrast, colour temperature, sharpen, film grain ([effects.md](effects.md)) |
| Client plugins | | WebAssembly plugins from `belumi-plugins/` ([client-plugins.md](client-plugins.md)) |
| Voice chat | V (push to talk), K (mute) | Simple Voice Chat compatible ([voice-chat.md](voice-chat.md)) |

## Builds and features

The plugin host and voice chat are cargo features of `horizon-client`, on by default since the launcher starts that build:

```sh
cd vendor/horizon
cargo build --profile fast -p horizon-client                                          # includes plugins and voice chat (needs libopus)
cargo build --profile fast -p horizon-client --no-default-features                    # without audio, controllers, plugins and voice
```

With the switch on but the feature missing, the game says so in chat once.

## Files

| File | Content |
|---|---|
| `belumi.toml` | All settings above |
| `belumi-waypoints.toml` | Waypoints, grouped by world or server and dimension |
| `belumi-maps/<world or server>/<dimension>/r.X.Z.bin` | The explored world map, one file per region of 32x32 chunks (changed regions are written in the background) |
| `belumi-plugins/<name>/` | Client plugins |

## Performance

Passive features do their work in the frame without extra threads, except the maps: two worker threads scan chunks and
paint the minimap's canvas, a third paints the world map's, and the game thread only schedules and draws them (the GPU
shifts and turns the finished canvas). The world map writes changed regions on a background thread every 30 seconds. The
screen effects add about 0.04 ms (measured, [effects.md](effects.md)).

## Known limits

- The world map shows the surface of the Overworld and the End; caves and the Nether are not stored yet.
- Freecam keeps the render distance of your body: it stays within that range.
- The in-world waypoint labels assume the default projection and ignore the small speed-based field of view changes.
- Voice chat connects to live Simple Voice Chat servers; audio has not been checked by ear ([voice-chat.md](voice-chat.md#status)).
- Shader packs read the player's choices from `shaderpacks/<pack>.zip.txt` (`NAME=value`; switches `true` or `false`). Solas runs without its floodfill and voxel lighting (`VX_SUPPORT` stays undefined until 3D images exist), so there is no coloured block light.
