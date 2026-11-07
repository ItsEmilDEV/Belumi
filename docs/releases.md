# Releases and updates

## Build an archive

```sh
packaging/build-release.sh            # version from Cargo.toml
packaging/build-release.sh 0.2.0      # explicit version
```

Every build has voice chat (it needs `libopus` and CMake to build) and client plugins: the default feature `belumi` includes them.

```sh
packaging/build-release.sh 0.2.0 && ls dist
```

The result in `dist/`, each with a `.sha256` file: `belumi-<version>-linux-<arch>.tar.gz` (any distro), `belumi_<version>_<arch>.deb`
(Debian family), `belumi-<version>-1.<arch>.rpm` (Fedora, openSUSE; needs `rpmbuild`) and `belumi-<version>-<arch>.AppImage` (any distro, one
file; downloads `appimagetool` unless it is installed, pin it with `APPIMAGETOOL_SHA256`). Locally a missing tool only skips its
format; in CI it fails the build. The `.tar.gz` holds `belumi-launcher`, `horizon-client`,
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

Tagging `vX.Y.Z` runs `.github/workflows/release.yml`: it builds the Linux archive and the `.deb` on Ubuntu, then the Windows zip
and the installer (Inno Setup) on Windows, and attaches all of them with their `.sha256` files to the GitHub release. A tag
with a `-` (`v0.2.0-beta.1`) becomes a prerelease. The workflow has not run yet; the first tag is its test.

The launcher's updater needs these names: `belumi-<version>-linux-<arch>.tar.gz` and `belumi-<version>-windows-x64.zip`, each
with a `<name>.sha256` file next to it. The Windows installer is `belumi-<version>-windows-x64-setup.exe`.

## Update channels

The launcher asks GitHub's public release list of `ItsEmilDEV/Belumi` once at start (Settings: "Check for updates at
start", off with one click; "Check now" asks again). When a newer version exists it shows a desktop notification (Linux),
a message in the status line and a banner on the Play page.

**Update now** downloads the archive of this system from the release, checks its SHA-256 against the release's `.sha256` file,
unpacks it and replaces `belumi-launcher` and `horizon-client` next to the running launcher (the old ones stay as `*.old`
until the next start); then **Restart now** starts the new launcher. Nothing is installed without the button, and a download
that is not from `github.com/ItsEmilDEV/Belumi/releases/download/` or has no matching checksum is refused. If the folder is not
writable (a system-wide install), the launcher says so: use the installer or your package manager then.

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

## The first release, step by step

1. **Check the version.** `version` in the root `Cargo.toml` is the number the files get when built locally; the workflow takes the
   version from the tag. For `v0.1.0` nothing needs to change. Update `CHANGELOG.md` (change "Unreleased" to the date).
2. **Rehearse with a prerelease.** `git tag v0.1.0-beta.1 && git push origin v0.1.0-beta.1`. Watch the Actions tab
   ("Release"): the `linux` job builds the archive, `.deb`, `.rpm` and AppImage, the `windows` job the zip and the installer.
   A tag with a `-` becomes a prerelease automatically. If a job fails, fix it, delete the tag and the release
   (`git push origin :refs/tags/v0.1.0-beta.1`, then "Delete" on the release page) and tag again.
3. **Try the downloads** on a real machine: install the `.deb` or run the AppImage, run the Windows installer, check the checksums
   (`sha256sum -c belumi-*.sha256`).
4. **Tag the release.** `git tag v0.1.0 && git push origin v0.1.0`. The workflow uses `docs/release-notes/v0.1.0.md` as the
   description and "Belumi 0.1.0" as the title. Without a notes file it generates them from the commits.
5. **Look at the release page.** Edit the text if needed. Mark it "Set as the latest release" (a prerelease is not offered to the
   stable channel).
6. **Test the updater** with a second, newer release: install 0.1.0, publish 0.1.1, start the launcher, press **Update now**.

Repository settings worth setting once: description "Minecraft: Java Edition client and launcher written in Rust", website empty,
topics `minecraft`, `minecraft-client`, `minecraft-launcher`, `rust`, `wgpu`, `voice-chat`, `discord-rich-presence`.
