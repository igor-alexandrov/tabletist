#!/bin/bash
# Fill a Homebrew template for one release: the cask or the formula.
#
#   packaging/homebrew/render.sh <tabletist.rb.in> <version> <checksums.txt> > Casks/tabletist.rb
#   packaging/homebrew/render.sh <tabletist-formula.rb.in> <version> <checksums.txt> > Formula/tabletist.rb
#
# <version> has no leading v. The checksums come from the release's
# checksums.txt (sha256sum format): @SHA256@ is the DMG's, @SOURCE_SHA256@
# the source archive's. Both files go to igor-alexandrov/homebrew-tabletist.
set -euo pipefail

template="$1"
version="$2"
checksums="$3"

# The checksum of one file of the release.
sum() {
  local name="$1" found
  found="$(awk -v name="$name" '$2 == name || $2 == "*" name { print $1 }' "$checksums")"
  [ -n "$found" ] || { echo "no checksum for $name in $checksums" >&2; exit 1; }
  printf '%s' "$found"
}

dmg=""
source=""
if grep -q '@SHA256@' "$template"; then
  dmg="$(sum "tabletist-v${version}-macos-universal.dmg")"
fi
if grep -q '@SOURCE_SHA256@' "$template"; then
  source="$(sum "tabletist-v${version}-source.tar.gz")"
fi

sed -e "s/@VERSION@/${version}/g" -e "s/@SHA256@/${dmg}/g" \
  -e "s/@SOURCE_SHA256@/${source}/g" "$template"
