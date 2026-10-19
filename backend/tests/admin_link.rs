//! Admin lives in another repo, so these run against a stand-in that keeps
//! the same contract: a valid `X-Tenant-Key` on `/api/v1/internal/tenant`
//! gets the tenant back, anything else is a 401.

use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, Request, StatusCode},
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tenants::{
    modules::tenancy::{domain::TenantStatus, AdminClient},
    platform::AppState,
};
use tokio::net::TcpListener;
use tower::ServiceExt;

const API_KEY: &str = "test-tenant-key";
const TENANT_ID: &str = "bb42e72f-6773-46a9-a9ae-73591aa68e1c";

/// Starts the stand-in admin service and returns its origin.
async fn fake_admin() -> String {
    let router = Router::new().route(
        "/api/v1/internal/tenant",
        get(|headers: HeaderMap| async move {
            if headers
                .get("x-tenant-key")
                .is_some_and(|key| key == API_KEY)
            {
                Ok(Json(json!({
                    "id": TENANT_ID,
                    "slug": "acme",
                    "name": "Acme Collections",
                    "status": "active",
                    "is_master": false,
                    "last_seen_at": "2026-10-19T21:12:21Z",
                    "created_at": "2026-10-19T21:12:21Z",
                })))
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fake admin");
    let addr = listener.local_addr().expect("fake admin address");
    tokio::spawn(async move { axum::serve(listener, router).await });
    format!("http://{addr}")
}

/// An origin nothing is listening on.
async fn dead_admin() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("address");
    drop(listener);
    format!("http://{addr}")
}

async fn get_json(router: Router, uri: &str) -> (StatusCode, Value) {
    let request = Request::get(uri)
        .body(Body::empty())
        .expect("valid request");
    let response = router.oneshot(request).await.expect("router is infallible");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("readable body");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn client_fetches_its_profile_with_its_key() {
    // A trailing slash on the configured URL must not break the path.
    let base_url = format!("{}/", fake_admin().await);
    let admin = AdminClient::new(&base_url, API_KEY.into()).expect("client");

    let profile = admin.tenant_profile().await.expect("profile");
    assert_eq!(profile.id.to_string(), TENANT_ID);
    assert_eq!(profile.slug, "acme");
    assert_eq!(profile.status, TenantStatus::Active);
}

#[tokio::test]
async fn rejected_key_is_an_error() {
    let admin = AdminClient::new(&fake_admin().await, "wrong-key".into()).expect("client");
    let err = admin
        .tenant_profile()
        .await
        .expect_err("key must be rejected");
    assert!(err.to_string().contains("401"), "{err}");
}

#[sqlx::test]
async fn tenant_route_serves_the_profile_from_admin(db: PgPool) {
    let admin = AdminClient::new(&fake_admin().await, API_KEY.into()).expect("client");
    let app = tenants::app(AppState { db, admin });

    let (status, body) = get_json(app, "/api/v1/tenant").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["slug"], "acme");
    assert_eq!(body["status"], "active");
    assert!(
        body.get("is_master").is_none(),
        "only the profile is exposed"
    );
}

#[sqlx::test]
async fn tenant_route_is_a_502_while_admin_is_down(db: PgPool) {
    let admin = AdminClient::new(&dead_admin().await, API_KEY.into()).expect("client");
    let app = tenants::app(AppState { db, admin });

    let (status, body) = get_json(app.clone(), "/api/v1/tenant").await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(body["error"], "admin service unavailable");

    // Losing admin doesn't make this deployment unhealthy.
    let (health, _) = get_json(app, "/api/v1/health").await;
    assert_eq!(health, StatusCode::OK);
}
