#!/bin/sh
set -e
here=$(cd "$(dirname "$0")" && pwd)
root=$(dirname "$here")
bin="${1:-$root/target/release/belumi-launcher}"
src="$root/crates/belumi-launcher/assets/logo-512.png"
for size in 64 128 256 512; do
    dir="$HOME/.local/share/icons/hicolor/${size}x${size}/apps"
    mkdir -p "$dir"
    magick "$src" -resize "${size}x${size}" "PNG32:$dir/belumi.png" 2>/dev/null || cp "$src" "$dir/belumi.png"
done
mkdir -p "$HOME/.local/share/applications"
sed -e "s|^Exec=.*|Exec=$bin|" -e "s|^Icon=.*|Icon=$HOME/.local/share/icons/hicolor/256x256/apps/belumi.png|" "$here/belumi-launcher.desktop" > "$HOME/.local/share/applications/belumi-launcher.desktop"
gicons="$HOME/.local/share/icons/hicolor/256x256/apps"
magick "$root/packaging/belumi-game-512.png" -resize 256x256 "PNG32:$gicons/belumi-game.png" 2>/dev/null || cp "$root/packaging/belumi-game-512.png" "$gicons/belumi-game.png"
client="${HORIZON_CLIENT:-$root/vendor/horizon/target/fast/horizon-client}"
sed -e "s|^Exec=.*|Exec=$client|" -e "s|^Icon=.*|Icon=$gicons/belumi-game.png|" "$here/belumi.desktop" > "$HOME/.local/share/applications/belumi.desktop"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
gtk-update-icon-cache -q -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
kbuildsycoca6 >/dev/null 2>&1 || true
echo "Installed menu entry and icon. Restart the launcher to see the task bar icon."
