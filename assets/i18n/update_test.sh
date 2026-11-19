#!/usr/bin/env bash
# Checks that update.sh extracts from every Rust file whatever its name, and
# skips one deleted but not yet staged: it runs the script in a throwaway
# repository whose files are named with a newline, a non-ASCII letter, and a
# deleted one last in the list. Needs what update.sh needs, GNU gettext 1.0
# or later. CI runs it in the translations job.
set -euo pipefail

script="$(cd "$(dirname "$0")" && pwd)/update.sh"
repo=$(mktemp -d)
trap 'rm -rf "$repo"' EXIT
cd "$repo"

git init -q
mkdir -p src assets/i18n
cp "$script" assets/i18n/update.sh
printf 'fn a(l: L) { gettext(l, "plain name"); }\n' > src/a.rs
printf 'fn b(l: L) { gettext(l, "newline name"); }\n' > $'src/b\nc.rs'
printf 'fn e(l: L) { gettext(l, "accented name"); }\n' > 'src/é.rs'
printf 'fn z(l: L) { gettext(l, "deleted name"); }\n' > src/zz.rs
git add .
# xgettext dates the template from the repository's history, so it needs one.
git -c commit.gpgsign=false -c user.name=test -c user.email=test@example.com \
    commit -q -m test
rm src/zz.rs

assets/i18n/update.sh

failed=0
for wanted in "plain name" "newline name" "accented name"; do
    if ! grep -qxF "msgid \"$wanted\"" assets/i18n/tabletist.pot; then
        echo "update_test: \"$wanted\" was not extracted" >&2
        failed=1
    fi
done
if grep -qF 'msgid "deleted name"' assets/i18n/tabletist.pot; then
    echo "update_test: a deleted file was extracted" >&2
    failed=1
fi
if (( failed == 0 )); then
    echo "update_test: ok"
fi
exit "$failed"
