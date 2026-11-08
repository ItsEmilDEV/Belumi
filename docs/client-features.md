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
| Distant chunks | Modules page | Like the Bobby mod: columns a server sent stay drawn after it forgot them, out to your render distance (raise it past the server's), and are cached in `belumi-chunks/` so the next visit shows them before the server sends anything. The server's own columns replace cached ones; cached ones are not updated while you are away. The cache is pruned to 1 GB, oldest first; deleting the folder clears it |
| Breadcrumb trail | Modules page, `/trail [on\|off\|clear]` | Small dots in the world behind you, one every 4 blocks (the last 200, per dimension), for finding the way back. The game also sets waypoints by itself: `Portal Overworld` / `Portal Nether` where you left a dimension, and `Spawn` |
| Changes | Modules page | On a server, when a column comes back that the cache held, the blocks are compared. After a few seconds chat says how many chunks changed since your last visit; Modules > Changes lists them (most blocks first) and a click sets a waypoint and points the compass at it. Edits you watched yourself are remembered (`belumi-chunks/.../X.Z.seen`) and do not count; fluids, fire, leaves, snow and torches are ignored, and a column needs 4 changed blocks to count |
| Chest memory | Modules page, `/chests TEXT` | On a server, every chest, barrel, shulker box, hopper and dispenser you open is remembered with what it held and when (`belumi-chests.json`, kept per world). `/chests diamond` lists the ones that hold something matching, nearest first; a click on a hit sets a waypoint and points the compass at it. A chest that was broken since (its block is air again where the chunk is loaded) is left out. Only what the server showed you is stored |
| Photo mode | with Freecam on: G, H, J | **G** circles the camera slowly around the block in the crosshair (or a point 10 blocks ahead); **H** takes a picture every 5 s into `screenshots/timelapse-N/`; **J** takes a 360° panorama (about 27 pictures stitched into a 4096 x 2048 equirectangular PNG in `screenshots/`, without HUD, hand and body, hold still for a few seconds). F1 hides the HUD as always. Freecam has to be switched on in Belumi Settings |
| Combat log | Modules page, `/combat` | Every fight (it ends 8 s after the last swing, hit or damage) is written down in `belumi-combat.json` (the last 50): swings, hits that landed (a creature that lost health after you swung at it; for players the server's damage event), damage dealt and taken, the peak and second-by-second clicks per second, and who you fought. A short summary shows when it ends; the page lists them with a small CPS graph |
| Join link | `/joinlink`, Discord page | Copies `belumi://join/ADDRESS` for the server you are on; a friend with Belumi opens the link and the launcher asks before it joins ([launcher.md](launcher.md)). Discord's own "Join" button is not built |
| Sound physics | Modules page (off by default) | The world's sounds (not music, not the interface) go through a reverb sized by the room you stand in: 14 rays from your head measure the walls every quarter second; the open air and a flat ground add nothing, a closet a little, a big cave a lot and long. The change is eased, so a door does not click. Sounds are not muffled by walls yet |
| World history | Modules page, `/rollback` (single player) | A world you host or play keeps older versions of every chunk it saves over: up to 6 per chunk, at least 5 minutes apart, 256 MB in all (`<dimension>/belumi-history/X.Z/UNIXTIME.z`, written on a thread of its own). `/rollback list` tells how far back it goes; `/rollback RADIUS MINUTES` asks for the chunks within RADIUS (0 to 16) of you to go back to how they were MINUTES ago. An open world holds its chunks in memory, so it is applied when the dimension loads next: save and quit, open the world again. The state you rolled back from stays in the history, so a rollback can be rolled back. Only blocks and the chunk's own data: mobs and dropped items stay as they are now. `BELUMI_NO_HISTORY=1` turns it off |
| Redstone workbench | Modules page | **Signal numbers** write the signal strength over redstone dust and weighted plates, daylight sensors and target blocks, and the delay of repeaters (`3t`), within 12 blocks, in the colour of the strength; **tick buttons** run `/tick` for you: freeze or unfreeze, step 1, 5 or 20 ticks, rate 1, 5, 10 or 20 per second (a world with commands on); the **Profiler** HUD module shows what the server tick spends its time on |
| Pings and shared waypoints | Z, `/sharewp`, `/acceptwp` | **Z** marks the block in your crosshair for everyone else with Belumi on the server for 20 seconds (Shift+Z: danger, Ctrl+Z: come here) with your name and the distance; **`/sharewp [NAME]`** offers one of your waypoints (the nearest without a name), **`/acceptwp`** keeps the one offered to you (as `Name: waypoint`). The messages travel as a `belumi:ping` payload that a Horizon server (a dedicated server, a world opened to LAN, single player) hands on to the other players with the sender's real name; vanilla, Paper and other servers drop it, so there it does nothing. Rate limit: one message every half second; the server refuses anything that is not a well-formed ping or waypoint |
| Blueprint | `/blueprint` (or `/bp`), Modules page | Shows a building plan in the world: `.schem` (Sponge 2 and 3) and `.litematic` files from `schematics/` in the game folder. `/blueprint list`, `load NAME`, `place` (the plan's corner goes next to the block in your crosshair) or `here`, `move X Y Z`, `layer N` or `layer all` (one level at a time), `rotate` (a quarter turn clockwise seen from above; facings, axes, sign rotation, fence sides turn with it), `mirror x` or `mirror z` (east/west or north/south, also swapping the left and right of stairs, doors and chests), `off`. **Cyan** outlines are blocks the plan has and the world does not, **red** ones are different blocks; within 28 blocks of you, the 900 nearest. The page counts built, missing and wrong blocks (plans up to 500,000 blocks) and lists the materials with what you carry. A block counts as built when its name and the properties the plan names (facing, half...) match. Not in this version: textured ghost blocks, block entities and entities in a plan, and several litematic regions are merged into one box |
| Accurate placement | Modules page, key (unbound by default) | The functions of the Fabric mod "Accurate Block Placement Reborn" (off by default; some servers count it as cheating, the game says so when you switch it on): with the right button held, blocks are placed as fast as the crosshair moves to a new spot instead of one every fourth tick, and in creative there is no pause between two blocks you break (the second switch, "Accurate creative break"). The vanilla wait stays for items that are not blocks, for blocks that do something when used (chests, shulker boxes, doors, ...) and for scaffolding while you hold scaffolding. Measured in a creative test: 15 against 22 blocks placed and 8 against 29 broken in 3 seconds |
| Light overlay | Modules page (off by default) | An X on the floor where a monster can spawn: **red** where there is no block light and no sky light (dark at any hour), **yellow** where there is no block light but the sky lights it (monsters spawn there at night); anything with block light shows nothing. Within 12 blocks, floors with two blocks of air above; the light is what the server sent, so it follows torches and lamps you place |
| Gamma boost | Modules page, keys (unbound by default) | Brightness beyond the options slider, for dark caves: 0 to 15 steps added on top of the slider, cycled 0, 4, 8, 12, 15 by the card or by a key; a second key switches fullbright. A dark cave measured 10 against 92 of 255 on average at level 15. Set the keys (`gamma_key`, `fullbright_key`) in `belumi-modules.toml`; some servers count the help as unfair |
| Sound controller | Modules page | Single sounds turned down or muted: the page lists the sound events played last, each with Mute, 25, 50, 100, 150 and 200 buttons; the levels are kept in `belumi-modules.toml` (`[misc.sound_levels]`, in percent). A muted sound is not played, shows no subtitle and is not handed on to guests of your world. Above 100 a sound is boosted after the game's own volume (a very loud mix is held in check by the limiter). Checked: a muted cow never reached the played list while the stone sound did |
| Ender chest tooltip | with Shulker preview on | The Ender Chest item's tooltip shows what your ender chest held when you last had it open (the grid of the shulker preview), kept per world in `belumi-chests.json`. The chest memory now follows what you move in and out while a container is open, not only what it held when it opened |
| Riptide fix | Modules page (off by default) | Depth Strider no longer slows a Riptide launch: the vanilla game has the bug MC-136249, where the boots add drag to the one push of the trident. Horizon copies the vanilla physics, so it had the bug too. Measured in water with Riptide III: 14.9 blocks without boots, 8.0 with Depth Strider III, 14.9 with the fix. It is a client-side change of your own movement: a server that checks speed may not like it |
| Switch notices | Modules page | What a switch of the newer modules did (accurate placement, gamma, fullbright, riptide fix) is said on the HUD, in chat, or not at all: the card cycles `hud`, `chat`, `off` (`confirm` in `belumi-modules.toml`; the Fabric mod calls it "Confirmation" and "Confirmation Type"). Switching on a feature a server may count as help adds a warning to the notice |
| Pat | Shift + right button | Pet any creature (and, by setting, players) within reach, like the mod PatPat: every press is one stroke and you can click as fast as you like (at most about 7 strokes a second per creature); holding the button strokes on at an even pace. Each stroke presses the creature in and lets it spring back (less for big ones, more for babies), your arm swings, a soft brushing rustle plays with changing pitch, hearts rise, and now and then the creature answers in its own voice; letting go fades it out quickly. With an empty hand (setting: any hand). Nothing is used meanwhile, so a pet does not sit or stand up under your hand; a click without Shift is vanilla. Other Belumi players on a Horizon server see your pats (the `belumi:ping` relay). Switches on the Modules page. **`belumi-patpat.json`** in the game folder sets the look per kind of creature, read again when it changes: `{"default": {"hearts": 2}, "minecraft:cat": {"strength": 0.6, "hearts": 6, "sound": "minecraft:entity.cat.purr", "volume": 0.4}, "minecraft:creeper": {"sound": "none"}}` (`strength` 0 to 2, `period` seconds of one stroke 0.2 to 4, default 0.5, `hearts` per second 0 to 10, `sound` `auto`, `none` or an event id, `volume` 0 to 1; what a kind leaves out comes from `default`). `/pat` lists what you patted most (`belumi-pats.json`) |
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
| Inspect | A few lines about the crosshair target, as the mod Jade does, as far as `inspect_range` reaches (20 blocks; the game's own reach is for hitting): a block's name, hardness, whether your tool in hand gets drops and how long breaking takes (survival), crop growth, signal strength, and the biggest stacks a chest you opened held; a creature's name and health (green, yellow, red); a player's name; and paintings, item frames, armor stands, boats and minecarts (within the game's reach) |
| Profiler | In single player: the server tick's time (green under 25 ms of its 50) and the six parts that take the most (`tick_blocks`, `tick_mobs`, ...), smoothed over about ten ticks. In a world on a server it says so; the server's own numbers are not sent |
| FPS | Frames per second, green from 60 |
| Ping | Your latency on a server, green up to 80 ms |
| Coordinates | Your block position |
| Compass | An arrow, the name and the distance of a waypoint: the nearest one, or the one you set with `/compass NAME`; `/compass back` points at where your breadcrumb trail began (the way back) |

Keystrokes light up with a short fade. Every module except armor and inventory has a size and a background setting in the
builder (0 % leaves only the text).

The **Modules** button leads to the modules without a place on the HUD: toggle sprint and sneak, freelook (hold Left Alt: the
camera looks around while the player keeps his direction), crosshair editor (presets, shape, colour with red, green and blue steps, size, thickness, gap, outline,
dot, spread while moving), item tooltips with ids and durability, break warning, chat tools, auto-reconnect, fullbright, mouse
tweaks, sort buttons, own name tag, the replays, and **Switch account**: it lists the accounts saved in the launcher (names only, never
tokens, from `belumi-accounts.json`); picking one saves the world, ends the game and the launcher signs that account in and starts
the game again on the same server. It needs the game to be started from the launcher and the launcher to stay open (not "close at start").

**Chat tools** (Modules > Chat tools): time stamps, merging of repeated lines (`(x3)`), words that hide a line, words that
colour a line, a bell when your name is written, and a history of up to 2000 lines. `/find TEXT` searches the history.

**Replays and clips**: with recording on, every session is saved to `replays/` (all server packets plus your own movement).
F10 saves a clip of the last 30 seconds (the length is set on the page), F6 a bookmark; a notification confirms both. The
page lists recordings and clips; Play opens one with a free camera. While it plays: Space pauses, `[` and `]` change the
speed (0.25x to 16x), the arrows jump 5 s (Shift: 30 s), Home restarts, V switches between the free camera and the
player's view, and Right Shift opens the controls with a clickable timeline. Jumping back restarts the session at that time.
**Video**: F7 (or the Video button on the controls) starts saving the picture 30 times a second of replay time into `replays/video-N/` (no HUD; close the controls first); at the end of the clip or on F7 again `ffmpeg`, when installed, makes `replay.mp4` of constant 30 fps from them, at the right speed even if the machine missed frames, and removes the pictures. Without ffmpeg the pictures stay with a `make-video.sh`. Plan for the disk: about 1 MB per picture, so a minute is about 2 GB until the video is made. `/replay` opens the list, `/replay play N [CLIP]` plays the Nth recording (newest first) or its CLIPth clip.

**Camera paths**: with the free camera on, P sets a camera point where the camera is at that moment of the replay (yellow
marks on the timeline; a point within a quarter second replaces the old one), O makes the camera follow a smooth curve through
the points as the replay plays (a second O gives the camera back), Backspace clears the points. Before the first point and after
the last the camera stands still. The points last until the replay is closed.

**Death clips**: with recording on, a death saves a clip of the last moments by itself (named `Death 0:31 by zombie`; switch it with
"Death clip" on the Replays page). The clip remembers who dealt the last blow (the server's damage event, else the nearest creature
within 8 blocks). While it plays with the free camera, **K** makes the camera circle the killer, behind the player, with both in the picture.

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
| `belumi-chunks/<server>/<dimension>/X.Z.bin` | The distant-chunk cache, one compressed column per file |
| `belumi-chests.json` | The remembered containers |
| `belumi-combat.json` | The last fights |
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
