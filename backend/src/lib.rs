//! Tenant service of the debt collection platform: one deployment per
//! tenant, linked to the platform admin by an API key.

pub mod modules;
pub mod platform;
pub mod shared_kernel;

use axum::{extract::State, http::StatusCode, routing::get, Router};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use modules::{employees, tenancy};
use platform::{error::AppError, AppState};

pub fn app(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .nest("/auth", employees::auth_router())
        .nest("/employees", employees::router())
        .merge(tenancy::router())
        .fallback(|| async { AppError::NotFound });

    // Paths outside the API that aren't real files get index.html, so
    // client-side routes survive a reload.
    let spa = ServeDir::new("web/dist").fallback(ServeFile::new("web/dist/index.html"));

    Router::new()
        .nest("/api/v1", api)
        .fallback_service(spa)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> StatusCode {
    match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => StatusCode::OK,
        Err(err) => {
            tracing::error!(%err, "health check failed");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}
