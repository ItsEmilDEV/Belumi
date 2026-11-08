#!/usr/bin/env bash
# Builds a .rpm (Fedora, openSUSE, RHEL family) from a staged release folder made by build-release.sh.
# Usage: packaging/build-rpm.sh STAGE_DIR VERSION OUT_DIR      (needs rpmbuild)
set -euo pipefail

stage="$(cd "${1:?stage folder (dist/belumi-<version>-linux-<arch>)}" && pwd)"
version="${2:?version}"
out="${3:?output folder}"
command -v rpmbuild >/dev/null || { echo "rpmbuild is missing (dnf install rpm-build, apt install rpm)"; exit 1; }

# RPM versions have no "-": 0.2.0-beta.1 becomes version 0.2.0, release 0.beta.1 (which sorts before 0.2.0-1).
if [ "${version#*-}" != "$version" ]; then
  rpm_version="${version%%-*}"
  rpm_release="0.${version#*-}"
else
  rpm_version="$version"
  rpm_release="1"
fi

top="$(mktemp -d)"
trap 'rm -rf "$top"' EXIT
mkdir -p "$top"/{BUILD,RPMS,SPECS,SRPMS} "$out"
cat > "$top/SPECS/belumi.spec" <<'SPEC'
Name:           belumi
Version:        %{rpm_version}
Release:        %{rpm_release}%{?dist}
Summary:        A Minecraft: Java Edition client and launcher written in Rust
License:        MIT
URL:            https://github.com/ItsEmilDEV/Belumi
ExclusiveArch:  x86_64 aarch64
AutoReqProv:    no
# Libraries by soname, so the same package installs on Fedora (alsa-lib, opus ...) and openSUSE (libasound2, libopus0 ...).
Requires:       libasound.so.2()(64bit), libopus.so.0()(64bit), libvulkan.so.1()(64bit), libudev.so.1()(64bit)

%global debug_package %{nil}
%global __strip /bin/true
%global _build_id_links none

%description
Belumi is a Minecraft: Java Edition client and launcher written in Rust, with voice chat, replays,
a HUD builder and Discord Rich Presence. It is not made or approved by Mojang or Microsoft.
On the first start the launcher downloads the game from Mojang and prepares it (see the README).

%install
install -Dm755 %{stage}/bin/belumi-launcher %{buildroot}%{_bindir}/belumi-launcher
install -Dm755 %{stage}/bin/horizon-client %{buildroot}%{_bindir}/horizon-client
install -Dm644 %{stage}/share/applications/belumi-launcher.desktop %{buildroot}%{_datadir}/applications/belumi-launcher.desktop
install -Dm644 %{stage}/share/applications/belumi.desktop %{buildroot}%{_datadir}/applications/belumi.desktop
install -Dm644 %{stage}/share/icons/hicolor/512x512/apps/belumi.png %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/belumi.png
mkdir -p %{buildroot}%{_datadir}/belumi && cp -R %{stage}/share/belumi/horizon-files %{buildroot}%{_datadir}/belumi/horizon-files
install -Dm644 %{stage}/LICENSE %{buildroot}%{_datadir}/licenses/belumi/LICENSE
install -Dm644 %{stage}/README.txt %{buildroot}%{_docdir}/belumi/README.txt

%files
%{_bindir}/belumi-launcher
%{_bindir}/horizon-client
%{_datadir}/applications/belumi-launcher.desktop
%{_datadir}/applications/belumi.desktop
%{_datadir}/icons/hicolor/512x512/apps/belumi.png
%{_datadir}/belumi/horizon-files
%license %{_datadir}/licenses/belumi/LICENSE
%doc %{_docdir}/belumi/README.txt
SPEC
rpmbuild -bb --quiet --define "_topdir $top" --define "stage $stage" \
  --define "rpm_version $rpm_version" --define "rpm_release $rpm_release" "$top/SPECS/belumi.spec"
rpm="$(find "$top/RPMS" -name '*.rpm' | head -1)"
cp "$rpm" "$out/"
( cd "$out" && sha256sum "$(basename "$rpm")" > "$(basename "$rpm").sha256" )
echo "$out/$(basename "$rpm")"
