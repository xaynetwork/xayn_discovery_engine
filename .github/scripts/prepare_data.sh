#!/usr/bin/env -S bash -eux -o pipefail

# This script takes as input a directory, the name of the archive and a version.
# It creates an archive in the correct format in the current directory
# and adds the necessary information to verify its content.
# The archive will contain the directory name and the provided version.
# If the option --upload is provided the script will upload the archive to the s3 bucket.
#
# prepare_data.sh <model_dir> <model_name> <version_tag> [--upload]
# It creates an archive <model_name>_<version>.tgz which contains one directory <model_name>_<version>
# that contains the files that are present in <model_dir>.

# directory to prepare for upload
DIR_PATH="$1"
shift
NAME="$1"
shift
VERSION="$1"
shift

UPLOAD=false
while [ $# -gt 0 ]; do
  opt="$1"
  shift

  case "$opt" in
    --upload)
    UPLOAD=true
    ;;
  esac
done

DIR_PATH="$(pwd)/$DIR_PATH"
DIR_NAME="$(basename $DIR_PATH)"
ARCHIVE_BASENAME="${NAME}_$VERSION"
ARCHIVE_NAME="$ARCHIVE_BASENAME.tgz"
URL="s3://xayn-yellow-bert/$NAME/$ARCHIVE_NAME"
CHECKSUM_FILE="sha256sums"

CURRENT_DIR="$(pwd)"

# create a directory with the expected name
TMP_DIR="$(mktemp -d)"
cp -r "$DIR_PATH" "$TMP_DIR/$ARCHIVE_BASENAME"

# compute checksum file
cd "$TMP_DIR/$ARCHIVE_BASENAME"
rm -f "$CHECKSUM_FILE"
find . -type f -not -iname "$CHECKSUM_FILE" -not -name ".DS_Store" -print0 | xargs -0 shasum -a 256 > "$CHECKSUM_FILE"

cd "$CURRENT_DIR"

# prepare archive
tar czf "$ARCHIVE_NAME" --exclude ".DS_Store" -C "$TMP_DIR" "$ARCHIVE_BASENAME"
rm -rf "$TMP_DIR"

if [ "$UPLOAD" = true ]; then
  aws s3 cp "$ARCHIVE_NAME" "$URL"
fi

