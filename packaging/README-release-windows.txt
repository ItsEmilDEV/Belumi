Belumi for Windows
==================

Start belumi-launcher.exe. Sign in with a Microsoft account that owns Minecraft: Java Edition, then press Play.

* The game needs a graphics driver with Vulkan or DirectX 12 (every current GPU driver has one).
* Windows may show "Unknown publisher" (SmartScreen) because the files are not code-signed yet:
  choose "More info" and then "Run anyway".
* The game's files come from Mojang: after you sign in, the launcher's "Download and prepare" button fetches them from
  Mojang's own servers (about 1 GB) and prepares them. Nothing of Mojang's ships with Belumi.
* Settings, accounts and logs live in %APPDATA%\Belumi. The sign-in is kept in the Windows Credential Manager
  (or, if it is too large for it, in an owner-only file there).

Command line: belumi-launcher.exe --play [--offline] [--server HOST[:PORT] | --world NAME | --main-menu]
