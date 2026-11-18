#!/usr/bin/env bash
# Extracts Tabletist's translatable messages into assets/i18n/tabletist.pot,
# merges the template into every catalog (assets/i18n/*.po), and checks each
# catalog with msgfmt.
#
# Run from anywhere in the repository after changing a user-facing string.
# Needs GNU gettext 1.0 or later (Homebrew's gettext, Arch's): 0.24 was
# the first to read Rust, but the catalogs are checked with 1.0, which
# also reads a call by its path (`crate::i18n::gettext`). Ordinary Cargo
# builds need no gettext tools.
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
if (( major < 1 )); then
    echo "xgettext $version is too old; gettext 1.0 or later is needed" >&2
    exit 2
fi

scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

# Every Rust file but the ignored ones, a new one not yet added among them
# and one deleted but not yet staged left out. Names are read NUL-separated,
# so git leaves a non-ASCII one unquoted, and go to xgettext as arguments,
# so no name is split. (A loop, not mapfile: macOS's bash is 3.2.)
files=()
while IFS= read -r -d '' file; do
    if [[ -e $file ]]; then files+=("$file"); fi
done < <(git ls-files -z --cached --others --exclude-standard 'src/*.rs' 'crates/*.rs' \
    | LC_ALL=C sort -zu)
if (( ${#files[@]} == 0 )); then
    echo "no Rust files to extract messages from" >&2
    exit 1
fi
# `counted(locale, n, "row", "rows")` (crate::i18n) is a plural too.
xgettext --language=Rust --from-code=UTF-8 \
    --keyword= --keyword=gettext:2 --keyword=ngettext:2,3 --keyword=pgettext:2c,3 \
    --keyword=counted:3,4 \
    --add-comments=Translators: --add-location=file --sort-by-file \
    --flag=ngettext:2:rust-format --flag=ngettext:3:rust-format \
    --package-name=Tabletist --copyright-holder="Tabletist contributors" \
    --msgid-bugs-address="https://github.com/igor-alexandrov/tabletist/issues" \
    --output="$scratch/tabletist.pot" -- "${files[@]}" 2> "$scratch/warnings" \
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
