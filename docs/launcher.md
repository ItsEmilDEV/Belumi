# Belumi launcher (raw GUI)

`crates/belumi-launcher`: Microsoft sign-in and starting `horizon-client`. No design yet.

```sh
cargo run --release -p belumi-launcher   # or: cargo run -p belumi-launcher
```

## What it does

- Device-code sign-in: Microsoft -> Xbox Live -> XSTS -> Minecraft, then the ownership
  check and the profile (the flow in `vendor/horizon/docs/microsoft-account.md`).
- Starts the game through Horizon's Launch API: `--session-stdin` hands over the account
  as JSON on standard input, so the access token never reaches the disk or `argv`.
- "Play offline" starts with `--name` for offline-mode servers (e.g. the local 26.2 test
  server). `--protocol auto` picks 26.2 or 26.3 from the server's status ping.
- `launcher.json` (in `$BELUMI_CONFIG_HOME` or the platform config dir, mode 0600) keeps the
  settings and the Microsoft *refresh* token. There is no keychain integration yet.
  The game's output goes to `<data dir>/logs/game.log`.

## Playing

- **Play** is the centre: the account, the big Play button, then your servers (with a live ping: online or offline,
  players, latency) and your singleplayer worlds with a one-click start. "Main menu" starts the game without joining.
- Join links: `belumi://join/host:port` starts the launcher (the .desktop file and the Windows installer register the scheme), which asks "Join / Join offline / Cancel" before it starts the game on that server. In the game, Right Shift > Modules > Discord presence > "Copy join link" or `/joinlink` copies the link for the server you are on.
- Switching in the game: the game lists the saved accounts (`belumi-accounts.json` in the game folder, names and ids only); picking one writes `belumi-switch.json` and quits, and the launcher signs that account in and starts the game on the same server. Not available when the launcher closes itself at start.
- A server can have its own account (select the server, pick one under Account): the launcher switches to it before
  the start.
- Headless start for shortcuts and scripts: `belumi-launcher --play [--offline] [--server HOST[:PORT] | --world NAME | --main-menu]`
  renews the saved account and starts the game without opening a window.
- Launch options (Settings): fullscreen, window size, graphics API (Vulkan or DirectX 12), and what the launcher does
  after the game starts (stay, minimise, close). A game folder can be set too.
- After a crash the Play page shows the last log line with "Open folder", "Copy report" and "Pack logs for support"
  (a ZIP of the game log and the crash report with the home folder and token-like strings removed).
- A first-start check lists a missing game file, missing prepared resources, an unwritable game folder or a missing
  Vulkan driver, with the path and what to do.
- Drop a shader pack `.zip` on the window to copy it into the game's `shaderpacks` folder.
- Game log -> Run diagnose (or `belumi-launcher --diagnose`) collects a support report: see [diagnose.md](diagnose.md).
- The launcher speaks English and German (Settings, or by the system language).

## Updates

At start the launcher asks GitHub for a newer release (Settings, Updates: "Check for updates at start"; "Check now" asks
again). If there is one you get a desktop notification (Linux), the status line says so and the Play page shows a banner with
**Update now**. The button downloads the archive of your system, checks its SHA-256, replaces `belumi-launcher` and
`horizon-client` next to the running launcher and offers **Restart now**. A launcher installed system-wide (`/usr`, a package)
cannot change itself: update it with the package manager or the installer. Details: [releases.md](releases.md).

## Shaders (Solas)

Belumi's look is the Solas shader pack by Septonious. Solas is "All Rights Reserved", so Belumi does not ship it: on the first
start the Play page asks "Download Solas?" and, on yes, the launcher fetches **Solas 3.9**, the version Belumi has been tested with, from Modrinth (the
author's official page), checks its size and SHA-512 (against the tested file, not only against what Modrinth reports) and saves it in the game folder's `shaderpacks`. The game picks it by itself
the first time it finds a Solas pack there. "Not now" asks once and never again; Settings, Shaders has the button and the link to
the Modrinth page. Anything but Modrinth's download host, or a file that is not the tested one, is refused. A newer Solas is
not installed automatically, because it may look different or need things the client cannot do yet; after trying one in the game,
move `PINNED_VERSION` and `PINNED_SHA512` in `crates/belumi-launcher/src/shaders.rs`.

## Discord

Settings, Discord: "Show Belumi on my Discord profile" (launcher and game; the launcher starts the game with
`BELUMI_NO_DISCORD=1` when it is off) and "Show my account name in the launcher". The game's own options (server address,
world name, time, game mode) are in the game under Right Shift, Modules, Discord presence. See
[discord-presence.md](discord-presence.md).

