# Discord Rich Presence

Belumi shows "Playing Belumi" on the player's Discord profile: menus, singleplayer, multiplayer or hosting,
the dimension and the time in the world. No server addresses, names or coordinates are sent.

## Switching it off

- In the game: Options, Extras, Discord Rich Presence.
- Environment: `BELUMI_NO_DISCORD=1` (or the older `HORIZON_NO_DISCORD=1`).

## The Discord application

The presence needs Belumi's own application on https://discord.com/developers/applications. Never use another
application's ID.

1. Create an application named **Belumi**.
2. Under Rich Presence, Art Assets, upload the logo with the key `belumi`.
3. The Application ID is set as `BELUMI_DISCORD_APP_ID` in
   `vendor/horizon/crates/horizon-client/src/discord_presence.rs` (or in the environment for a test).

While the ID is empty, the game starts no presence at all.

Optional images `survival`, `creative`, `adventure` and `spectator` show the game mode as a small icon; upload them
and set `MODE_IMAGES` to `true` in the same file.
