#!/bin/bash

set -e

files_to_check() {
    find . \
         -path "./discovery_engine_core/ai/rubert-tokenizer" -prune \
         -o -type d -name target -prune \
         -o -type d -name .dart_tool -prune \
         -o \( -name "*.rs" -o -name "*.dart" \) \
         -print
}

errors=0

for file in $(files_to_check)
do
    if ! head -n 1 $file | grep 'Copyright 202[12] Xayn AG' > /dev/null
    then
        echo "##[error] Missing copyright in $file"
        errors=$((errors + 1))
    fi
done

exit $errors
