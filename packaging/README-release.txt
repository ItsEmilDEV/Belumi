Belumi
======

1. Run ./install.sh (user install under ~/.local) or ./install.sh --prefix /usr/local (as root).
2. Start "belumi-launcher". Sign in or play offline, pick a server and its game version.

Requirements: a Vulkan driver, ALSA, libopus (voice chat), and a Microsoft account that owns Minecraft: Java Edition. Mojang's files
are never shipped here: on the first start the launcher offers "Download and prepare" (about 1 GB, a few minutes, after you sign in)
and fetches the game from Mojang's own servers. You can also prepare it yourself: see docs/DEVELOPMENT.md, "Prepared resources".

Belumi is built on Horizon (https://github.com/horizon-rust/horizon). MIT license, see LICENSE.
Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft.
