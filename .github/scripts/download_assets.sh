#!/bin/bash
set -eu -o pipefail

DATA_DIR="$PWD/../../assets"
CHECKSUM_FILE="sha256sums"
BASE_URL="http://s3-de-central.profitbricks.com/xayn-yellow-bert"

download()
{
    ARCHIVE_BASENAME="$1_$2"
    ARCHIVE_NAME="$ARCHIVE_BASENAME.tgz"
    TMP_ARCHIVE_NAME="$ARCHIVE_NAME.tmp"

    if [ -f "$DATA_DIR/$ARCHIVE_NAME" ]; then
        echo "skip downloading $DATA_DIR/$ARCHIVE_NAME"
    else
        curl "$BASE_URL/$NAME/$ARCHIVE_NAME" -o "$DATA_DIR/$TMP_ARCHIVE_NAME" -C -
        mv "$DATA_DIR/$TMP_ARCHIVE_NAME" "$DATA_DIR/$ARCHIVE_NAME"

        cd "$DATA_DIR"
        tar -zxf "$ARCHIVE_NAME"

        # check content
        cd "$ARCHIVE_BASENAME"
        shasum -c "$CHECKSUM_FILE"
    fi
}

if [ $# -gt 0 ]; then
    while [ $# -ge 2 ]; do
        download $1 $2
        shift 2
    done
else
    download smbert v0003
    download smbert_mocked v0003
    download sjbert v0003
    download smroberta_tokenizer v0000
fi
