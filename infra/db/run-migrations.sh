#!/bin/sh
set -eu

psql "$DATABASE_URL" -v ON_ERROR_STOP=1 <<'SQL'
CREATE TABLE IF NOT EXISTS schema_migration (
    version TEXT PRIMARY KEY,
    checksum TEXT,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
ALTER TABLE schema_migration ADD COLUMN IF NOT EXISTS checksum TEXT;
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
    case "$version" in
        *[!A-Za-z0-9_-]*) echo "Invalid migration version: $version" >&2; exit 1 ;;
    esac
    checksum="$(sha256sum "$migration" | awk '{print $1}')"
    applied_checksum="$(psql "$DATABASE_URL" -Atc "SELECT COALESCE(checksum, '__missing__') FROM schema_migration WHERE version = '$version'")"
    if [ -n "$applied_checksum" ]; then
        if [ "$applied_checksum" = "__missing__" ]; then
            psql "$DATABASE_URL" -v ON_ERROR_STOP=1 \
                -c "UPDATE schema_migration SET checksum = '$checksum' WHERE version = '$version'"
        elif [ "$version:$applied_checksum:$checksum" = "0010_normalized_building_gameplay_content:53fe119c2f20d52a67d3a1856c535a06730908367a0da7965386adb50952e13c:218ffa02208a139ec0626d318cee668bc9b4788b2f459ac4d7c1ed5fca2cfccc" ]; then
            # The first production publication of 0010 predates the canonical
            # generated artifact retained in Git. Keep its recorded checksum
            # immutable while allowing later forward-only migrations.
            echo "Accepting recorded production checksum for $version"
        elif [ "$applied_checksum" != "$checksum" ]; then
            echo "Migration checksum mismatch for $version" >&2
            exit 1
        fi
        echo "Skipping already-applied migration $version"
        continue
    fi
    echo "Applying migration $version"
    psql "$DATABASE_URL" -v ON_ERROR_STOP=1 --single-transaction \
        -f "$migration" \
        -c "INSERT INTO schema_migration(version, checksum) VALUES ('$version', '$checksum')"
done
