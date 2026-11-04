#!/usr/bin/env bash
# Builds the Linux release archive: dist/belumi-<version>-linux-<arch>.tar.gz and its SHA256SUMS.
# Usage: packaging/build-release.sh [version]   (default: the launcher's Cargo version)
# Env:   BELUMI_FEATURES   client cargo features (default: belumi,belumi-plugins; add belumi-voice for voice chat)
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

version="${1:-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}"
arch="$(uname -m)"
features="${BELUMI_FEATURES-belumi,belumi-plugins}"
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
install -m755 packaging/install.sh "$stage/install.sh"
install -m755 packaging/uninstall.sh "$stage/uninstall.sh"
install -m644 LICENSE "$stage/LICENSE"
install -m644 packaging/README-release.txt "$stage/README.txt"

( cd dist && tar --owner=0 --group=0 -czf "${name}.tar.gz" "$name" && sha256sum "${name}.tar.gz" > "${name}.tar.gz.sha256" )
echo "dist/${name}.tar.gz"
cat "dist/${name}.tar.gz.sha256"
