#!/usr/bin/env bash
set -euo pipefail

SOURCE="/Users/trana/Downloads/Evil_Hunter_Tycoon_1.411_decoded/xapk_splits/base_assets/assets"
DESTINATION="$(cd "$(dirname "$0")/.." && pwd)/game-assets/source/unity-assets"

if [[ ! -d "$SOURCE" ]]; then
  echo "Source extraction not found: $SOURCE" >&2
  exit 2
fi

source_list="$(mktemp)"
destination_list="$(mktemp)"
trap 'rm -f "$source_list" "$destination_list"' EXIT

(cd "$SOURCE" && find . -type f -print0 | sort -z | xargs -0 shasum -a 256) > "$source_list"
(cd "$DESTINATION" && find . -type f -print0 | sort -z | xargs -0 shasum -a 256) > "$destination_list"

if ! diff -u "$source_list" "$destination_list"; then
  echo "Asset copy verification failed." >&2
  exit 1
fi

echo "Asset copy verified: $(wc -l < "$source_list" | tr -d ' ') files match byte-for-byte."

