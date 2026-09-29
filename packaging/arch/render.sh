#!/bin/bash
# Fill a PKGBUILD template for one release.
#
#   packaging/arch/render.sh <PKGBUILD.in> <version> <checksums.txt> > PKGBUILD
#
# <version> has no leading v. The checksums come from the release's
# checksums.txt (sha256sum format).
set -euo pipefail

template="$1"
version="$2"
checksums="$3"

sum() {
  local name="$1"
  local found
  found="$(awk -v name="$name" '$2 == name || $2 == "*" name { print $1 }' "$checksums")"
  [ -n "$found" ] || { echo "no checksum for $name in $checksums" >&2; exit 1; }
  printf '%s' "$found"
}

amd64="$(sum "tabletist-v${version}-x86_64-unknown-linux-gnu.tar.gz")"
arm64="$(sum "tabletist-v${version}-aarch64-unknown-linux-gnu.tar.gz")"
source="$(sum "tabletist-v${version}-source.tar.gz")"

sed -e "s/@VERSION@/${version}/g" -e "s/@PKGREL@/1/g" \
    -e "s/@AMD64_SHA256@/${amd64}/g" -e "s/@ARM64_SHA256@/${arm64}/g" \
    -e "s/@SOURCE_SHA256@/${source}/g" "$template"
