#!/bin/sh
set -eu

if [ "${MIGRATION_VERIFY_DISPOSABLE:-}" != "true" ]; then
  echo "Refusing migration rollback check without MIGRATION_VERIFY_DISPOSABLE=true" >&2
  exit 1
fi
: "${DATABASE_URL:?DATABASE_URL is required}"

repo_root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
migration_dir="$repo_root/infra/db/migrations"
runner="$repo_root/infra/db/run-migrations.sh"
latest_up=$(find "$migration_dir" -maxdepth 1 -type f -name '[0-9][0-9][0-9][0-9]_*.sql' ! -name '*.down.sql' | sort | tail -n 1)
latest_version=$(basename "$latest_up" .sql)
latest_down="$migration_dir/$latest_version.down.sql"

if [ ! -f "$latest_down" ]; then
  echo "Latest migration has no rollback: $latest_version" >&2
  exit 1
fi

DATABASE_URL="$DATABASE_URL" sh "$runner"
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 --single-transaction -f "$latest_down"
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c "DELETE FROM schema_migration WHERE version = '$latest_version'"
DATABASE_URL="$DATABASE_URL" sh "$runner"
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc "SELECT version FROM schema_migration WHERE version = '$latest_version'" | grep -Fx "$latest_version" >/dev/null
echo "Migration round-trip passed: $latest_version"
