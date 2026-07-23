#!/bin/sh
set -eu

psql "$DATABASE_URL" -v ON_ERROR_STOP=1 <<'SQL'
CREATE TABLE IF NOT EXISTS schema_migration (
    version TEXT PRIMARY KEY,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
SQL

migration_dir="/migrations/migrations"
set -- "$migration_dir"/[0-9][0-9][0-9][0-9]_*.sql
if [ ! -e "$1" ]; then
    echo "No forward migrations found in $migration_dir" >&2
    exit 1
fi

for migration in "$@"; do
    case "$migration" in
        *.down.sql) continue ;;
    esac
    version="$(basename "$migration" .sql)"
    applied="$(psql "$DATABASE_URL" -Atc "SELECT 1 FROM schema_migration WHERE version = '$version'")"
    if [ "$applied" = "1" ]; then
        echo "Skipping already-applied migration $version"
        continue
    fi
    echo "Applying migration $version"
    psql "$DATABASE_URL" -v ON_ERROR_STOP=1 --single-transaction \
        -f "$migration" \
        -c "INSERT INTO schema_migration(version) VALUES ('$version')"
done
