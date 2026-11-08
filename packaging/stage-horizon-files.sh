#!/usr/bin/env bash
# Copies what the launcher needs to prepare the game on the player's computer into a folder: the pinned versions and
# checksums (research-lock.json) and the sources of Horizon's own exporters. No Minecraft file is among them.
# Usage: packaging/stage-horizon-files.sh <destination folder>
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
dest="${1:?destination folder}"
rm -rf "$dest"
mkdir -p "$dest/mods/horizon/src/test"
cp "$root/vendor/horizon/research-lock.json" "$dest/"
cp -R "$root/vendor/horizon/mods/horizon/src/test/java" "$dest/mods/horizon/src/test/"
