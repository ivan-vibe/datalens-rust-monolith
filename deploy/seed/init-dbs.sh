#!/bin/sh
# First-start seed for the compose cluster: restore the DataLens databases from the
# custom-format dumps captured with `pg_dump -U pg-user -Fc --no-owner --no-privileges`.
set -e
for db in pg-compeng-db pg-demo-db pg-meta-manager-db pg-auth-db pg-us-db; do
  echo "seed: creating $db"
  if ! psql -U "$POSTGRES_USER" -lqt | cut -d"|" -f1 | grep -qw "$db"; then
    createdb -U "$POSTGRES_USER" "$db"
  fi
  echo "seed: restoring $db"
  pg_restore -U "$POSTGRES_USER" -d "$db" --no-owner --no-privileges "/seed/$db.dump" \
    || echo "seed: $db restored with warnings (already-present objects are expected)"
done
echo "seed: done"
