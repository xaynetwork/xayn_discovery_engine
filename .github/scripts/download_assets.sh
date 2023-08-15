#!/usr/bin/env -S bash -eu -o pipefail

realpath() {
    [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}

# path to the directory where this file is
SELF_DIR_PATH="$(dirname "$0")"

# the assets are always downloaded in the top lovel /assets directoy
DATA_DIR="$SELF_DIR_PATH/../../assets"
DATA_DIR=`realpath $DATA_DIR`

CHECKSUM_FILE="sha256sums"

download()
{
  NAME="$1"
  VERSION="$2"
  ARCHIVE_BASENAME="${NAME}_$VERSION"
  ARCHIVE_NAME="$ARCHIVE_BASENAME.tgz"
  TMP_ARCHIVE_NAME="$ARCHIVE_NAME.tmp"
  TMP_DIR=$(mktemp -d)
  URL="s3://xayn-yellow-bert/$NAME/$ARCHIVE_NAME"

  cd "$DATA_DIR"

  if [ -d "$DATA_DIR/$ARCHIVE_BASENAME" ]; then
    echo "skip downloading $DATA_DIR/$ARCHIVE_NAME"
  else

    aws s3 cp "$URL" "$DATA_DIR/$TMP_ARCHIVE_NAME" 
    mv "$DATA_DIR/$TMP_ARCHIVE_NAME" "$TMP_DIR/$ARCHIVE_NAME"

    cd "$TMP_DIR"
    tar -zxf "$ARCHIVE_NAME"

    # check content
    cd "$ARCHIVE_BASENAME"
    shasum -c "$CHECKSUM_FILE"

    mv "$TMP_DIR/$ARCHIVE_BASENAME" "$DATA_DIR/"
  fi
}

if [ $# -gt 0 ]; then
    while [ $# -ge 2 ]; do
        download $1 $2
        shift 2
    done
else
    download qasmbert v0002
    download smbert v0003
    download smbert v0004
    download smbert_mocked v0004
    download e5_mocked v0000
    download xaynia v0002
    download ort v0000
fi
