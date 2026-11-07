#!/usr/bin/env bash
# Builds a .deb from a staged release folder made by build-release.sh.
# Usage: packaging/build-deb.sh STAGE_DIR VERSION OUT_DIR
# Uses dpkg-deb when it exists, else assembles the ar archive by hand (works on any Linux with binutils).
set -euo pipefail

stage="${1:?stage folder (dist/belumi-<version>-linux-<arch>)}"
version="${2:?version}"
# Debian sorts a prerelease before its release with "~": 0.2.0~beta.1 comes before 0.2.0.
file_version="$version"   # GitHub renames "~" in file names, so the file keeps the dash
version="$(printf '%s' "$version" | tr - '~')"
out="${3:?output folder}"
case "$(uname -m)" in
  x86_64) arch=amd64 ;;
  aarch64) arch=arm64 ;;
  *) echo "no .deb for $(uname -m)"; exit 0 ;;
esac

root="$(mktemp -d)"
trap 'rm -rf "$root"' EXIT
pkg="$root/pkg"
mkdir -p "$pkg/DEBIAN" "$pkg/usr/bin" "$pkg/usr/share/applications" "$pkg/usr/share/icons/hicolor/512x512/apps" "$pkg/usr/share/doc/belumi"
install -m755 "$stage/bin/belumi-launcher" "$stage/bin/horizon-client" "$pkg/usr/bin/"
install -m644 "$stage/share/applications/"*.desktop "$pkg/usr/share/applications/"
install -m644 "$stage/share/icons/hicolor/512x512/apps/belumi.png" "$pkg/usr/share/icons/hicolor/512x512/apps/belumi.png"
install -m644 "$stage/LICENSE" "$pkg/usr/share/doc/belumi/copyright"
install -m644 "$stage/README.txt" "$pkg/usr/share/doc/belumi/README.txt"

size="$(du -sk "$pkg/usr" | cut -f1)"
cat > "$pkg/DEBIAN/control" <<CONTROL
Package: belumi
Version: ${version}
Section: games
Priority: optional
Architecture: ${arch}
Installed-Size: ${size}
Depends: libasound2t64 | libasound2, libopus0, libvulkan1, libudev1
Maintainer: Belumi <noreply@users.noreply.github.com>
Homepage: https://github.com/ItsEmilDEV/Belumi
Description: Belumi, a Minecraft: Java Edition client and launcher written in Rust
 Not made or approved by Mojang or Microsoft. The game needs resources prepared
 from your own Minecraft installation (see /usr/share/doc/belumi/README.txt).
CONTROL

mkdir -p "$out"
deb="$out/belumi_${file_version}_${arch}.deb"
if command -v dpkg-deb >/dev/null; then
  dpkg-deb --root-owner-group --build "$pkg" "$deb" >/dev/null
else
  ( cd "$pkg/DEBIAN" && tar --owner=0 --group=0 -czf "$root/control.tar.gz" ./control )
  ( cd "$pkg" && tar --owner=0 --group=0 --exclude=./DEBIAN -czf "$root/data.tar.gz" . )
  printf '2.0\n' > "$root/debian-binary"
  rm -f "$deb"
  ( cd "$root" && ar rc "$deb" debian-binary control.tar.gz data.tar.gz )
fi
( cd "$out" && sha256sum "$(basename "$deb")" > "$(basename "$deb").sha256" )
echo "$deb"
