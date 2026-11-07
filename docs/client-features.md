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
| Map overlays | Settings, `/map` | Chunk borders and slime chunks (tinted green) on both maps. Slime chunks need the world seed: singleplayer knows it, on a server type `/map seed NUMBER` (kept per server). `/map grid`, `/map slime` toggle, `/map export` writes a PNG of everything explored to `map-exports/` |
| Death waypoint | automatic | A waypoint "Death" is set where you died (replacing the last one) when waypoints are on |
| Interface style | Settings | Minecraft (translucent black boxes) or Modern (flat, soft corners, thin edge) for the HUD panels, the voice panel and the minimap rim |
| Player-list view | Tab (held) | While the player list is held, both maps show players as their face with the name below and mobs as their spawn egg (with the name they were given, if any) instead of dots |
| World map | M | Overlay of everything you explored, saved per world and server. Left button drags, arrows/WASD move, wheel zooms, C centres on you, PageUp/PageDown change the layer (Home: surface, End: follow you), B sets a waypoint at the crosshair, Enter teleports there (needs permission); the player stands still while it is open |
| Waypoints | B | Saves your position; `/waypoint add [name]`, `remove NAME`, `list`, `clear`, `import FOLDER_OR_FILE` (Xaero waypoint files, dimension from the `dim%N` folder, duplicates skipped); labels in the world, on the maps and a nearest list |
| Packet inspector | F8 | Live packet rates and sizes, top packets by bytes, the latest packets |
| Freecam | F9 | Detaches the camera and blocks all movement input of the player; WASD, Space, Shift, Ctrl for speed, the wheel changes speed |
| Screen effects | | Vignette, saturation, contrast, colour temperature, sharpen, film grain ([effects.md](effects.md)) |
| Client plugins | | WebAssembly plugins from `belumi-plugins/` ([client-plugins.md](client-plugins.md)) |
| Voice chat | V (push to talk), K (mute) | Simple Voice Chat compatible ([voice-chat.md](voice-chat.md)) |

## Client screen (Right Shift)

Right Shift (changeable in `belumi-modules.toml`, `menu_key`) opens the client screen. It starts in the **HUD builder**: every HUD
module can be dragged where you want it, with magnets to the screen's edges, centre lines and the other modules (hold Shift to
switch them off), the wheel resizes the chosen module, the arrows nudge it, Ctrl+Z takes the last change back. The bar at the bottom switches modules on and off;
**Starter** turns on a first set. The layout is saved in `belumi-modules.toml` and survives other window sizes.

| HUD module | What it shows |
|---|---|
| Keystrokes | WASD, jump and the mouse buttons as you press them, with the clicks per second on the buttons (keys follow your bindings) |
| CPS | Clicks per second of both mouse buttons, green from 12, yellow from 7 |
| Armor and items | Armor, main hand and off hand with the durability left, coloured and flashing when it is nearly gone (break warning) |
| Effects | Active effects with their icon, level and time left, shortest first; yellow under 30 s, blinking red under 10 s; icons and beacon effects can be switched off |
| Clock | The time, with or without seconds |
| Recording | A blinking REC light with the time of the running recording |
| Inventory | Your 27 storage slots as a small grid |
| FPS | Frames per second, green from 60 |
| Ping | Your latency on a server, green up to 80 ms |
| Coordinates | Your block position |

Keystrokes light up with a short fade. Every module except armor and inventory has a size and a background setting in the
builder (0 % leaves only the text).

The **Modules** button leads to the modules without a place on the HUD: toggle sprint and sneak, freelook (hold Left Alt: the
camera looks around while the player keeps his direction), crosshair editor (presets, shape, colour with red, green and blue steps, size, thickness, gap, outline,
dot, spread while moving), item tooltips with ids and durability, break warning, chat tools, auto-reconnect, fullbright, mouse
tweaks, sort buttons, own name tag, and the replays.

**Chat tools** (Modules > Chat tools): time stamps, merging of repeated lines (`(x3)`), words that hide a line, words that
colour a line, a bell when your name is written, and a history of up to 2000 lines. `/find TEXT` searches the history.

**Replays and clips**: with recording on, every session is saved to `replays/` (all server packets plus your own movement).
F10 saves a clip of the last 30 seconds (the length is set on the page), F6 a bookmark; a notification confirms both. The
page lists recordings and clips; Play opens one with a free camera. While it plays: Space pauses, `[` and `]` change the
speed (0.25x to 16x), the arrows jump 5 s (Shift: 30 s), Home restarts, V switches between the free camera and the
player's view, and Right Shift opens the controls with a clickable timeline. Jumping back restarts the session at that time.
**Camera paths**: with the free camera on, P sets a camera point where the camera is at that moment of the replay (yellow
marks on the timeline; a point within a quarter second replaces the old one), O makes the camera follow a smooth curve through
the points as the replay plays (a second O gives the camera back), Backspace clears the points. Before the first point and after
the last the camera stands still. The points last until the replay is closed.

The oldest recordings are deleted past the limit (`keep`, 20).

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
| `belumi-maps/<world or server>/<dimension>/cave-N/r.X.Z.bin` | The same for caves and the Nether: one map per 16-block height band `N` (`y >> 4`) of where you were |
| `belumi-modules.toml` | The HUD layout and the settings of the modules (crosshair, chat tools, freelook, replays) |
| `replays/*.brpl`, `*.clips.json` | Recorded sessions and their clips |
| `belumi-plugins/<name>/` | Client plugins |

## Performance

Passive features do their work in the frame without extra threads, except the maps: two worker threads scan chunks and
paint the minimap's canvas, a third paints the world map's, and the game thread only schedules and draws them (the GPU
shifts and turns the finished canvas). The world map writes changed regions on a background thread every 30 seconds. The
screen effects add about 0.04 ms (measured, [effects.md](effects.md)).

## Known limits

- Underground (a cave, or the Nether) the world map shows the cave floor at your height band and stores it per band; PageUp/PageDown look at the bands above and below. Up to 8 bands stay open at once.
- Freecam keeps the render distance of your body: it stays within that range.
- The in-world waypoint labels assume the default projection and ignore the small speed-based field of view changes.
- Voice chat connects to live Simple Voice Chat servers; audio has not been checked by ear ([voice-chat.md](voice-chat.md#status)).
- Shader packs read the player's choices from `shaderpacks/<pack>.zip.txt` (`NAME=value`; switches `true` or `false`). Solas runs without its floodfill and voxel lighting (`VX_SUPPORT` stays undefined until 3D images exist), so there is no coloured block light.
