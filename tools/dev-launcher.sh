#!/bin/sh
set -e
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
cargo build --release -p belumi-launcher
sh "$root/packaging/install-local.sh" >/dev/null 2>&1 || true
exec "$root/target/release/belumi-launcher" "$@"
