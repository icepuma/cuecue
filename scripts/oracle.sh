#!/usr/bin/env bash
# The oracle: the official cue binary at a pinned version (docs/TESTING.md).
#
#   scripts/oracle.sh install            install cue into .oracle/bin
#   scripts/oracle.sh run <dir> [file…]  JSON export of the files (default: <dir>/*.cue),
#                                        or one "<kind> <path>" line per error, sorted, and exit 1
#   scripts/oracle.sh check              compare tests/oracle-fixtures with their `expected` files
set -euo pipefail

version=v0.17.1
root=$(cd "$(dirname "$0")/.." && pwd)
bin="$root/.oracle/bin/cue"

is_pinned() {
    [ -x "$bin" ] && [ "$("$bin" version | head -n 1)" = "cue version $version" ]
}

install() {
    is_pinned && return
    GOBIN="$root/.oracle/bin" go install "cuelang.org/go/cmd/cue@$version"
    is_pinned || {
        echo "oracle: $bin is not cue $version" >&2
        exit 1
    }
}

# Maps cue's error output to "<kind> <path>" lines. Kinds are defined in docs/SPEC.md §5;
# a message that matches no pattern becomes "other".
normalize() {
    awk '
        # Order matters: the first matching pattern wins.
        function kind(m) {
            if (m ~ /field not allowed/) return "closed"
            if (m ~ /field is required but not present|required field missing|missing required field/) return "required"
            if (m ~ /structural cycle|cyclic reference|circular dependency|field set was already referenced/) return "cycle"
            if (m ~ /incomplete|non-concrete|not concrete|non-ground|requires concrete value|unresolved disjunction|cannot reference optional field|invalid type _\)/) return "incomplete"
            if (m ~ /^(expected |missing |illegal |found packages |unreferenced alias or let|comprehension values not allowed|cannot use _ as )|not terminated/) return "syntax"
            if (m ~ /reference ".*" not found|undefined|out of range|must be non-negative|invalid slice index|is not available|import failed|cannot find package/) return "reference"
            if (m ~ /cannot call /) return "call"
            if (m ~ /error in call to|failed arithmetic|invalid regexp|exceeds limit/) return "builtin"
            if (m ~ /conflicting values|invalid value|invalid operand|invalid operation|invalid index|mismatched types|incompatible |empty disjunction|cannot use |cannot range over|cannot slice|cannot convert|want list or struct|not supported|explicit error/) return "conflict"
            return "other"
        }
        /^[ \t]/ { next }  # position lines under an error
        {
            line = $0
            sub(/:$/, "", line)
            path = "-"
            msg = line
            # A path is a run of labels (quoted labels may contain spaces) followed by ": ".
            if (match(line, /^([^ ":]|"([^"\\]|\\.)*")+: /)) {
                path = substr(line, 1, RLENGTH - 2)
                msg = substr(line, RLENGTH + 1)
            }
            print kind(msg), path
        }
    ' | LC_ALL=C sort -u
}

run() {
    install
    local dir=$1 tmp status=0
    shift
    tmp=$(mktemp -d)
    (
        cd "$dir"
        [ $# -gt 0 ] || set -- *.cue
        # A clean environment keeps CUE_* variables and the user's config out of the result.
        env -i HOME="$tmp" PATH=/usr/bin:/bin "$bin" export --out json "$@"
    ) >"$tmp/out" 2>"$tmp/err" || status=$?
    if [ "$status" -eq 0 ]; then
        cat "$tmp/out"
    else
        normalize <"$tmp/err"
    fi
    rm -rf "$tmp"
    [ "$status" -eq 0 ]
}

check() {
    local dir got failed=0
    for dir in "$root"/tests/oracle-fixtures/*/; do
        got=$(run "$dir" || true)
        if [ "$got" != "$(cat "$dir/expected")" ]; then
            printf 'oracle: %s: got\n%s\n' "$(basename "$dir")" "$got" >&2
            failed=1
        fi
    done
    return "$failed"
}

case "${1:-}" in
install) install ;;
run)
    shift
    run "$@"
    ;;
check) check ;;
*)
    echo "usage: scripts/oracle.sh install | run <dir> [file…] | check" >&2
    exit 2
    ;;
esac
