#!/bin/sh

set -e

cd $(dirname $0)

URL="${1:-http://localhost:9200/test_index}"

# There is also `/_cluster/health` but using OPTION here works a bit better
# even through the index doesn't yet exist.
while ! curl -sf -X OPTIONS "${URL}" ; do
    echo "Waiting for elasticsearch";
    sleep 1;
done;

curl -if -X PUT "${URL}?pretty" \
    -H 'Content-Type: application/json' \
    -d @mapping.json
