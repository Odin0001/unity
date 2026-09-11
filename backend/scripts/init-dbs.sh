#!/usr/bin/env bash
# Runs once, on first initialization of the postgres data volume, via
# /docker-entrypoint-initdb.d/. The official postgres image already creates
# the database named by POSTGRES_DB — this script creates the additional
# databases needed by the platform (control-plane + per-tenant template).
set -euo pipefail

MASTER_DB="${MASTER_DB:-erp_master_db}"
TENANT_TEMPLATE_DB="${TENANT_TEMPLATE_DB:-erp_tenant_template_db}"

create_database_if_missing() {
  local db_name="$1"

  local exists
  exists=$(psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname postgres --tuples-only --no-align \
    -c "SELECT 1 FROM pg_database WHERE datname = '${db_name}'")

  if [ "$exists" = "1" ]; then
    echo "[init-dbs] Database '${db_name}' already exists, skipping."
  else
    echo "[init-dbs] Creating database '${db_name}'..."
    psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname postgres \
      -c "CREATE DATABASE \"${db_name}\""
  fi
}

echo "[init-dbs] Initializing ERP platform databases..."

create_database_if_missing "$MASTER_DB"
create_database_if_missing "$TENANT_TEMPLATE_DB"

echo "[init-dbs] Done. Databases available: ${MASTER_DB}, ${TENANT_TEMPLATE_DB}"
