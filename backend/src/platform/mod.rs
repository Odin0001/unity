pub mod db;
pub mod error;
pub mod password;
pub mod token;

use sqlx::PgPool;

use crate::modules::tenancy::AdminClient;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub admin: AdminClient,
}
