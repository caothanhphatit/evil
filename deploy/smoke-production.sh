#!/bin/sh
set -eu

base_url="${EVIL_PRODUCTION_URL:-https://evil.poeviethoa.net}"
case "$base_url" in
    https://*) ;;
    *) echo "EVIL_PRODUCTION_URL must use https://" >&2; exit 1 ;;
esac

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT HUP INT TERM

curl --fail --silent --show-error "$base_url/healthz" > "$tmp_dir/healthz"
test "$(tr -d '\r\n' < "$tmp_dir/healthz")" = "ok"

curl --fail --silent --show-error "$base_url/ready" > "$tmp_dir/ready.json"
python3 - "$tmp_dir/ready.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    payload = json.load(handle)

dependencies = payload.get("dependencies", {})
if payload.get("status") != "ok":
    raise SystemExit("server readiness status is not ok")
if not dependencies.get("postgres_ready") or not dependencies.get("redis_ready"):
    raise SystemExit("server dependencies are not ready")
PY

curl --fail --silent --show-error "$base_url/" > "$tmp_dir/index.html"
if ! grep -q '<div id="app"></div>' "$tmp_dir/index.html"; then
    echo "production index does not contain the application mount" >&2
    exit 1
fi

echo "Production smoke passed: $base_url"
