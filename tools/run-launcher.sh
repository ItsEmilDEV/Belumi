#!/bin/sh
root=$(cd "$(dirname "$0")/.." && pwd)
bin="$root/target/release/belumi-launcher"
[ -x "$bin" ] || { echo "Not built yet: run tools/dev-launcher.sh" >&2; exit 1; }
exec "$bin" "$@"
