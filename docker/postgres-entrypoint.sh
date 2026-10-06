#!/bin/sh
# Reject deployment defaults before PostgreSQL starts, including existing volumes.
set -eu
if [ "${#POSTGRES_PASSWORD}" -lt 16 ]; then
  echo 'Postgres requires a nondefault bootstrap password of at least 16 characters.' >&2
  exit 1
fi
case "$POSTGRES_PASSWORD" in
  postgres|cl8y_legal|changeme*|change-me*|replace-me*)
    echo 'Postgres bootstrap password is a known default or placeholder.' >&2
    exit 1
    ;;
esac
exec docker-entrypoint.sh "$@"
