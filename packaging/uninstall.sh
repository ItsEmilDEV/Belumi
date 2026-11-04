#!/usr/bin/env bash
# Removes what install.sh put in the prefix (default ~/.local). Settings and worlds stay.
set -euo pipefail
prefix="${HOME}/.local"
if [ "${1:-}" = "--prefix" ]; then
  prefix="${2:?--prefix needs a path}"
fi
rm -f "$prefix/bin/belumi-launcher" "$prefix/bin/horizon-client" \
  "$prefix/share/applications/belumi-launcher.desktop" "$prefix/share/applications/belumi.desktop" \
  "$prefix/share/icons/hicolor/512x512/apps/belumi.png"
echo "Removed Belumi from $prefix. Your settings (~/.config/belumi) and game data stay."
