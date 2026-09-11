//! Platform: global infrastructure and glue code shared by every feature
//! module — the database pool, Redis-backed auth, and feature-flag
//! enforcement. Feature modules depend on `platform`; `platform` never
//! depends back on `modules`.

pub mod auth;
pub mod db;
pub mod feature_flags;

use axum::extract::FromRef;
use redis::aio::ConnectionManager;
use sqlx::PgPool;

/// Shared application state injected into every Axum handler via
/// `State<AppState>`. Individual fields are pulled out through
/// `axum::extract::FromRef`, which is what lets an extractor such as
/// `OpaqueAuth` depend on `ConnectionManager` alone instead of the whole
/// state struct.
#[derive(Clone, FromRef)]
pub struct AppState {
    pub db: PgPool,
    pub redis: ConnectionManager,
}
