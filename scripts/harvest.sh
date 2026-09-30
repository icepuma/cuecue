#!/usr/bin/env bash
# Rebuilds tests/corpus/v1 from CUE's own test archives plus the oracle's results
# (docs/TESTING.md). Rerunning it on the same CUE version changes nothing.
set -euo pipefail

version=v0.17.1
root=$(cd "$(dirname "$0")/.." && pwd)
src="$root/.oracle/src/cue"
dest="$root/tests/corpus/v1"

if [ ! -d "$src" ]; then
    git -c advice.detachedHead=false clone --quiet --depth 1 --branch "$version" \
        https://github.com/cue-lang/cue.git "$src"
fi
if [ "$(git -C "$src" describe --tags)" != "$version" ]; then
    echo "harvest: $src is not at $version" >&2
    exit 1
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/v1"
cp "$src/LICENSE" "$tmp/v1/LICENSE"

(cd "$src/cue/testdata" && find . -name '*.txtar' | LC_ALL=C sort) | while IFS= read -r rel; do
    rel=${rel#./}
    out="$tmp/v1/$rel"
    work="$tmp/work"
    rm -rf "$work" "$tmp/roots"
    mkdir -p "$work" "$(dirname "$out")"
    : >"$tmp/roots"

    # Copy the archive comment and its .cue sections to $out, extract those files into
    # $work, and list the root-level ones (the files CUE's own harness evaluates) in $tmp/roots.
    awk -v work="$work" -v out="$out" -v roots="$tmp/roots" '
        /^-- .* --$/ {
            if (file != "") close(file)
            name = substr($0, 4, length($0) - 6)
            keep = name ~ /\.cue$/
            file = ""
            if (keep) {
                print > out
                file = work "/" name
                dir = file
                sub(/\/[^\/]*$/, "", dir)
                system("mkdir -p \"" dir "\"")
                printf "" > file
                if (name !~ /\//) print name > roots
            }
            started = 1
            next
        }
        !started || keep { print > out }
        keep { print > file }
    ' "$src/cue/testdata/$rel"

    if grep -q '@experiment(' "$out"; then
        printf -- '-- oracle/skip --\nuses @experiment\n' >>"$out"
        continue
    fi

    files=()
    while IFS= read -r f; do files+=("$f"); done <"$tmp/roots"
    if result=$("$root/scripts/oracle.sh" run "$work" ${files[@]+"${files[@]}"}); then
        section=oracle/export.json
    else
        section=oracle/error
    fi
    printf -- '-- %s --\n%s\n' "$section" "$result" >>"$out"
done

rm -rf "$dest"
mkdir -p "$(dirname "$dest")"
mv "$tmp/v1" "$dest"
