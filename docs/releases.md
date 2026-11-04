# Releases and updates

## Build an archive

```sh
packaging/build-release.sh            # version from Cargo.toml
packaging/build-release.sh 0.2.0      # explicit version
BELUMI_FEATURES=belumi,belumi-plugins,belumi-voice packaging/build-release.sh   # with voice chat (needs libopus)
```

The result is `dist/belumi-<version>-linux-<arch>.tar.gz` plus a `.sha256` file. It holds `belumi-launcher`, `horizon-client`,
the desktop entries, the icon, `install.sh`, `uninstall.sh`, `README.txt` and the license. No Mojang file is included: the
game still needs resources prepared from the player's own install ([DEVELOPMENT.md](DEVELOPMENT.md)).

## Install

```sh
tar xzf belumi-0.2.0-linux-x86_64.tar.gz && cd belumi-0.2.0-linux-x86_64
./install.sh                        # ~/.local
./install.sh --prefix /usr/local    # system-wide, as root
```

`uninstall.sh` removes the same files and leaves settings and worlds alone. An installed launcher finds `horizon-client`
next to itself and looks for resources in `~/.local/share/belumi/resources/26.3`.

## Publish

Tagging `vX.Y.Z` runs `.github/workflows/release.yml`: it builds the archive on Ubuntu and creates the GitHub release with
the archive and checksum attached. A tag with a `-` (`v0.2.0-beta.1`) becomes a prerelease. The workflow has not run yet;
the first tag is its test.

## Update channels

The launcher asks GitHub's public release list of `ItsEmilDEV/Belumi` once at start (Settings: "Check for updates at
start", off with one click) and shows "Update available" with a link on the Play page. Nothing is downloaded or
installed automatically.

| Channel | Offers |
|---|---|
| stable | releases without a prerelease mark |
| nightly | every non-draft release, prereleases included |

Versions compare by number (`0.10.0` is newer than `0.9.9`, a release is newer than its own prereleases).

## Game versions per server

Every server entry has a version: Auto-detect (the game asks the server's status ping), 26.3 (native) or 26.2 (translated
by the protocol layer, [wire-differences-26.2.md](wire-differences-26.2.md)). The choice becomes the client's `--protocol`
option. Adding an older version later is one line in `crates/belumi-launcher/src/versions.rs` once the protocol layer
supports it.
