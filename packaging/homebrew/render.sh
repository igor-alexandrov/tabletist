#!/bin/bash
# Fill the Homebrew cask template for one release.
#
#   packaging/homebrew/render.sh <tabletist.rb.in> <version> <checksums.txt> > tabletist.rb
#
# <version> has no leading v. The checksum comes from the release's
# checksums.txt (sha256sum format). The rendered cask is Casks/tabletist.rb
# in igor-alexandrov/homebrew-tabletist.
set -euo pipefail

template="$1"
version="$2"
checksums="$3"

name="tabletist-v${version}-macos-universal.dmg"
dmg="$(awk -v name="$name" '$2 == name || $2 == "*" name { print $1 }' "$checksums")"
[ -n "$dmg" ] || { echo "no checksum for $name in $checksums" >&2; exit 1; }

sed -e "s/@VERSION@/${version}/g" -e "s/@SHA256@/${dmg}/g" "$template"