## Looks

Settings -> Look switches between three styles: Modern (dark, calm), Modern Glass (gradient with translucent panels) and
Pixel (parchment, wood and chunky outlines in the spirit of a farm sim; own art, no game assets; the old `minecraft` value still loads). The choice is saved as `style` in `launcher.json`.
Tokens and rules are in `DESIGN.md`. The game log stays solid in every style, and a test checks that text, muted text,
danger text and text on the accent colour keep their contrast on every surface of every style.

Development aid: `BELUMI_LAUNCHER_SHOTS=DIR belumi-launcher` saves a PNG of every page in every style (with two made-up
accounts) and quits.

## Several accounts and renewal

- "Add account" signs in another Microsoft account; every account stays saved (refresh tokens in the keyring or the
  owner-only file) and "Use" switches between them. "Remove" forgets one. Older settings with one saved account
  are migrated into the list.
- The launcher renews the active account when it starts, when you switch, with "Renew now" and in the background
  about 15 minutes before its Minecraft token ends.
- The game gets the client ID, the refresh token and the token's end time in the session JSON. It renews the
  account itself shortly before joining a server and again when the session service says "invalid session", so
  a long play session or a reconnect after a day needs no restart. Prism accounts carry no refresh token and
  are not renewed in the game.

## Using a Prism Launcher account

"Use my Prism account" (and the checkbox "Keep using Prism's login automatically") reads
only the active Microsoft account's name, UUID and Minecraft access token from Prism's
`accounts.json` (`~/.local/share/PrismLauncher/`, the Flatpak path, or the path you enter).
Nothing is written to Prism's files and Prism's refresh token is never used: it belongs to
Prism's app.

- The token lasts about 24 hours and Prism renews it while Prism runs. The launcher re-reads
  the file every few seconds, picks up the renewed token, and reads it once more right before
  every start. If the token has expired it says so: open Prism once, the launcher continues
  by itself.
- Priority: the Belumi app's own sign-in wins; Prism is the fallback while it is not available.
- "Sign out" also turns the Prism login off.

## You need your own Azure app (client ID)

There is no public client ID for Minecraft launchers. Minecraft services answer 403
"Invalid app registration" for any app Mojang hasn't approved, and borrowing another
launcher's ID would impersonate it.

1. Azure portal -> Microsoft Entra ID -> App registrations -> New registration, account
   type **Personal Microsoft accounts only**, then Authentication -> **Allow public client
   flows: Yes**. No secret, no permissions to add.
2. Request Mojang's review for the Application (client) ID: https://aka.ms/mce-reviewappid
   (days to weeks).
3. Put the ID into `BUILT_IN_CLIENT_ID` in `crates/belumi-launcher/src/settings.rs` (it holds a placeholder), or enter it in the launcher / set `BELUMI_MSA_CLIENT_ID`. Until Mojang approves it,
   sign-in stops at the last step with a message saying so.

## Look, server list, log, keyring

- Pages: Play (account card, server list, Play / Play offline), Account, Game log (tail of `game.log`), Settings.
- Server list lives in `launcher.json` (`servers`, `selected_server`); an old single `server` field is migrated.
- The Microsoft refresh token is stored in the desktop keyring via `secret-tool` (libsecret); the file then only names
  the account. Without `secret-tool` or a keyring the token stays in the owner-only file.
- Light on resources: no images or animation, redraws only on events plus a slow tick (250 ms while signing in or
  starting, 500 ms on the log page while the game runs, 2 s with Prism watching, otherwise 10 s). Idle CPU measured at 0.

## Dev scripts

- `tools/dev-launcher.sh` builds the launcher (release) and starts it; also installs the menu entry and icon.
- `tools/run-launcher.sh` only starts the built launcher.
- `packaging/install-local.sh` installs the desktop entry `belumi-launcher` (the window's app id) and the
  icon for the current user; the Wayland task bar takes its icon from that entry.
