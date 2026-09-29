#!/bin/bash
# Wrap Tabletist.app in a DMG with an Applications link, on a macOS machine.
#
#   packaging/macos/dmg.sh <Tabletist.app> <output.dmg>
#
# With CODESIGN_IDENTITY the DMG is signed; with NOTARY_PROFILE as well (a
# profile saved by `xcrun notarytool store-credentials`, in NOTARY_KEYCHAIN
# when that is set) it is also notarized and stapled (Apple rejects an
# unsigned one). The Apple password never appears on this script's command
# lines. Without them the DMG is left as is, and says so.
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
if [ -n "${CODESIGN_IDENTITY:-}" ] && [ -n "${NOTARY_PROFILE:-}" ]; then
    if [ -n "${NOTARY_KEYCHAIN:-}" ]; then
        xcrun notarytool submit "$dmg" --keychain-profile "$NOTARY_PROFILE" \
            --keychain "$NOTARY_KEYCHAIN" --wait
    else
        xcrun notarytool submit "$dmg" --keychain-profile "$NOTARY_PROFILE" --wait
    fi
    xcrun stapler staple "$dmg"
else
    echo "::notice::Not notarized: it takes a signing identity and the Apple notary secrets."
fi
echo "$dmg"
