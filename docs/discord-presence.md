# Discord Rich Presence

Belumi shows "Playing Belumi" on the player's Discord profile, both in the launcher and in the game. The connection to the
Discord app is the crate `crates/belumi-presence` (Discord's local IPC socket); the game and the launcher both use it.

## What is shown

In the launcher: "In the Belumi launcher", the time since it opened and, if you allow it, the signed-in account name. While the
game runs the launcher shows nothing and the game speaks for itself.

In the game: the menus, singleplayer, multiplayer or an open world, plus what you choose under Right Shift, Modules,
**Discord presence** (saved in `belumi-modules.toml`, `[discord]`):

| Option | Default | Shows |
|---|---|---|
| Server address | off | "Playing on play.example.net" while on a server |
| World name | off | "Singleplayer: My Base" |
| Dimension | on | In the Overworld, the Nether or the End |
| Game mode icon | on | the small survival, creative, adventure or spectator icon |
| Time played | on | how long you have been in the world |
| Player count | on | the players on the server, or the party bar of your open world |

Nothing else is sent: no name, no coordinates, no chat.

## Switching it off

- In the game: the same screen (Rich Presence: OFF), or Options, Extras, Discord Rich Presence.
- In the launcher: Settings, Discord. The setting also switches the game off, because the launcher starts it with
  `BELUMI_NO_DISCORD=1`.
- For one run: `BELUMI_NO_DISCORD=1` (or the older `HORIZON_NO_DISCORD=1`).

## The Discord application

Belumi's own application ID is `APP_ID` in `crates/belumi-presence/src/lib.rs`; `BELUMI_DISCORD_APP_ID` in the environment
overrides it. Never use another application's ID. The Rich Presence art assets of that application:

| Key | Used for |
|---|---|
| `belumi` | the large image in the launcher |
| `belumi-minecraft` | the large image in the game |
| `survival`, `creative`, `adventure`, `spectator` | the small game mode icon |

Discord has to run on the same computer, with "Share your activity" on in its settings.
