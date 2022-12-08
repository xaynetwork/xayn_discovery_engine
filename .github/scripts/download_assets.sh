#!/bin/bash
set -eu -o pipefail

realpath() {
    [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}

# path to the directory where this file is
SELF_DIR_PATH="$(dirname "$0")"

# a parameter for the destination of the assets can be passed.
# the default is the directory assets, we assume the script is in .github/scripts/
DATA_DIR="${1:-$SELF_DIR_PATH/../../assets}"
DATA_DIR=`realpath $DATA_DIR`

CHECKSUM_FILE="sha256sums"

download()
{
  NAME="$1"
  VERSION="$2"
  ARCHIVE_BASENAME="${NAME}_$VERSION"
  ARCHIVE_NAME="$ARCHIVE_BASENAME.tgz"
  URL="http://s3-de-central.profitbricks.com/xayn-yellow-bert/$NAME/$ARCHIVE_NAME"

  if [  -f "$DATA_DIR/$ARCHIVE_NAME" ]; then
    echo "skip downloading $DATA_DIR/$ARCHIVE_NAME"
  else
    curl "$URL" -o "$DATA_DIR/$ARCHIVE_NAME" -C -

    cd "$DATA_DIR"
    tar -zxf "$ARCHIVE_NAME"

    # check content
    cd "$ARCHIVE_BASENAME"
    shasum -c "$CHECKSUM_FILE"
  fi
}

download smbert v0003
download smbert_mocked v0003
download sjbert v0003
