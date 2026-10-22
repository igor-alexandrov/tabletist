#!/bin/bash
# Sign a release's checksums.txt for the app's updater, and check the
# signature against the public key the app has built in.
#
#   TABLETIST_UPDATE_SIGNING_KEY="$(cat key.pem)" \
#     packaging/sign-checksums.sh dist/checksums.txt assets/update-public-key.hex
#
# Writes dist/checksums.txt.sig: the raw 64-byte Ed25519 signature over the
# file's exact bytes, which fastframe-update checks before it downloads
# anything. Fails when the key is not set or is not the public key's pair:
# no updater would install a release signed with another key. The signature
# is written only once it verifies. Needs OpenSSL 3.

# The lines below expand the private key: never trace them, whatever the
# caller's shell options say.
set +x
set -euo pipefail

checksums="${1:?usage: sign-checksums.sh CHECKSUMS PUBLIC_KEY_HEX}"
public_hex="${2:?usage: sign-checksums.sh CHECKSUMS PUBLIC_KEY_HEX}"
# A signature from an earlier run never outlives a failed one.
rm -f "$checksums.sig"
: "${TABLETIST_UPDATE_SIGNING_KEY:?TABLETIST_UPDATE_SIGNING_KEY is not set}"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
# The key is never readable by anyone else, not even briefly.
(umask 077 && printf '%s\n' "$TABLETIST_UPDATE_SIGNING_KEY" > "$work/key.pem")
# Nothing below needs the variable; openssl must not inherit it.
unset TABLETIST_UPDATE_SIGNING_KEY
openssl pkeyutl -sign -inkey "$work/key.pem" -rawin -in "$checksums" -out "$work/signature"

# The built-in key must be exactly 64 hex digits: OpenSSL ignores trailing
# bytes in a key, so a longer file would otherwise pass.
hex="$(tr -d '[:space:]' < "$public_hex")"
if ! [[ "$hex" =~ ^[0-9a-fA-F]{64}$ ]]; then
    echo "$public_hex does not hold exactly 64 hex digits" >&2
    exit 1
fi
# The built-in key as DER: Ed25519's SubjectPublicKeyInfo prefix, then the
# 32 bytes the hex file holds.
printf '%b' "$(printf '%s' "302a300506032b6570032100${hex}" | sed 's/../\\x&/g')" > "$work/public.der"
openssl pkeyutl -verify -pubin -keyform DER -inkey "$work/public.der" -rawin \
    -in "$checksums" -sigfile "$work/signature"
[ "$(wc -c < "$work/signature")" -eq 64 ]
mv "$work/signature" "$checksums.sig"
