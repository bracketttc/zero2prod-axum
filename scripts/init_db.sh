#!/usr/bin/env bash

set -x
set -eo pipefail

if ! [ -x "$(command -v psql)" ]; then
  echo >&2 "Error: psql is not installed."
  exit 1
fi

if ! [ -x "$(command -v sqlx)" ]; then
  echo >&2 "Error: sqlx is not installed."
  echo >&2 "Use:"
  echo >&2 "    cargo isntall --version='~0.6' sqlx-cli \
  --no-default-features --features rustls,postgres"
  echo >&2 "to install it."
  exit 1
fi

source "$(dirname "$0")/../.devcontainer/.env"

until psql -h "${POSTGRES_HOSTNAME}" -U "${POSTGRES_USER}" -p "${POSTGRES_PORT}" -d "postgres" -c '\q' ; do
  >&2 echo "Postgres is still unavailable - sleeping"
  sleep 1
done

>&2 echo "Postgres is up and running on port ${POSTGRES_PORT} - running migrations now!"

echo "${DATABASE_URL}"

sqlx database create --database-url ${DATABASE_URL}
sqlx migrate run --database-url ${DATABASE_URL}

>&2 echo "Postgres has been migrated, ready to go!"
