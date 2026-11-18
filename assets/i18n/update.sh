#!/usr/bin/env bash
# Extracts Tabletist's translatable messages into assets/i18n/tabletist.pot,
# merges the template into every catalog (assets/i18n/*.po), and checks each
# catalog with msgfmt.
#
# Run from anywhere in the repository after changing a user-facing string.
# Needs GNU gettext 0.24 or later, which reads Rust (Homebrew's gettext,
# Arch's, Fedora 42's). Ordinary Cargo builds need no gettext tools.
#
#   assets/i18n/update.sh           update the template and the catalogs
#   assets/i18n/update.sh --check   change nothing; fail if either is stale
#
# References name the file and not the line, so moving code changes
# nothing here: only a message added, changed or taken out does.
set -euo pipefail

mode=update
case ${1:-} in
    '') ;;
    --check) mode=check ;;
    *) echo "usage: $0 [--check]" >&2; exit 2 ;;
esac

root=$(git rev-parse --show-toplevel)
cd "$root"
dir=assets/i18n
template=$dir/tabletist.pot

version=$(xgettext --version | sed -n '1s/.* //p')
major=${version%%.*}
minor=${version#*.}; minor=${minor%%.*}
if (( major == 0 && minor < 24 )); then
    echo "xgettext $version cannot read Rust; gettext 0.24 or later is needed" >&2
    exit 2
fi

scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

# Every Rust file but the ignored ones, a new one not yet added among them.
git ls-files --cached --others --exclude-standard 'src/*.rs' 'crates/*.rs' \
    | LC_ALL=C sort -u > "$scratch/files"
# `counted(locale, n, "row", "rows")` (crate::i18n) is a plural too.
xgettext --language=Rust --from-code=UTF-8 \
    --keyword= --keyword=gettext:2 --keyword=ngettext:2,3 --keyword=pgettext:2c,3 \
    --keyword=counted:3,4 \
    --add-comments=Translators: --add-location=file --sort-by-file \
    --flag=ngettext:2:rust-format --flag=ngettext:3:rust-format \
    --package-name=Tabletist --copyright-holder="Tabletist contributors" \
    --msgid-bugs-address="https://github.com/igor-alexandrov/tabletist/issues" \
    --files-from="$scratch/files" --output="$scratch/tabletist.pot" 2> "$scratch/warnings" \
    || { cat "$scratch/warnings" >&2; exit 1; }
# A message used both alone and as a plural, among others, is a warning
# xgettext does not fail on: here it fails.
if [[ -s $scratch/warnings ]]; then
    cat "$scratch/warnings" >&2
    exit 1
fi

# The extraction and merge dates are the only headers that change on
# every run.
undated() { sed -e '/^"POT-Creation-Date:/d' -e '/^"PO-Revision-Date:/d' "$1"; }

stale=0
if [[ $mode == check ]]; then
    if ! diff -u <(undated "$template") <(undated "$scratch/tabletist.pot"); then
        echo "$template is out of date: run $dir/update.sh" >&2
        stale=1
    fi
else
    cp "$scratch/tabletist.pot" "$template"
fi

for catalog in "$dir"/*.po; do
    [[ -e $catalog ]] || continue
    merged=$scratch/$(basename "$catalog")
    # A message the code no longer has is dropped, not kept as obsolete.
    msgmerge --quiet --no-fuzzy-matching --add-location=file --sort-by-file \
        --output-file="$merged.all" "$catalog" "$scratch/tabletist.pot"
    msgattrib --no-obsolete --add-location=file --sort-by-file \
        --output-file="$merged" "$merged.all"
    if [[ $mode == check ]]; then
        if ! diff -u <(undated "$catalog") <(undated "$merged"); then
            echo "$catalog is out of date: run $dir/update.sh" >&2
            stale=1
        fi
    else
        cp "$merged" "$catalog"
    fi
    msgfmt --check --statistics --output-file=/dev/null "$catalog"
done
exit "$stale"
