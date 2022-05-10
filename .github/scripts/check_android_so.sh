#!/usr/bin/env bash
set -euo pipefail

ERROR_COUNT=0

function error() {
    ((ERROR_COUNT++))
    echo "$@" 1>&2
}

function warn() {
    echo "$@" 1>&2
}

function check_so() {
    SO_FILE="$1"
    local OLD_ERROR_COUNT="$ERROR_COUNT"

    if [ ! -f "$SO_FILE" ]; then
        error "USAGE: $0 <SO_FILE_OR_DIR>"
        error "Shared object must be a file."
        return
    fi

    SCAN_RESULT="$(scanelf -qT "$SO_FILE")"

    if [ "$SCAN_RESULT" != "" ]; then
        error "RELOCATIONS in: '$SO_FILE'"
        warn "$SCAN_RESULT"
    fi

    if [ "$(readelf --dynamic "$SO_FILE" | grep SONAME)" = "" ]; then
        warn "NO SONAME in: '$SO_FILE'"
    fi

    if [ "$OLD_ERROR_COUNT" = "$ERROR_COUNT" ]; then
        echo "OK: $SO_FILE"
    fi
}

if [ -d "$1" ]; then
    CHECKED_SO_FILE=false
    for file in `find $1 -name '*.so' -type f`; do
        CHECKED_SO_FILE=true
        check_so "$file"
    done
    if [ "$CHECKED_SO_FILE" = "false" ]; then
        error "FOUND NO SO FILES IN: $1"
    fi
else
    check_so "$1"
fi

if ((ERROR_COUNT>0)); then
    exit 1
fi
