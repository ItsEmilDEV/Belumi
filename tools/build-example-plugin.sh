#!/bin/sh
set -eu
out="${1:?usage: $0 OUT_DIR}"
cd "$(dirname "$0")/.."
cargo build -q --release --target wasm32-unknown-unknown -p hello-plugin
mkdir -p "$out/hello"
cp examples/hello-plugin/plugin.toml "$out/hello/plugin.toml"
cp target/wasm32-unknown-unknown/release/hello_plugin.wasm "$out/hello/plugin.wasm"
echo "plugin written to $out/hello"
