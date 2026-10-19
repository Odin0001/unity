// Each test binary uses a different subset of these helpers.
#![allow(dead_code)]

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tenants::{
    modules::{employees::application::create_executive_manager, tenancy::AdminClient},
    platform::AppState,
};
use tower::ServiceExt;

pub const EXECUTIVE_EMAIL: &str = "boss@acme.test";
pub const EXECUTIVE_PASSWORD: &str = "executive-password";
pub const EMPLOYEE_PASSWORD: &str = "employee-password";

/// The real router on a fresh database from `#[sqlx::test]`, with the
/// executive manager already created. Admin is never called here.
pub struct TestApp {
    router: Router,
    pub db: PgPool,
}

pub struct TestResponse {
    pub status: StatusCode,
    pub body: Value,
}

impl TestApp {
    pub async fn new(db: PgPool) -> Self {
        create_executive_manager(&db, EXECUTIVE_EMAIL, EXECUTIVE_PASSWORD.into(), "The Boss")
            .await
            .expect("create executive manager");
        let admin = AdminClient::new("http://127.0.0.1:9", "unused".into()).expect("client");
        Self {
            router: tenants::app(AppState {
                db: db.clone(),
                admin,
            }),
            db,
        }
    }

    pub async fn send(
        &self,
        method: &str,
        uri: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> TestResponse {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let request = match body {
            Some(json) => request
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json.to_string())),
            None => request.body(Body::empty()),
        }
        .expect("valid request");

        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("router is infallible");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("readable body");
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);

        TestResponse { status, body }
    }

    pub async fn login(&self, email: &str, password: &str) -> TestResponse {
        let credentials = json!({ "email": email, "password": password });
        self.send("POST", "/api/v1/auth/login", None, Some(credentials))
            .await
    }

    pub async fn token(&self, email: &str, password: &str) -> String {
        let response = self.login(email, password).await;
        assert_eq!(response.status, StatusCode::OK, "{}", response.body);
        response.body["token"]
            .as_str()
            .expect("token in login response")
            .to_string()
    }

    pub async fn executive_token(&self) -> String {
        self.token(EXECUTIVE_EMAIL, EXECUTIVE_PASSWORD).await
    }

    /// Has the executive create an employee and grant `permissions`, then
    /// signs that employee in. Returns `(id, token)`.
    pub async fn hire(&self, email: &str, permissions: Value) -> (String, String) {
        let boss = self.executive_token().await;
        let created = self
            .send(
                "POST",
                "/api/v1/employees",
                Some(&boss),
                Some(
                    json!({ "email": email, "full_name": "Agent", "password": EMPLOYEE_PASSWORD }),
                ),
            )
            .await;
        assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
        let id = created.body["id"]
            .as_str()
            .expect("employee id")
            .to_string();

        let granted = self
            .send(
                "PUT",
                &format!("/api/v1/employees/{id}/permissions"),
                Some(&boss),
                Some(permissions),
            )
            .await;
        assert_eq!(granted.status, StatusCode::OK, "{}", granted.body);

        let token = self.token(email, EMPLOYEE_PASSWORD).await;
        (id, token)
    }
}
