// `modules/` is intentionally empty right now (see src/modules/mod.rs), so
// every item in `platform` and `shared_kernel` below — including their
// public re-exports — is unused until the first feature module consumes
// it. Remove this once that module lands; it's silencing real signal, not
// a permanent policy.
#![allow(dead_code, unused_imports)]

mod modules;
mod platform;
mod shared_kernel;

use axum::Router;
use redis::aio::ConnectionManager;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use platform::{
    db::{self, DbConfig},
    AppState,
};

#[derive(Debug, thiserror::Error)]
enum StartupError {
    #[error(transparent)]
    Db(#[from] db::DbError),
    #[error("failed to connect to redis: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("failed to bind {addr}: {source}")]
    Bind {
        addr: String,
        #[source]
        source: std::io::Error,
    },
    #[error("server error: {0}")]
    Serve(#[source] std::io::Error),
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("fatal startup error: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), StartupError> {
    // Missing .env is fine in production, where config comes from the
    // environment directly (e.g. injected by the orchestrator).
    let _ = dotenvy::dotenv();
    init_tracing();

    let db_config = DbConfig::from_env();
    let pool = db::init_pool(&db_config).await?;
    db::run_migrations(&pool).await?;

    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://:erp_dev_password@127.0.0.1:6379".to_string());
    let redis_client = redis::Client::open(redis_url)?;
    let redis = ConnectionManager::new(redis_client).await?;

    let state = AppState { db: pool, redis };

    // Serves the compiled SPA from web/dist; any path that doesn't match a
    // real file falls back to index.html so client-side routing works.
    //
    // Deliberately `.fallback()`, not `.not_found_service()`: the latter
    // forces the response status to 404 (see tower_http::services::fs::
    // ServeDir::not_found_service docs) even though index.html is served
    // successfully — which reads as a broken page to health checks and to
    // client-side routers expecting a normal 200 navigation. `.fallback()`
    // serves the same content and preserves whatever status the fallback
    // service actually returns (200 for a served file).
    let static_assets = ServeDir::new("web/dist").fallback(ServeFile::new("web/dist/index.html"));

    // `modules/` is empty for now — nest each feature module's router here
    // as it's added, e.g. `.nest("/api/inventory", modules::inventory::
    // infrastructure::http::router())`.
    let app: Router = Router::new()
        .fallback_service(static_assets)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .map_err(|source| StartupError::Bind { addr: bind_addr.clone(), source })?;

    tracing::info!(%bind_addr, "tenants service listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(StartupError::Serve)?;

    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let _ = tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .try_init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(err) => tracing::error!(%err, "failed to install SIGTERM handler"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received, draining in-flight requests");
}
