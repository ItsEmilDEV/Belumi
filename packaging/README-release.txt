Belumi
======

1. Run ./install.sh (user install under ~/.local) or ./install.sh --prefix /usr/local (as root).
2. Start "belumi-launcher". Sign in or play offline, pick a server and its game version.

Requirements: a Vulkan driver, ALSA, libopus (voice chat), and your own copy of Minecraft: Java Edition. The game needs resources
prepared from your install (they are Mojang's files and are never shipped here): see docs/DEVELOPMENT.md,
"Prepared resources". Put them in ~/.local/share/belumi/resources/26.3 or set the path in the launcher's Settings.

Belumi is built on Horizon (https://github.com/horizon-rust/horizon). MIT license, see LICENSE.
Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft.
