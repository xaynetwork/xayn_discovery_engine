#!/bin/sh

set -e

cd $(dirname $0)

URL="${1:-http://localhost:9200/test_index}"

while ! curl -s -X OPTIONS "${URL}" ; do
    echo "Waiting for elasticsearch";
    sleep 1;
done;

curl -v -X PUT "${URL}?pretty" \
    -H 'Content-Type: application/json' \
    -d @mapping.json
