#!/bin/bash
# Decide how far the macOS release goes with the Apple secrets it has.
#
#   packaging/macos/signing-plan.sh
#
# Signing takes APPLE_CERTIFICATE_P12, APPLE_CERTIFICATE_PASSWORD and
# APPLE_SIGNING_IDENTITY; notarizing takes those and APPLE_ID, APPLE_TEAM_ID
# and APPLE_APP_PASSWORD. Writes sign=true or sign=false to $GITHUB_OUTPUT.
# A partial set warns, naming what is missing, and the release still ships:
# signed but not notarized, or ad-hoc signed.
set -euo pipefail

signing=(APPLE_CERTIFICATE_P12 APPLE_CERTIFICATE_PASSWORD APPLE_SIGNING_IDENTITY)
notary=(APPLE_ID APPLE_TEAM_ID APPLE_APP_PASSWORD)

# The names among "$@" whose variables are empty, comma separated.
missing() {
    local name list=""
    for name in "$@"; do
        if [ -z "${!name:-}" ]; then
            list="${list:+$list, }$name"
        fi
    done
    echo "$list"
}

missing_signing="$(missing "${signing[@]}")"
missing_notary="$(missing "${notary[@]}")"
missing_all="$(missing "${signing[@]}" "${notary[@]}")"
everything="$(IFS=,; echo "${signing[*]},${notary[*]}" | sed 's/,/, /g')"
output="${GITHUB_OUTPUT:-/dev/null}"

if [ -z "$missing_signing" ]; then
    echo "sign=true" >> "$output"
    if [ -n "$missing_notary" ]; then
        echo "::warning::Signed but not notarized: $missing_notary not set."
    fi
elif [ "$missing_all" = "$everything" ]; then
    echo "sign=false" >> "$output"
    echo "::notice::Apple signing secrets are not set; the app is ad-hoc signed."
else
    echo "sign=false" >> "$output"
    echo "::warning::Not signed or notarized, the app is ad-hoc signed: $missing_all not set."
fi
