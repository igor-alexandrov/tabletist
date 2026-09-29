#!/bin/bash
# Wrap Tabletist.app in a DMG with an Applications link, on a macOS machine.
#
#   packaging/macos/dmg.sh <Tabletist.app> <output.dmg>
#
# With CODESIGN_IDENTITY the DMG is signed; with APPLE_ID, APPLE_TEAM_ID and
# APPLE_APP_PASSWORD as well it is also notarized and stapled (Apple rejects
# an unsigned one). Without them it is left as is, and says so.
set -euo pipefail

app="$1"
dmg="$2"

stage="$(mktemp -d)"
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
rm -f "$dmg"
hdiutil create -volname Tabletist -srcfolder "$stage" -ov -format UDZO "$dmg"

if [ -n "${CODESIGN_IDENTITY:-}" ]; then
    codesign --force --timestamp --sign "$CODESIGN_IDENTITY" "$dmg"
fi
if [ -n "${CODESIGN_IDENTITY:-}" ] && [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ] \
    && [ -n "${APPLE_APP_PASSWORD:-}" ]; then
    xcrun notarytool submit "$dmg" --apple-id "$APPLE_ID" --team-id "$APPLE_TEAM_ID" \
        --password "$APPLE_APP_PASSWORD" --wait
    xcrun stapler staple "$dmg"
else
    echo "::notice::Not notarized: it takes a signing identity and the Apple notary secrets."
fi
echo "$dmg"
