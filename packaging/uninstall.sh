#!/usr/bin/env bash
# Removes what install.sh put in the prefix (default ~/.local). Settings and worlds stay. --cache removes only caches and logs, --all also the data.
set -euo pipefail
prefix="${HOME}/.local"
mode=app
while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) prefix="${2:?--prefix needs a path}"; shift 2 ;;
    --cache) mode=cache; shift ;;
    --all) mode=all; shift ;;
    *) echo "Usage: uninstall.sh [--prefix DIR] [--cache | --all]"; exit 2 ;;
  esac
done
if [ "$mode" = cache ]; then
  rm -rf "${HOME}/.config/belumi/heads" "${HOME}/.local/share/belumi/logs" "${HOME}/.local/share/belumi/belumi-report" \
    "${HOME}/.local/share/horizon/logs" "${HOME}/.local/share/horizon/crash-reports"
  echo "Removed the cache and usage data. Settings, accounts and worlds stay."
  exit 0
fi
rm -f "$prefix/bin/belumi-launcher" "$prefix/bin/horizon-client" \
  "$prefix/share/applications/belumi-launcher.desktop" "$prefix/share/applications/belumi.desktop" \
  "$prefix/share/icons/hicolor/512x512/apps/belumi.png"
rm -rf "$prefix/share/belumi/horizon-files"; rmdir "$prefix/share/belumi" 2>/dev/null || true
if [ "$mode" = all ]; then
  rm -rf "${HOME}/.config/belumi" "${HOME}/.local/share/belumi" "${HOME}/.local/share/horizon"
  echo "Removed Belumi from $prefix and all its data (settings, accounts, resources). Worlds in your game folder stay."
else
  echo "Removed Belumi from $prefix. Your settings (~/.config/belumi) and game data stay."
fi
