#!/usr/bin/env bash
# Installs Belumi for the current user (default prefix ~/.local) or system-wide: ./install.sh [--prefix /usr/local]
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
prefix="${HOME}/.local"
if [ "${1:-}" = "--prefix" ]; then
  prefix="${2:?--prefix needs a path}"
fi

install -d "$prefix/bin" "$prefix/share/applications" "$prefix/share/icons/hicolor/512x512/apps"
install -m755 "$here/bin/belumi-launcher" "$prefix/bin/belumi-launcher"
install -m755 "$here/bin/horizon-client" "$prefix/bin/horizon-client"
install -m644 "$here/share/icons/hicolor/512x512/apps/belumi.png" "$prefix/share/icons/hicolor/512x512/apps/belumi.png"
for entry in belumi-launcher belumi; do
  exe="$prefix/bin/$([ "$entry" = belumi ] && echo horizon-client || echo belumi-launcher)"
  awk -v exe="$exe" '/^Exec=/ { sub(/^Exec=[^ ]*/, "Exec=" exe) } { print }' \
    "$here/share/applications/$entry.desktop" > "$prefix/share/applications/$entry.desktop"
done
rm -rf "$prefix/share/belumi/horizon-files" && mkdir -p "$prefix/share/belumi" && cp -R "$here/share/belumi/horizon-files" "$prefix/share/belumi/horizon-files"
command -v update-desktop-database >/dev/null && update-desktop-database "$prefix/share/applications" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q "$prefix/share/icons/hicolor" 2>/dev/null || true

echo "Installed to $prefix. Start 'belumi-launcher' or find Belumi in your application menu."
echo "On the first start the launcher downloads the game files from Mojang and prepares them (sign in first)."
