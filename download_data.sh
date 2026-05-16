#!/usr/bin/env sh

CURRENT_DIR=$(pwd)
TMP_PATH="$(mktemp -d)"
trap 'rm -rf "$TMP_PATH"' EXIT

echo "Downloading data"
curl "https://pub-ce8744ae82a64ae7b60e364ba9c7ab52.r2.dev/data.zip" -o "${TMP_PATH}/data.zip"

cd "$TMP_PATH" || exit
echo "Unpacking archive"
unzip data.zip
mv tsp "$CURRENT_DIR/data"

cd "$CURRENT_DIR" || echo "Failed to switch back $CURRENT_DIR"
echo "Done!"

