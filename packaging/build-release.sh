#!/usr/bin/env bash
# Builds the Linux release files in dist/: the .tar.gz (any distro), .deb, .rpm and AppImage, each with its .sha256 file.
# Usage: packaging/build-release.sh [version]   (default: the launcher's Cargo version)
# Env:   BELUMI_FEATURES   client cargo features (default: belumi, which brings plugins and voice chat along)
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

version="${1:-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}"
arch="$(uname -m)"
features="${BELUMI_FEATURES-belumi}"
# The launcher knows which version it is (the updater compares it, the window shows it).
export BELUMI_VERSION="$version"
name="belumi-${version}-linux-${arch}"
stage="dist/${name}"

rm -rf "$stage"
mkdir -p "$stage/bin" "$stage/share/applications" "$stage/share/icons/hicolor/512x512/apps"

cargo build --release -p belumi-launcher
( cd vendor/horizon && cargo build --release -p horizon-client ${features:+--features "$features"} )

install -m755 target/release/belumi-launcher "$stage/bin/belumi-launcher"
install -m755 vendor/horizon/target/release/horizon-client "$stage/bin/horizon-client"
install -m644 packaging/belumi-launcher.desktop "$stage/share/applications/belumi-launcher.desktop"
install -m644 packaging/belumi.desktop "$stage/share/applications/belumi.desktop"
install -m644 packaging/belumi-game-512.png "$stage/share/icons/hicolor/512x512/apps/belumi.png"
packaging/stage-horizon-files.sh "$stage/share/belumi/horizon-files"
install -m755 packaging/install.sh "$stage/install.sh"
install -m755 packaging/uninstall.sh "$stage/uninstall.sh"
install -m644 LICENSE "$stage/LICENSE"
install -m644 packaging/README-release.txt "$stage/README.txt"

( cd dist && tar --owner=0 --group=0 -czf "${name}.tar.gz" "$name" && sha256sum "${name}.tar.gz" > "${name}.tar.gz.sha256" )
# The other formats from the same staged folder: .deb (Debian, Ubuntu), .rpm (Fedora, openSUSE), AppImage (any distro).
# On a developer's machine a missing tool only skips its format; in CI (CI is set) it fails the build.
for format in deb rpm appimage; do
  packaging/build-$format.sh "$stage" "$version" dist || {
    [ -z "${CI:-}" ] || exit 1
    echo "skipped the $format package"
  }
done
echo "dist/${name}.tar.gz"
cat "dist/${name}.tar.gz.sha256"
