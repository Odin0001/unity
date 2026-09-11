use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("failed to establish database connection pool: {0}")]
    PoolInit(#[source] sqlx::Error),
    #[error("database migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub acquire_timeout: Duration,
}

impl DbConfig {
    pub fn from_env() -> Self {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://erp_admin:erp_dev_password@localhost:5432/erp_master_db".to_string()
        });
        let max_connections = std::env::var("DATABASE_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        Self {
            database_url,
            max_connections,
            acquire_timeout: Duration::from_secs(5),
        }
    }
}

/// Builds the process-wide Postgres connection pool.
///
/// This pool targets a single database (the control-plane `erp_master_db`
/// by default) and every repository scopes its queries with
/// `WHERE tenant_id = $1` for row-level tenancy. A database-per-tenant
/// deployment (cloning `erp_tenant_template_db` per signup instead) can be
/// layered on later by replacing this with a `TenantPoolRegistry` that
/// resolves a `PgPool` per `TenantId` — every repository trait in
/// `modules::*::domain` already takes `TenantId` as an explicit parameter
/// on every method, so that swap would not touch application or HTTP code.
pub async fn init_pool(config: &DbConfig) -> Result<PgPool, DbError> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(config.acquire_timeout)
        .connect(&config.database_url)
        .await
        .map_err(DbError::PoolInit)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
