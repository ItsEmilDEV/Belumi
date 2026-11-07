#!/usr/bin/env bash
# Builds an AppImage (one file that runs on any Linux distro) from a staged release folder.
# Usage: packaging/build-appimage.sh STAGE_DIR VERSION OUT_DIR
# libopus travels inside the image (the game links it dynamically); the GPU driver, ALSA and Vulkan come from the system.
set -euo pipefail

stage="${1:?stage folder (dist/belumi-<version>-linux-<arch>)}"
version="${2:?version}"
out="${3:?output folder}"
arch="$(uname -m)"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
app="$work/Belumi.AppDir"
mkdir -p "$app/usr/bin" "$app/usr/lib" "$out"
install -m755 "$stage/bin/belumi-launcher" "$stage/bin/horizon-client" "$app/usr/bin/"

# The libraries only the game needs and a system may lack: Opus.
opus="$(ldd "$stage/bin/horizon-client" | awk '/libopus\.so/ { print $3; exit }')"
if [ -n "$opus" ] && [ -f "$opus" ]; then
  install -m644 "$(readlink -f "$opus")" "$app/usr/lib/$(basename "$opus")"
fi

install -m644 "$stage/share/icons/hicolor/512x512/apps/belumi.png" "$app/belumi.png"
cp "$app/belumi.png" "$app/.DirIcon"
cat > "$app/belumi.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Belumi
Comment=Minecraft 26.2 / 26.3 client launcher
Exec=belumi-launcher
Icon=belumi
StartupWMClass=belumi-launcher
Categories=Game;
Terminal=false
DESKTOP
cat > "$app/AppRun" <<'APPRUN'
#!/bin/sh
here="$(dirname "$(readlink -f "$0")")"
export LD_LIBRARY_PATH="$here/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec "$here/usr/bin/belumi-launcher" "$@"
APPRUN
chmod +x "$app/AppRun"
install -m644 "$stage/LICENSE" "$app/LICENSE"

tool="$(command -v appimagetool || true)"
if [ -z "$tool" ]; then
  tool="$work/appimagetool"
  curl -fsSL -o "$tool" "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${arch}.AppImage"
  # Pin the tool once a release is chosen: APPIMAGETOOL_SHA256=<digest> refuses any other file.
  if [ -n "${APPIMAGETOOL_SHA256:-}" ]; then
    echo "${APPIMAGETOOL_SHA256}  $tool" | sha256sum -c - >/dev/null
  fi
  chmod +x "$tool"
fi
file="$out/belumi-${version}-${arch}.AppImage"
rm -f "$file"
# --appimage-extract-and-run: runs without FUSE (containers, CI).
ARCH="$arch" "$tool" --appimage-extract-and-run --no-appstream "$app" "$file" >/dev/null
( cd "$out" && sha256sum "$(basename "$file")" > "$(basename "$file").sha256" )
echo "$file"
