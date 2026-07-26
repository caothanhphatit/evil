#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_XAPK="$REPO_ROOT/game-assets/source/Evil+Hunter+Tycoon_1.411_APKPure.xapk"
DESTINATION="$REPO_ROOT/game-assets/source/unity-assets"
XAPK="$DEFAULT_XAPK"
SOURCE=""
EXPECTED_COUNT=415

usage() {
  cat <<'EOF'
Usage: tools/verify-asset-copy.sh [options]

Verify the immutable Unity asset copy against base_assets.apk in the repository XAPK.

Options:
  --xapk PATH          Use a different XAPK input.
  --source DIRECTORY   Compare against an already extracted assets directory instead.
  --destination DIR    Override the repository destination (primarily for tests).
  --expected-count N   Require exactly N source and destination files (default: 415).
  --help               Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --xapk)
      [[ $# -ge 2 ]] || { echo "Missing value for --xapk" >&2; exit 2; }
      XAPK="$2"
      shift 2
      ;;
    --source)
      [[ $# -ge 2 ]] || { echo "Missing value for --source" >&2; exit 2; }
      SOURCE="$2"
      shift 2
      ;;
    --destination)
      [[ $# -ge 2 ]] || { echo "Missing value for --destination" >&2; exit 2; }
      DESTINATION="$2"
      shift 2
      ;;
    --expected-count)
      [[ $# -ge 2 ]] || { echo "Missing value for --expected-count" >&2; exit 2; }
      EXPECTED_COUNT="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ "$EXPECTED_COUNT" =~ ^[1-9][0-9]*$ ]] || { echo "--expected-count must be a positive integer" >&2; exit 2; }
[[ -d "$DESTINATION" ]] || { echo "Destination asset directory not found: $DESTINATION" >&2; exit 2; }

work_dir=""
cleanup() {
  if [[ -n "$work_dir" && -d "$work_dir" ]]; then
    rm -rf -- "$work_dir"
  fi
}
trap cleanup EXIT

if [[ -z "$SOURCE" ]]; then
  [[ -f "$XAPK" ]] || { echo "XAPK not found: $XAPK" >&2; exit 2; }
  command -v unzip >/dev/null || { echo "unzip is required to verify the XAPK" >&2; exit 2; }
  work_dir="$(mktemp -d "${TMPDIR:-/tmp}/evil-asset-verify.XXXXXX")"
  unzip -qq "$XAPK" base_assets.apk -d "$work_dir"
  unzip -qq "$work_dir/base_assets.apk" 'assets/*' -d "$work_dir/base-assets"
  SOURCE="$work_dir/base-assets/assets"
fi

[[ -d "$SOURCE" ]] || { echo "Source asset directory not found: $SOURCE" >&2; exit 2; }

if command -v sha256sum >/dev/null; then
  hash_command=(sha256sum)
elif command -v shasum >/dev/null; then
  hash_command=(shasum -a 256)
else
  echo "sha256sum or shasum is required" >&2
  exit 2
fi

source_list="$(mktemp "${TMPDIR:-/tmp}/evil-source-assets.XXXXXX")"
destination_list="$(mktemp "${TMPDIR:-/tmp}/evil-destination-assets.XXXXXX")"
trap 'rm -f -- "$source_list" "$destination_list"; cleanup' EXIT

build_manifest() {
  local root="$1"
  local output="$2"
  (
    cd "$root"
    find . -type f -print0 | sort -z | xargs -0 "${hash_command[@]}"
  ) > "$output"
}

build_manifest "$SOURCE" "$source_list"
build_manifest "$DESTINATION" "$destination_list"

source_count="$(wc -l < "$source_list" | tr -d ' ')"
destination_count="$(wc -l < "$destination_list" | tr -d ' ')"
if [[ "$source_count" -ne "$EXPECTED_COUNT" || "$destination_count" -ne "$EXPECTED_COUNT" ]]; then
  echo "Asset count mismatch: expected=$EXPECTED_COUNT source=$source_count destination=$destination_count" >&2
  exit 1
fi

if ! diff -u "$source_list" "$destination_list"; then
  echo "Asset copy verification failed." >&2
  exit 1
fi

echo "Asset copy verified: $source_count files match byte-for-byte."
