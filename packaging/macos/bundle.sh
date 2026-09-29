#!/bin/bash
# Build Tabletist.app from a (universal) binary, on a macOS machine.
#
#   packaging/macos/bundle.sh <binary> <output.app> <version>
#
# Set CODESIGN_IDENTITY to sign with a Developer ID (hardened runtime, for
# notarization); otherwise the app gets an ad-hoc signature, which Apple
# Silicon requires before it will launch at all.
set -euo pipefail

binary="$1"
app="$2"
version="$3"
here="$(cd "$(dirname "$0")" && pwd)"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$binary" "$app/Contents/MacOS/tabletist"
chmod 755 "$app/Contents/MacOS/tabletist"
# The build number has to be numbers: a pre-release's -rc1 comes off.
build="${version%%-*}"
sed -e "s/__VERSION__/$version/g" -e "s/__BUILD__/$build/g" "$here/Info.plist" \
    > "$app/Contents/Info.plist"

# The .icns comes from the committed 1024 px PNG (rendered from icon.svg,
# which puts the artwork on Apple's icon grid): iconutil only runs on macOS.
iconset="$(mktemp -d)/tabletist.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    sips -z $size $size "$here/icon-1024.png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z $double $double "$here/icon-1024.png" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/tabletist.icns"

if [ -n "${CODESIGN_IDENTITY:-}" ]; then
    codesign --force --timestamp --options runtime --sign "$CODESIGN_IDENTITY" "$app"
else
    codesign --force --sign - "$app"
fi
codesign --verify --strict "$app"
echo "$app"
